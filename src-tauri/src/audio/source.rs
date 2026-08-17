//! Reading a file off disk and turning it into an endless, seamless stream at
//! the output device's sample rate.
//!
//! Three layers, each doing one job:
//!
//!   FrameReader    decode → interleaved f32 at the file's own rate
//!   LoopingSource  apply the loop, never runs out
//!   Stream         channel-map and resample to what the device wants
//!
//! The loop is the whole point of this module, so it is worth being explicit
//! about why it is built this way. Elpy — the app Nuru is a remake of — looped by
//! watching `timeupdate` on an `<audio>` element and setting `currentTime` back
//! to 0.5 s when it got near the end. That fails three ways at once: the event
//! fires roughly every 250 ms so the wrap point is wherever it happens to land,
//! `currentTime` on a compressed stream seeks to a codec frame rather than a
//! sample, and AAC's encoder delay means the decoded output does not begin where
//! the file says it does. It only passes unnoticed because rain masks it.
//!
//! Here the wrap happens in the producer, at an exact sample index, on a codec
//! that decodes to exactly the samples that went in.

use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::gain::equal_power;

// ─── FrameReader ──────────────────────────────────────────────────────────────

/// Decodes a local file to interleaved `f32`, and can seek to an exact frame.
pub struct FrameReader {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_buf: Option<SampleBuffer<f32>>,

    /// Decoded samples from the last packet that the caller has not taken yet.
    pending: Vec<f32>,
    pending_cursor: usize,

    /// Index of the next frame `read` will hand out.
    next_frame: u64,

    pub rate: u32,
    pub channels: usize,
    pub frames: u64,
}

impl FrameReader {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .with_context(|| format!("probing {}", path.display()))?;
        let reader = probed.format;

        let track = reader
            .default_track()
            .ok_or_else(|| anyhow!("{} has no default audio track", path.display()))?;
        let track_id = track.id;
        let params = track.codec_params.clone();

        let rate = params.sample_rate.ok_or_else(|| anyhow!("unknown sample rate"))?;
        let channels = params
            .channels
            .ok_or_else(|| anyhow!("unknown channel layout"))?
            .count();
        let frames = params.n_frames.unwrap_or(0);

        let decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions { verify: false })
            .with_context(|| format!("no decoder for {}", path.display()))?;

        Ok(Self {
            reader,
            decoder,
            track_id,
            sample_buf: None,
            pending: Vec::new(),
            pending_cursor: 0,
            next_frame: 0,
            rate,
            channels,
            frames,
        })
    }

    /// Decode one packet from the track into `pending`. Returns false at EOF.
    fn decode_next(&mut self) -> Result<bool> {
        loop {
            let packet = match self.reader.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(false)
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(e) => return Err(e.into()),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let capacity = decoded.capacity() as u64;
                    let buf = self
                        .sample_buf
                        .get_or_insert_with(|| SampleBuffer::<f32>::new(capacity, spec));
                    // A stream whose spec changes mid-file would need a new
                    // buffer; Nuru only plays files it transcoded itself, so
                    // treat it as a hard error rather than silently mangling it.
                    if buf.capacity() < decoded.frames() * spec.channels.count() {
                        *buf = SampleBuffer::<f32>::new(capacity, spec);
                    }
                    buf.copy_interleaved_ref(decoded);
                    self.pending.clear();
                    self.pending.extend_from_slice(buf.samples());
                    self.pending_cursor = 0;
                    return Ok(true);
                }
                // A single corrupt packet should drop out of the bed, not kill
                // the stream.
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Fill `out` with interleaved frames. Returns frames written, which is less
    /// than requested only at end of file.
    pub fn read(&mut self, out: &mut [f32]) -> Result<usize> {
        let ch = self.channels;
        let want = out.len() / ch;
        let mut done = 0;

        while done < want {
            if self.pending_cursor >= self.pending.len() && !self.decode_next()? {
                break;
            }
            let available = (self.pending.len() - self.pending_cursor) / ch;
            if available == 0 {
                self.pending_cursor = self.pending.len();
                continue;
            }
            let n = available.min(want - done);
            let src = self.pending_cursor;
            out[done * ch..(done + n) * ch].copy_from_slice(&self.pending[src..src + n * ch]);
            self.pending_cursor += n * ch;
            done += n;
        }

        self.next_frame += done as u64;
        Ok(done)
    }

    /// Seek so the next `read` returns frame `frame`.
    ///
    /// Containers are allowed to land earlier than asked, so whatever the
    /// demuxer actually gave us gets discarded up to the requested frame. That
    /// discard is what makes the seek sample-exact rather than packet-exact.
    pub fn seek_to_frame(&mut self, frame: u64) -> Result<()> {
        let seeked = self.reader.seek(
            SeekMode::Accurate,
            SeekTo::TimeStamp { ts: frame, track_id: self.track_id },
        )?;
        self.decoder.reset();
        self.pending.clear();
        self.pending_cursor = 0;
        self.next_frame = seeked.actual_ts;

        if seeked.actual_ts < frame {
            let mut skip = (frame - seeked.actual_ts) as usize;
            let mut scratch = vec![0.0f32; 4096 * self.channels];
            while skip > 0 {
                let n = skip.min(4096);
                let got = self.read(&mut scratch[..n * self.channels])?;
                if got == 0 {
                    break;
                }
                skip -= got;
            }
        }
        Ok(())
    }
}

