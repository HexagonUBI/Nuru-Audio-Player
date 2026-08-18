
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

pub struct FrameReader {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_buf: Option<SampleBuffer<f32>>,

    pending: Vec<f32>,
    pending_cursor: usize,

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
                    if buf.capacity() < decoded.frames() * spec.channels.count() {
                        *buf = SampleBuffer::<f32>::new(capacity, spec);
                    }
                    buf.copy_interleaved_ref(decoded);
                    self.pending.clear();
                    self.pending.extend_from_slice(buf.samples());
                    self.pending_cursor = 0;
                    return Ok(true);
                }
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }

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

#[derive(Debug, Clone, Copy)]
pub enum LoopMode {
    Exact,
    Crossfade { frames: u64 },
}

pub struct LoopingSource {
    reader: FrameReader,
    loop_start: u64,
    loop_end: u64,
    mode: LoopMode,
    head: Vec<f32>,
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

    fn fade_start(&self) -> u64 {
        match self.mode {
            LoopMode::Exact => self.loop_end,
            LoopMode::Crossfade { frames } => self.loop_end - frames,
        }
    }

    fn wrap(&mut self) -> Result<()> {
        let resume = match self.mode {
            LoopMode::Exact => self.loop_start,
            LoopMode::Crossfade { frames } => self.loop_start + frames,
        };
        self.reader.seek_to_frame(resume)?;
        self.pos = resume;
        Ok(())
    }

    pub fn read(&mut self, out: &mut [f32]) -> Result<()> {
        let ch = self.reader.channels;
        let want = out.len() / ch;
        let mut done = 0;
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

const RESAMPLE_CHUNK: usize = 1024;

pub struct Stream {
    src: LoopingSource,
    out_channels: usize,

    resampler: Option<SincFixedIn<f32>>,
    in_interleaved: Vec<f32>,
    in_planar: Vec<Vec<f32>>,
    out_planar: Vec<Vec<f32>>,

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
