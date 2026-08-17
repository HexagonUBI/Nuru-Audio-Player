//! The mixer and its output stream.
//!
//! Threading, because it is the part that is easy to get wrong:
//!
//!   control thread   Tauri commands land here. Opens files, spawns decoders,
//!                    and posts messages. Allowed to allocate and block.
//!   decoder thread   One per layer. Decodes, resamples and loops into a
//!                    lock-free ring. Allowed to allocate and block.
//!   audio callback   Owned by the OS. Drains rings, applies gain, sums. May not
//!                    allocate, lock, or block — doing any of those is what
//!                    makes an audio app crackle under load.
//!
//! Everything crossing into the callback goes through an SPSC ring: samples one
//! way, commands the other. Layers that die are posted *back* to the control
//! thread on a third ring so the callback never runs a destructor.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use parking_lot::Mutex;
use rtrb::{Consumer, Producer, RingBuffer};

use super::gain::{amplitude, tau, Smoothed};
use super::source::Stream;

/// How much decoded audio sits ahead of the callback, in seconds. Enough to ride
/// out a disk hiccup or a scheduler stall; small enough that switching a sound
/// off frees its memory promptly.
const RING_SECONDS: f32 = 1.0;

/// Samples the decoder hands over per iteration.
const DECODE_CHUNK: usize = 4096;

/// Ceiling on simultaneous layers. The callback's layer vector is allocated to
/// this size up front and never grows, because growing it would allocate.
pub const MAX_LAYERS: usize = 32;

// ─── Messages ─────────────────────────────────────────────────────────────────

enum Command {
    Add(Box<ActiveLayer>),
    SetLayerFader { handle: u64, fader: f32 },
    Remove { handle: u64 },
    RemoveAll,
    SetMaster { fader: f32 },
    SetPlaying(bool),
}

/// A layer as the audio callback sees it.
struct ActiveLayer {
    handle: u64,
    samples: Consumer<f32>,
    gain: Smoothed,
    /// Tells the decoder thread to wind up.
    stop: Arc<AtomicBool>,
    /// Set once the layer is fading out; it is dropped when the fade completes.
    retiring: bool,
    /// Incremented when the callback wanted samples and the ring was short.
    /// Surfaced in diagnostics rather than logged, because logging from the
    /// audio thread is itself a way to cause underruns.
    underruns: Arc<AtomicU32>,
}

// ─── Callback state ───────────────────────────────────────────────────────────

struct Mixer {
    channels: usize,
    sample_rate: u32,
    layers: Vec<Box<ActiveLayer>>,
    commands: Consumer<Command>,
    graveyard: Producer<Box<ActiveLayer>>,
    /// The user's master volume.
    master: Smoothed,
    /// Play/pause, kept separate from `master` so pausing can reach true zero
    /// without forgetting what the volume was. Once it settles at zero the
    /// callback stops draining the rings entirely, which parks the decoder
    /// threads and resumes exactly where playback stopped.
    transport: Smoothed,
    playing: bool,
    /// Per-frame master × transport gain for the current callback. Pre-allocated.
    master_scratch: Vec<f32>,
}

impl Mixer {
    fn drain_commands(&mut self) {
        while let Ok(cmd) = self.commands.pop() {
            match cmd {
                Command::Add(mut layer) => {
                    if self.layers.len() < MAX_LAYERS {
                        // Start silent and ramp up, so switching a sound on is a
                        // fade rather than a click.
                        layer.gain.reset(0.0);
                        self.layers.push(layer);
                    } else {
                        layer.stop.store(true, Ordering::Release);
                        let _ = self.graveyard.push(layer);
                    }
                }
                Command::SetLayerFader { handle, fader } => {
                    if let Some(l) = self.layers.iter_mut().find(|l| l.handle == handle) {
                        if !l.retiring {
                            // A drag wants a short time constant so the sound
                            // tracks the handle; the long one is only for the
                            // fades on either side of a layer's life.
                            l.gain.set_tau(tau::FADER, self.sample_rate);
                            l.gain.set_target(amplitude(fader));
                        }
                    }
                }
                Command::Remove { handle } => {
                    if let Some(l) = self.layers.iter_mut().find(|l| l.handle == handle) {
                        l.retiring = true;
                        l.gain.set_tau(tau::LAYER_FADE, self.sample_rate);
                        l.gain.set_target(0.0);
                    }
                }
                Command::RemoveAll => {
                    for l in self.layers.iter_mut() {
                        l.retiring = true;
                        l.gain.set_tau(tau::LAYER_FADE, self.sample_rate);
                        l.gain.set_target(0.0);
                    }
                }
                Command::SetMaster { fader } => self.master.set_target(amplitude(fader)),
                Command::SetPlaying(p) => {
                    self.playing = p;
                    self.transport.set_target(if p { 1.0 } else { 0.0 });
                }
            }
        }
    }