// ─── LoopingSource ────────────────────────────────────────────────────────────

/// How a source wraps back to its loop start.
#[derive(Debug, Clone, Copy)]
pub enum LoopMode {
    /// The material was authored to loop: the last sample joins the first with
    /// no discontinuity. Nothing to blend, so nothing is blended.
    Exact,
    /// The material was not authored to loop — a field recording of rain has no
    /// reason for its end to match its beginning. The tail is crossfaded into a
    /// copy of the head over `frames`, which makes the join inaudible at the
    /// cost of shortening the loop by that much.
    Crossfade { frames: u64 },
}

/// An endless stream of interleaved frames at the file's own sample rate.
pub struct LoopingSource {
    reader: FrameReader,
    loop_start: u64,
    loop_end: u64,
    mode: LoopMode,
    /// A decoded copy of the first `frames` of the loop, mixed into the tail.
    head: Vec<f32>,
    /// Playback position on the source timeline.
    pos: u64,
}

impl LoopingSource {
    pub fn open(path: &Path, loop_start: u64, loop_end: Option<u64>, crossfade_frames: u64) -> Result<Self> {
        let mut reader = FrameReader::open(path)?;
        let total = if reader.frames > 0 { reader.frames } else { u64::MAX };
        let loop_end = loop_end.unwrap_or(total).min(total);

        if loop_end <= loop_start {
            return Err(anyhow!(
                "{}: loop end {} is not after loop start {}",
                path.display(),
                loop_end,
                loop_start
            ));
        }

        let body = loop_end - loop_start;
        // Never let the crossfade eat more than a third of the loop; past that
        // the result is a tremolo, not a loop.
        let xfade = crossfade_frames.min(body / 3);
        let mode = if xfade == 0 { LoopMode::Exact } else { LoopMode::Crossfade { frames: xfade } };

        let mut head = Vec::new();
        if xfade > 0 {
            head.resize(xfade as usize * reader.channels, 0.0);
            reader.seek_to_frame(loop_start)?;
            let got = reader.read(&mut head)?;
            if (got as u64) < xfade {
                head.truncate(got * reader.channels);
            }
        }

        reader.seek_to_frame(loop_start)?;

        Ok(Self { reader, loop_start, loop_end, mode, head, pos: loop_start })
    }

    pub fn channels(&self) -> usize {
        self.reader.channels
    }

    pub fn rate(&self) -> u32 {
        self.reader.rate
    }

    /// First frame of the crossfade region, or `loop_end` when there is none.
    fn fade_start(&self) -> u64 {
        match self.mode {
            LoopMode::Exact => self.loop_end,
            LoopMode::Crossfade { frames } => self.loop_end - frames,
        }
    }

    fn wrap(&mut self) -> Result<()> {
        // With a crossfade the head has already been played, mixed under the
        // tail, so playback resumes past it. Without one it resumes at the top.
        let resume = match self.mode {
            LoopMode::Exact => self.loop_start,
            LoopMode::Crossfade { frames } => self.loop_start + frames,
        };
        self.reader.seek_to_frame(resume)?;
        self.pos = resume;
        Ok(())
    }

    /// Fill `out` completely with interleaved frames. This never returns short —
    /// the stream has no end.
    pub fn read(&mut self, out: &mut [f32]) -> Result<()> {
        let ch = self.reader.channels;
        let want = out.len() / ch;
        let mut done = 0;
        // A pathological loop (empty body, unreadable file) must not spin here.
        let mut wraps = 0;

        while done < want {
            let fade_start = self.fade_start();
            let boundary = if self.pos < fade_start { fade_start } else { self.loop_end };
            let room = (boundary.saturating_sub(self.pos)) as usize;

            if room == 0 {
                self.wrap()?;
                wraps += 1;
                if wraps > 64 {
                    return Err(anyhow!("loop made no progress"));
                }
                continue;
            }

            let n = room.min(want - done);
            let got = self.reader.read(&mut out[done * ch..(done + n) * ch])?;

            if got > 0 {
                if let LoopMode::Crossfade { frames } = self.mode {
                    if self.pos >= fade_start {
                        let head_frames = self.head.len() / ch;
                        for i in 0..got {
                            let into_fade = (self.pos - fade_start) as usize + i;
                            if into_fade >= head_frames {
                                break;
                            }
                            let (out_g, in_g) = equal_power(into_fade as f32 / frames as f32);
                            let o = (done + i) * ch;
                            let h = into_fade * ch;
                            for c in 0..ch {
                                out[o + c] = out[o + c] * out_g + self.head[h + c] * in_g;
                            }
                        }
                    }
                }
                self.pos += got as u64;
                done += got;
                wraps = 0;
            }

            // Short read means the file ended earlier than its header claimed.
            // Treat wherever we stopped as the real loop end and carry on.
            if got < n {
                if got == 0 && self.pos < boundary {
                    self.loop_end = self.pos.max(self.loop_start + 1);
                }
                self.wrap()?;
                wraps += 1;
                if wraps > 64 {
                    return Err(anyhow!("loop made no progress"));
                }
            }
        }
        Ok(())
    }
}

