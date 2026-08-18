
use std::f32::consts::TAU;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::source::{LoopingSource, Stream};

const RATE: u32 = 48_000;
const FREQ: f32 = 100.0;
const PERIOD: u64 = 480;

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("nuru-audio-tests");
    fs::create_dir_all(&dir).unwrap();
    dir
}

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
    f.write_all(&1u16.to_le_bytes()).unwrap();
    f.write_all(&channels.to_le_bytes()).unwrap();
    f.write_all(&rate.to_le_bytes()).unwrap();
    f.write_all(&byte_rate.to_le_bytes()).unwrap();
    f.write_all(&block_align.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&data_len.to_le_bytes()).unwrap();
    for s in samples {
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}

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

fn max_step(buf: &[f32], channels: usize) -> f32 {
    buf.chunks_exact(channels)
        .map(|f| f[0])
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0f32, f32::max)
}

fn expected_step() -> f32 {
    TAU * FREQ / RATE as f32 * 0.8
}

#[test]
fn exact_loop_has_no_seam() {
    let path = sine_wav("exact.wav", PERIOD * 10);
    let mut src = LoopingSource::open(&path, 0, None, 0).unwrap();

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
    let path = sine_wav("short.wav", PERIOD * 10);
    let mut src = LoopingSource::open(&path, 0, Some(PERIOD), 0).unwrap();

    let mut out = vec![0.0f32; (PERIOD * 25) as usize];
    src.read(&mut out).unwrap();

    assert!(out.iter().all(|s| s.is_finite()), "non-finite sample in output");
    assert!(
        out.iter().any(|s| s.abs() > 0.1),
        "output is silent - the loop stopped producing"
    );
}

#[test]
fn crossfade_is_capped_at_a_third_of_the_loop() {
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
    let src_rate = 44_100u32;
    let frames = src_rate as u64;
    let path = tmp_dir().join("resample.wav");
    let samples: Vec<i16> = (0..frames)
        .map(|n| {
            let t = n as f32 / src_rate as f32;
            ((TAU * FREQ * t).sin() * 0.8 * i16::MAX as f32) as i16
        })
        .collect();
    write_wav(&path, src_rate, 1, &samples);

    let mut stream = Stream::open(&path, 0, None, 0, RATE, 2).unwrap();
    let mut out = vec![0.0f32; RATE as usize * 2];
    stream.read(&mut out).unwrap();

    assert!(out.iter().all(|s| s.is_finite()), "resampler produced non-finite samples");

    let step = max_step(&out, 2);
    assert!(
        step < expected_step() * 3.0,
        "resampled output is discontinuous: max step {step:.5}"
    );

    let peak = out.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    assert!(peak > 0.5 && peak < 1.2, "resampled peak drifted to {peak:.3}");
}

#[test]
fn a_sound_can_be_switched_off_and_straight_back_on() {
    use super::AudioEngine;

    let Ok(engine) = AudioEngine::open(None) else {
        eprintln!("no audio device available, skipping");
        return;
    };

    let path = sine_wav("toggle.wav", PERIOD * 10);
    let add = |fader: f32| {
        engine.add_layer("toggle", path.clone(), 0, None, 0, fader)
    };

    add(0.5).unwrap();
    assert!(engine.active_sounds().contains(&"toggle".to_string()));

    engine.remove_layer("toggle").unwrap();

    add(0.5).unwrap();
    assert!(
        engine.active_sounds().contains(&"toggle".to_string()),
        "the sound did not come back after being switched off and on"
    );

    for _ in 0..6 {
        engine.remove_layer("toggle").unwrap();
        add(0.5).unwrap();
        assert!(
            engine.active_sounds().contains(&"toggle".to_string()),
            "a rapid toggle left the sound missing"
        );
    }
}