    /// Hand finished layers back to the control thread. Nothing is dropped here.
    fn retire(&mut self) {
        let mut i = 0;
        while i < self.layers.len() {
            let done = self.layers[i].retiring && self.layers[i].gain.settled();
            if done {
                let layer = self.layers.swap_remove(i);
                layer.stop.store(true, Ordering::Release);
                // If the graveyard is full the Box is dropped here, which
                // allocates. That only happens if the control thread has stopped
                // collecting entirely, at which point a hitch is the least of it.
                let _ = self.graveyard.push(layer);
            } else {
                i += 1;
            }
        }
    }

    fn render(&mut self, out: &mut [f32]) {
        let ch = self.channels;
        let frames = out.len() / ch;

        self.drain_commands();

        // Paused and fully faded: emit silence without touching the rings, so
        // the decoders idle and playback resumes exactly where it stopped.
        let silent = !self.playing && self.transport.settled();
        out.fill(0.0);
        if silent || self.layers.is_empty() {
            self.retire();
            return;
        }

        // Master ramp is per-frame and shared by every layer, so compute it once.
        let scratch = &mut self.master_scratch[..frames];
        for s in scratch.iter_mut() {
            *s = self.master.next() * self.transport.next();
        }

        for layer in self.layers.iter_mut() {
            let want = frames * ch;
            let have = layer.samples.slots();
            if have < want {
                layer.underruns.fetch_add(1, Ordering::Relaxed);
            }
            // Whole frames only — a partial frame would swap the channels for
            // everything that follows.
            let take = (have.min(want) / ch) * ch;
            if take == 0 {
                continue;
            }

            let Ok(chunk) = layer.samples.read_chunk(take) else {
                continue;
            };
            let (a, b) = chunk.as_slices();

            let mut frame = 0usize;
            let mut lane = 0usize;
            let mut g = layer.gain.current();

            for slice in [a, b] {
                for &sample in slice {
                    if lane == 0 {
                        g = layer.gain.next();
                    }
                    out[frame * ch + lane] += sample * g * scratch[frame];
                    lane += 1;
                    if lane == ch {
                        lane = 0;
                        frame += 1;
                    }
                }
            }
            chunk.commit_all();
        }

        // Sums of many beds can exceed full scale. A hard clip there is the
        // ugliest sound in the app, so fold the peaks with tanh, which is
        // transparent below about -3 dBFS and soft above it.
        for s in out.iter_mut() {
            if s.abs() > 0.7 {
                *s = s.signum() * (0.7 + 0.3 * ((s.abs() - 0.7) / 0.3).tanh());
            }
        }

        self.retire();
    }
}

// ─── Engine ───────────────────────────────────────────────────────────────────

pub struct DeviceInfo {
    pub name: String,
    pub sample_rate: u32,
    pub channels: usize,
}

struct Layer {
    handle: u64,
    sound_id: String,
    stop: Arc<AtomicBool>,
    underruns: Arc<AtomicU32>,
    decoder: Option<JoinHandle<()>>,
}

struct EngineInner {
    commands: Producer<Command>,
    graveyard: Consumer<Box<ActiveLayer>>,
    layers: Vec<Layer>,
    next_handle: u64,
}

pub struct AudioEngine {
    inner: Mutex<EngineInner>,
    /// Held so the stream stays alive; cpal stops output when it is dropped.
    _stream: cpal::Stream,
    pub device: DeviceInfo,
}