// ─── Stream ───────────────────────────────────────────────────────────────────

/// Chunk size handed to the resampler. Bigger amortises the sinc cost, smaller
/// keeps the producer responsive to a stop request.
const RESAMPLE_CHUNK: usize = 1024;

/// A `LoopingSource` mapped to the device's channel count and sample rate.
pub struct Stream {
    src: LoopingSource,
    out_channels: usize,

    resampler: Option<SincFixedIn<f32>>,
    /// Interleaved source-channel input, one resampler chunk worth.
    in_interleaved: Vec<f32>,
    in_planar: Vec<Vec<f32>>,
    out_planar: Vec<Vec<f32>>,

    /// Device-rate, device-channel frames waiting to be handed out.
    ready: VecDeque<f32>,
}

impl Stream {
    pub fn open(
        path: &Path,
        loop_start: u64,
        loop_end: Option<u64>,
        crossfade_frames: u64,
        device_rate: u32,
        out_channels: usize,
    ) -> Result<Self> {
        let src = LoopingSource::open(path, loop_start, loop_end, crossfade_frames)?;
        let src_rate = src.rate();
        let src_ch = src.channels();

        // The common case — a pack transcoded at the device rate — skips the
        // resampler entirely rather than running it at ratio 1.0.
        let resampler = if src_rate == device_rate {
            None
        } else {
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Cubic,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            Some(SincFixedIn::<f32>::new(
                device_rate as f64 / src_rate as f64,
                1.0,
                params,
                RESAMPLE_CHUNK,
                out_channels,
            )?)
        };

        Ok(Self {
            src,
            out_channels,
            resampler,
            in_interleaved: vec![0.0; RESAMPLE_CHUNK * src_ch],
            in_planar: vec![vec![0.0; RESAMPLE_CHUNK]; out_channels],
            out_planar: vec![vec![0.0; RESAMPLE_CHUNK * 4]; out_channels],
            ready: VecDeque::with_capacity(RESAMPLE_CHUNK * 8),
        })
    }

    /// Mono sources go to every output channel; anything wider than the device
    /// keeps its first channels. Real surround downmixing is out of scope —
    /// Nuru's packs are mono or stereo.
    fn map_channels(&mut self, frames: usize) {
        let src_ch = self.src.channels();
        for c in 0..self.out_channels {
            let take = if src_ch == 1 { 0 } else { c.min(src_ch - 1) };
            let dst = &mut self.in_planar[c];
            for f in 0..frames {
                dst[f] = self.in_interleaved[f * src_ch + take];
            }
        }
    }

    /// Decode, map and resample one chunk into `ready`.
    fn pump(&mut self) -> Result<()> {
        self.src.read(&mut self.in_interleaved)?;
        self.map_channels(RESAMPLE_CHUNK);

        match self.resampler.as_mut() {
            None => {
                for f in 0..RESAMPLE_CHUNK {
                    for c in 0..self.out_channels {
                        self.ready.push_back(self.in_planar[c][f]);
                    }
                }
            }
            Some(rs) => {
                let (_, written) = rs.process_into_buffer(&self.in_planar, &mut self.out_planar, None)?;
                for f in 0..written {
                    for c in 0..self.out_channels {
                        self.ready.push_back(self.out_planar[c][f]);
                    }
                }
            }
        }
        Ok(())
    }

    /// Fill `out` with interleaved device-rate frames. Always fills completely.
    pub fn read(&mut self, out: &mut [f32]) -> Result<()> {
        let mut written = 0;
        while written < out.len() {
            if self.ready.is_empty() {
                self.pump()?;
            }
            while written < out.len() {
                match self.ready.pop_front() {
                    Some(s) => {
                        out[written] = s;
                        written += 1;
                    }
                    None => break,
                }
            }
        }
        Ok(())
    }
}
