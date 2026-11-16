
use std::f32::consts::FRAC_PI_2;

#[inline]
pub fn amplitude(fader: f32) -> f32 {
    let f = fader.clamp(0.0, 1.0);
    f * f
}

#[inline]
pub fn equal_power(t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    let a = t * FRAC_PI_2;
    (a.cos(), a.sin())
}

#[derive(Debug, Clone, Copy)]
pub struct Smoothed {
    current: f32,
    target: f32,
    coeff: f32,
}

impl Smoothed {
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

    #[inline]
    pub fn silence(&mut self) {
        self.current = 0.0;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        self.current += (self.target - self.current) * self.coeff;
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

pub mod tau {
    pub const FADER: f32 = 0.020;
    pub const LAYER_FADE: f32 = 0.35;
    pub const TRANSPORT: f32 = 0.12;
}