// cpal::Stream is not Send on every backend, but the engine only ever lives
// inside Tauri's managed state on the main thread, and every field that other
// threads touch is behind the mutex or an atomic.
unsafe impl Send for AudioEngine {}
unsafe impl Sync for AudioEngine {}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("no default audio output device"))?;
        let name = device.name().unwrap_or_else(|_| "unknown".into());
        let supported = device
            .default_output_config()
            .context("querying the default output config")?;

        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let channels = config.channels as usize;
        let sample_rate = config.sample_rate.0;

        let (cmd_tx, cmd_rx) = RingBuffer::<Command>::new(256);
        let (grave_tx, grave_rx) = RingBuffer::<Box<ActiveLayer>>::new(MAX_LAYERS * 2);

        let mut mixer = Mixer {
            channels,
            sample_rate,
            layers: Vec::with_capacity(MAX_LAYERS),
            commands: cmd_rx,
            graveyard: grave_tx,
            // Start muted and ramp to whatever the UI sets, so launching Nuru is
            // never a jolt.
            master: Smoothed::new(0.0, tau::TRANSPORT, sample_rate),
            transport: Smoothed::new(1.0, tau::TRANSPORT, sample_rate),
            playing: true,
            // 8192 frames is far above any sane buffer size; sized once so the
            // callback never has to grow it.
            master_scratch: vec![0.0; 8192],
        };

        let err_fn = |e| log::error!("audio output stream error: {e}");

        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &config,
                move |out: &mut [f32], _| mixer.render(out),
                err_fn,
                None,
            ),
            SampleFormat::I16 => {
                let mut scratch = vec![0.0f32; 8192 * channels];
                device.build_output_stream(
                    &config,
                    move |out: &mut [i16], _| {
                        let n = out.len().min(scratch.len());
                        mixer.render(&mut scratch[..n]);
                        for (d, s) in out.iter_mut().zip(&scratch[..n]) {
                            *d = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => return Err(anyhow!("unsupported output sample format {other:?}")),
        }
        .context("building the output stream")?;

        stream.play().context("starting the output stream")?;

        Ok(Self {
            inner: Mutex::new(EngineInner {
                commands: cmd_tx,
                graveyard: grave_rx,
                layers: Vec::new(),
                next_handle: 1,
            }),
            _stream: stream,
            device: DeviceInfo { name, sample_rate, channels },
        })
    }

    /// Collect layers the callback has finished with and join their decoders.
    fn collect(inner: &mut EngineInner) {
        while let Ok(dead) = inner.graveyard.pop() {
            if let Some(i) = inner.layers.iter().position(|l| l.handle == dead.handle) {
                let mut layer = inner.layers.swap_remove(i);
                layer.stop.store(true, Ordering::Release);
                if let Some(h) = layer.decoder.take() {
                    let _ = h.join();
                }
            }
            drop(dead);
        }
    }

    fn send(inner: &mut EngineInner, cmd: Command) -> Result<()> {
        inner
            .commands
            .push(cmd)
            .map_err(|_| anyhow!("audio command queue is full"))
    }

    /// Start playing a local file as a new layer.
    ///
    /// `path` must already exist on disk. Nuru never streams from the network:
    /// a pack is downloaded and verified in full before any of it reaches here,
    /// so playback can never stutter because a connection did.
    pub fn add_layer(
        &self,
        sound_id: &str,
        path: PathBuf,
        loop_start: u64,
        loop_end: Option<u64>,
        crossfade_ms: u32,
        fader: f32,
    ) -> Result<()> {
        if !path.is_file() {
            return Err(anyhow!("{} is not on disk", path.display()));
        }

        let mut inner = self.inner.lock();
        Self::collect(&mut inner);

        if inner.layers.iter().any(|l| l.sound_id == sound_id) {
            return Ok(());
        }
        if inner.layers.len() >= MAX_LAYERS {
            return Err(anyhow!("already playing the maximum of {MAX_LAYERS} sounds"));
        }

        let rate = self.device.sample_rate;
        let channels = self.device.channels;

        // Crossfade length is specified in milliseconds against the *source*
        // rate, which the stream resolves once it has opened the file.
        let mut stream = Stream::open(
            &path,
            loop_start,
            loop_end,
            (crossfade_ms as u64 * rate as u64) / 1000,
            rate,
            channels,
        )?;

        let ring_len = ((rate as f32 * RING_SECONDS) as usize * channels).next_power_of_two();
        let (mut producer, consumer) = RingBuffer::<f32>::new(ring_len);

        let stop = Arc::new(AtomicBool::new(false));
        let underruns = Arc::new(AtomicU32::new(0));
        let handle = inner.next_handle;
        inner.next_handle += 1;

        let decoder = {
            let stop = stop.clone();
            let id = sound_id.to_string();
            thread::Builder::new()
                .name(format!("nuru-decode-{id}"))
                .spawn(move || {
                    let mut scratch = vec![0.0f32; DECODE_CHUNK];
                    while !stop.load(Ordering::Acquire) {
                        let slots = producer.slots();
                        if slots < channels {
                            // The ring is full, which is the healthy state. Sleep
                            // for a fraction of the buffer rather than spinning.
                            thread::sleep(Duration::from_millis(4));
                            continue;
                        }
                        let n = (slots.min(DECODE_CHUNK) / channels) * channels;
                        if let Err(e) = stream.read(&mut scratch[..n]) {
                            log::error!("decoder for {id} stopped: {e:#}");
                            break;
                        }
                        // The ring wraps, so the writable region can be two
                        // slices; copy across the split rather than assuming one.
                        if let Ok(mut chunk) = producer.write_chunk(n) {
                            let (a, b) = chunk.as_mut_slices();
                            a.copy_from_slice(&scratch[..a.len()]);
                            b.copy_from_slice(&scratch[a.len()..a.len() + b.len()]);
                            chunk.commit_all();
                        }
                    }
                })
                .context("spawning the decoder thread")?
        };

        let mut gain = Smoothed::new(0.0, tau::LAYER_FADE, rate);
        gain.set_target(amplitude(fader));

        let active = Box::new(ActiveLayer {
            handle,
            samples: consumer,
            gain,
            stop: stop.clone(),
            retiring: false,
            underruns: underruns.clone(),
        });

        Self::send(&mut inner, Command::Add(active))?;
        inner.layers.push(Layer {
            handle,
            sound_id: sound_id.to_string(),
            stop,
            underruns,
            decoder: Some(decoder),
        });
        Ok(())
    }

    pub fn remove_layer(&self, sound_id: &str) -> Result<()> {
        let mut inner = self.inner.lock();
        Self::collect(&mut inner);
        let Some(handle) = inner.layers.iter().find(|l| l.sound_id == sound_id).map(|l| l.handle)
        else {
            return Ok(());
        };
        Self::send(&mut inner, Command::Remove { handle })
    }

    pub fn set_layer_fader(&self, sound_id: &str, fader: f32) -> Result<()> {
        let mut inner = self.inner.lock();
        let Some(handle) = inner.layers.iter().find(|l| l.sound_id == sound_id).map(|l| l.handle)
        else {
            return Ok(());
        };
        Self::send(&mut inner, Command::SetLayerFader { handle, fader })
    }

    pub fn set_master(&self, fader: f32) -> Result<()> {
        let mut inner = self.inner.lock();
        Self::send(&mut inner, Command::SetMaster { fader })
    }

    pub fn set_playing(&self, playing: bool) -> Result<()> {
        let mut inner = self.inner.lock();
        Self::send(&mut inner, Command::SetPlaying(playing))
    }

    pub fn clear(&self) -> Result<()> {
        let mut inner = self.inner.lock();
        Self::collect(&mut inner);
        Self::send(&mut inner, Command::RemoveAll)
    }

    pub fn active_sounds(&self) -> Vec<String> {
        let mut inner = self.inner.lock();
        Self::collect(&mut inner);
        inner.layers.iter().map(|l| l.sound_id.clone()).collect()
    }

    /// Total ring underruns since start. Non-zero means the decoders are not
    /// keeping up — useful when someone reports crackling.
    pub fn underruns(&self) -> u32 {
        let inner = self.inner.lock();
        inner.layers.iter().map(|l| l.underruns.load(Ordering::Relaxed)).sum()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        let mut inner = self.inner.lock();
        for layer in inner.layers.iter_mut() {
            layer.stop.store(true, Ordering::Release);
            if let Some(h) = layer.decoder.take() {
                let _ = h.join();
            }
        }
    }
}
