
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

const RING_SECONDS: f32 = 1.0;

const DECODE_CHUNK: usize = 4096;

pub const MAX_LAYERS: usize = 32;


enum Command {
    Add(Box<ActiveLayer>),
    SetLayerFader { handle: u64, fader: f32 },
    Remove { handle: u64 },
    RemoveAll,
    SetMaster { fader: f32 },
    SetPlaying(bool),
}

struct ActiveLayer {
    handle: u64,
    samples: Consumer<f32>,
    gain: Smoothed,
    stop: Arc<AtomicBool>,
    retiring: bool,
    underruns: Arc<AtomicU32>,
}


struct Mixer {
    channels: usize,
    sample_rate: u32,
    layers: Vec<Box<ActiveLayer>>,
    commands: Consumer<Command>,
    graveyard: Producer<Box<ActiveLayer>>,
    master: Smoothed,
    transport: Smoothed,
    playing: bool,
    master_scratch: Vec<f32>,
}

impl Mixer {
    fn drain_commands(&mut self) {
        while let Ok(cmd) = self.commands.pop() {
            match cmd {
                Command::Add(mut layer) => {
                    if self.layers.len() < MAX_LAYERS {
                        layer.gain.silence();
                        self.layers.push(layer);
                    } else {
                        layer.stop.store(true, Ordering::Release);
                        let _ = self.graveyard.push(layer);
                    }
                }
                Command::SetLayerFader { handle, fader } => {
                    if let Some(l) = self.layers.iter_mut().find(|l| l.handle == handle) {
                        if !l.retiring {
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

    fn retire(&mut self) {
        let mut i = 0;
        while i < self.layers.len() {
            let done = self.layers[i].retiring && self.layers[i].gain.settled();
            if done {
                let layer = self.layers.swap_remove(i);
                layer.stop.store(true, Ordering::Release);
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

        let silent = !self.playing && self.transport.settled();
        out.fill(0.0);
        if silent || self.layers.is_empty() {
            self.retire();
            return;
        }

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

        for s in out.iter_mut() {
            if s.abs() > 0.7 {
                *s = s.signum() * (0.7 + 0.3 * ((s.abs() - 0.7) / 0.3).tanh());
            }
        }

        self.retire();
    }
}


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
    retiring: bool,
}

struct EngineInner {
    commands: Producer<Command>,
    graveyard: Consumer<Box<ActiveLayer>>,
    layers: Vec<Layer>,
    next_handle: u64,
}

pub struct AudioEngine {
    inner: Mutex<EngineInner>,
    _stream: cpal::Stream,
    pub device: DeviceInfo,
}

unsafe impl Send for AudioEngine {}
unsafe impl Sync for AudioEngine {}

pub fn output_devices() -> Vec<String> {
    let host = cpal::default_host();
    let default = host.default_output_device().and_then(|d| d.name().ok());
    let mut names: Vec<String> = match host.output_devices() {
        Ok(devices) => devices.filter_map(|d| d.name().ok()).collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names.dedup();
    if let Some(d) = default {
        names.retain(|n| n != &d);
        names.insert(0, d);
    }
    names
}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        Self::open(None)
    }

    pub fn open(wanted: Option<&str>) -> Result<Self> {
        let host = cpal::default_host();

        let device = wanted
            .and_then(|want| {
                host.output_devices().ok().and_then(|mut ds| {
                    ds.find(|d| d.name().map(|n| n == want).unwrap_or(false))
                })
            })
            .or_else(|| {
                if let Some(want) = wanted {
                    log::warn!("output device '{want}' is not available, using the default");
                }
                host.default_output_device()
            })
            .ok_or_else(|| anyhow!("no audio output device available"))?;

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
            master: Smoothed::new(0.0, tau::TRANSPORT, sample_rate),
            transport: Smoothed::new(1.0, tau::TRANSPORT, sample_rate),
            playing: true,
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

        if inner.layers.iter().any(|l| l.sound_id == sound_id && !l.retiring) {
            log::info!("'{sound_id}' is already playing, ignoring the request to add it again");
            return Ok(());
        }
        if inner.layers.iter().filter(|l| !l.retiring).count() >= MAX_LAYERS {
            return Err(anyhow!("already playing the maximum of {MAX_LAYERS} sounds"));
        }

        let rate = self.device.sample_rate;
        let channels = self.device.channels;

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
                            thread::sleep(Duration::from_millis(4));
                            continue;
                        }
                        let n = (slots.min(DECODE_CHUNK) / channels) * channels;
                        if let Err(e) = stream.read(&mut scratch[..n]) {
                            log::error!("decoder for {id} stopped: {e:#}");
                            break;
                        }
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
            retiring: false,
        });
        Ok(())
    }

    pub fn remove_layer(&self, sound_id: &str) -> Result<()> {
        let mut inner = self.inner.lock();
        Self::collect(&mut inner);
        let Some(i) = inner.layers.iter().position(|l| l.sound_id == sound_id && !l.retiring)
        else {
            return Ok(());
        };
        inner.layers[i].retiring = true;
        let handle = inner.layers[i].handle;
        Self::send(&mut inner, Command::Remove { handle })
    }

    pub fn set_layer_fader(&self, sound_id: &str, fader: f32) -> Result<()> {
        let mut inner = self.inner.lock();
        let Some(handle) = inner
            .layers
            .iter()
            .find(|l| l.sound_id == sound_id && !l.retiring)
            .map(|l| l.handle)
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
        for l in inner.layers.iter_mut() {
            l.retiring = true;
        }
        Self::send(&mut inner, Command::RemoveAll)
    }

    pub fn active_sounds(&self) -> Vec<String> {
        let mut inner = self.inner.lock();
        Self::collect(&mut inner);
        inner
            .layers
            .iter()
            .filter(|l| !l.retiring)
            .map(|l| l.sound_id.clone())
            .collect()
    }

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
