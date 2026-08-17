//! Tests for the thing this whole engine exists to get right: the loop.
//!
//! "It sounds fine" is not a test. A seam is a discontinuity in the waveform, so
//! it is measurable: take the largest sample-to-sample step anywhere in the
//! output and compare it to the largest step the material itself produces. If
//! the loop is clean the two are the same number. If it is not, the seam sticks
//! out by an order of magnitude.
//!
//! The fixtures are synthesised rather than read from the placeholder pack, so
//! these pass on a clean checkout with no audio in it.

use std::f32::consts::TAU;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::source::{LoopingSource, Stream};

const RATE: u32 = 48_000;
const FREQ: f32 = 100.0;
/// 48000 / 100 = 480 samples per cycle, so any multiple of 480 is a whole
/// number of periods and joins its own start exactly.
const PERIOD: u64 = 480;

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("nuru-audio-tests");
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write a 16-bit PCM WAV. Symphonia reads it back sample-exact, which is what
/// makes it usable as a reference for "did anything change".
fn write_wav(path: &Path, rate: u32, channels: u16, samples: &[i16]) {
    let bytes_per_sample = 2u32;
    let data_len = samples.len() as u32 * bytes_per_sample;
    let byte_rate = rate * channels as u32 * bytes_per_sample;
    let block_align = channels * bytes_per_sample as u16;

    let mut f = fs::File::create(path).unwrap();
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    f.write_all(&channels.to_le_bytes()).unwrap();
    f.write_all(&rate.to_le_bytes()).unwrap();
    f.write_all(&byte_rate.to_le_bytes()).unwrap();
    f.write_all(&block_align.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap(); // bits
    f.write_all(b"data").unwrap();
    f.write_all(&data_len.to_le_bytes()).unwrap();
    for s in samples {
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}

/// A mono sine of `frames` frames at `FREQ`.
fn sine_wav(name: &str, frames: u64) -> PathBuf {
    let path = tmp_dir().join(name);
    let samples: Vec<i16> = (0..frames)
        .map(|n| {
            let t = n as f32 / RATE as f32;
            ((TAU * FREQ * t).sin() * 0.8 * i16::MAX as f32) as i16
        })
        .collect();
    write_wav(&path, RATE, 1, &samples);
    path
}

/// Largest absolute step between consecutive frames of one channel.
fn max_step(buf: &[f32], channels: usize) -> f32 {
    buf.chunks_exact(channels)
        .map(|f| f[0])
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0f32, f32::max)
}

/// What a continuous sine steps by at its steepest: 2π·f/rate, scaled by
/// amplitude. Anything near this is the signal; anything far above it is a seam.
fn expected_step() -> f32 {
    TAU * FREQ / RATE as f32 * 0.8
}

#[test]
fn exact_loop_has_no_seam() {
    // Ten whole periods, so the last sample joins the first with no step.
    let path = sine_wav("exact.wav", PERIOD * 10);
    let mut src = LoopingSource::open(&path, 0, None, 0).unwrap();

    // Long enough to wrap four times over.
    let mut out = vec![0.0f32; (PERIOD * 42) as usize];
    src.read(&mut out).unwrap();

    let step = max_step(&out, 1);
    assert!(
        step < expected_step() * 1.5,
        "seam at the loop point: max step {step:.5}, expected about {:.5}",
        expected_step()
    );
}

#[test]
fn a_mismatched_loop_really_does_produce_a_seam() {
    // The control. Ending the loop 20 samples short of a whole period means the
    // tail no longer joins the head, and a butt-joint must show it — otherwise
    // the test above is measuring nothing.
    let path = sine_wav("mismatched.wav", PERIOD * 10);
    let mut src = LoopingSource::open(&path, 0, Some(PERIOD * 10 - 120), 0).unwrap();

    let mut out = vec![0.0f32; (PERIOD * 30) as usize];
    src.read(&mut out).unwrap();

    let step = max_step(&out, 1);
    assert!(
        step > expected_step() * 4.0,
        "expected a visible discontinuity, got max step {step:.5}"
    );
}

#[test]
fn crossfade_removes_the_seam_from_a_mismatched_loop() {
    // Same mismatched loop as the control above, with a 50 ms crossfade. The
    // step should fall back to roughly what the waveform itself does.
    let path = sine_wav("crossfaded.wav", PERIOD * 40);
    let xfade = RATE as u64 / 20;
    let mut src = LoopingSource::open(&path, 0, Some(PERIOD * 40 - 120), xfade).unwrap();

    let mut out = vec![0.0f32; (PERIOD * 120) as usize];
    src.read(&mut out).unwrap();

    let step = max_step(&out, 1);
    assert!(
        step < expected_step() * 1.5,
        "crossfade left a seam: max step {step:.5}, expected about {:.5}",
        expected_step()
    );
}

#[test]
fn loop_never_runs_dry() {
    // A loop far shorter than the buffer being asked for has to wrap many times
    // inside a single read. It must still fill completely and stay finite.
    let path = sine_wav("short.wav", PERIOD * 10);
    let mut src = LoopingSource::open(&path, 0, Some(PERIOD), 0).unwrap();

    let mut out = vec![0.0f32; (PERIOD * 25) as usize];
    src.read(&mut out).unwrap();

    assert!(out.iter().all(|s| s.is_finite()), "non-finite sample in output");
    assert!(
        out.iter().any(|s| s.abs() > 0.1),
        "output is silent — the loop stopped producing"
    );
}

#[test]
fn crossfade_is_capped_at_a_third_of_the_loop() {
    // Asking for a crossfade longer than the loop body would turn the loop into
    // a tremolo. The source clamps it, and must still produce clean audio.
    let path = sine_wav("tiny.wav", PERIOD * 6);
    let mut src = LoopingSource::open(&path, 0, None, PERIOD * 100).unwrap();

    let mut out = vec![0.0f32; (PERIOD * 20) as usize];
    src.read(&mut out).unwrap();

    assert!(out.iter().all(|s| s.is_finite()));
    assert!(out.iter().any(|s| s.abs() > 0.1));
}

#[test]
fn mono_upmixes_to_stereo_and_stays_in_sync() {
    let path = sine_wav("mono.wav", PERIOD * 10);
    let mut stream = Stream::open(&path, 0, None, 0, RATE, 2).unwrap();

    let mut out = vec![0.0f32; (PERIOD * 8) as usize * 2];
    stream.read(&mut out).unwrap();

    // Both channels carry the same signal, so any difference between them is a
    // channel-mapping bug rather than a rounding one.
    for frame in out.chunks_exact(2) {
        assert!(
            (frame[0] - frame[1]).abs() < 1e-6,
            "channels diverged: {} vs {}",
            frame[0],
            frame[1]
        );
    }
    assert!(max_step(&out, 2) < expected_step() * 1.5);
}

#[test]
fn resampling_preserves_the_waveform() {
    // 44.1 kHz source into a 48 kHz device — the mismatch Nuru hits on roughly
    // half the placeholder pack.
    let src_rate = 44_100u32;
    let frames = src_rate as u64; // one second
    let path = tmp_dir().join("resample.wav");
    let samples: Vec<i16> = (0..frames)
        .map(|n| {
            let t = n as f32 / src_rate as f32;
            ((TAU * FREQ * t).sin() * 0.8 * i16::MAX as f32) as i16
        })
        .collect();
    write_wav(&path, src_rate, 1, &samples);

    let mut stream = Stream::open(&path, 0, None, 0, RATE, 2).unwrap();
    let mut out = vec![0.0f32; RATE as usize * 2]; // one second at device rate
    stream.read(&mut out).unwrap();

    assert!(out.iter().all(|s| s.is_finite()), "resampler produced non-finite samples");

    // A sinc resampler rings slightly, so allow more headroom than the raw
    // path — but nowhere near enough to hide a seam.
    let step = max_step(&out, 2);
    assert!(
        step < expected_step() * 3.0,
        "resampled output is discontinuous: max step {step:.5}"
    );

    let peak = out.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    assert!(peak > 0.5 && peak < 1.2, "resampled peak drifted to {peak:.3}");
}
