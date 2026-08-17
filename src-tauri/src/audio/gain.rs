//! Gain maths. Everything here runs in the audio callback, so it is branch-light
//! and allocation-free.

use std::f32::consts::FRAC_PI_2;

/// Fader position (0..=1) to linear amplitude.
///
/// A fader that maps straight to amplitude feels wrong: the top half of the
/// travel does almost nothing and the bottom tenth does everything. A square law
/// is the usual fix and is close enough to perceived loudness over the range a
/// listener actually uses, without the `-inf dB` cliff a true log curve has at
/// zero.
#[inline]
pub fn amplitude(fader: f32) -> f32 {
    let f = fader.clamp(0.0, 1.0);
    f * f
}

/// Equal-power crossfade pair for `t` in 0..=1.
///
/// `sin`/`cos` rather than `t`/`1-t`: two uncorrelated ambient beds summed with a
/// linear fade lose ~3 dB in the middle of the crossover, which is audible as a
/// dip every time the loop wraps. The sine pair keeps the summed power constant.
#[inline]
pub fn equal_power(t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    let a = t * FRAC_PI_2;
    (a.cos(), a.sin())
}

/// One-pole smoother, so a gain change never arrives as a step.
///
/// A step in gain is a discontinuity in the waveform, which is a click. Ramping
/// over a few milliseconds removes it; ramping over a second or two turns
/// switching a sound on into a fade-in.
#[derive(Debug, Clone, Copy)]
pub struct Smoothed {
    current: f32,
    target: f32,
    coeff: f32,
}

impl Smoothed {
    /// `tau_secs` is the time constant: the value covers ~63% of the remaining
    /// distance in that time, and is inaudibly close after about 4×.
    pub fn new(initial: f32, tau_secs: f32, sample_rate: u32) -> Self {
        Self {
            current: initial,
            target: initial,
            coeff: Self::coeff(tau_secs, sample_rate),
        }
    }

    fn coeff(tau_secs: f32, sample_rate: u32) -> f32 {
        if tau_secs <= 0.0 {
            return 1.0;
        }
        1.0 - (-1.0 / (tau_secs * sample_rate as f32)).exp()
    }

    pub fn set_tau(&mut self, tau_secs: f32, sample_rate: u32) {
        self.coeff = Self::coeff(tau_secs, sample_rate);
    }

    #[inline]
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    #[inline]
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Jump straight to a value, skipping the ramp. Only for initialisation.
    #[inline]
    pub fn reset(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    /// Advance one frame and return the gain to use for it.
    #[inline]
    pub fn next(&mut self) -> f32 {
        self.current += (self.target - self.current) * self.coeff;
        // Snap once we are far below anything audible, so a fade-out actually
        // reaches silence instead of asymptotically approaching it forever.
        if (self.target - self.current).abs() < 1.0e-5 {
            self.current = self.target;
        }
        self.current
    }

    #[inline]
    pub fn settled(&self) -> bool {
        (self.target - self.current).abs() < 1.0e-5
    }
}

/// Time constants used across the engine, in seconds.
pub mod tau {
    /// A fader the user is dragging. Long enough to kill zipper noise, short
    /// enough that the sound tracks the handle.
    pub const FADER: f32 = 0.020;
    /// Switching a sound on or off.
    pub const LAYER_FADE: f32 = 0.35;
    /// Master play/pause.
    pub const TRANSPORT: f32 = 0.12;
}
