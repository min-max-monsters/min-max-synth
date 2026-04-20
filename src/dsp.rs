//! DSP building blocks for the retro synth.
//!
//! Everything here is pure DSP: no allocations on the audio thread, no I/O.
//! Each block is sample-accurate and parameterised by sample rate.

use std::f32::consts::TAU;

/// Standard tuning constant: A4 = 440 Hz.
pub const A4_HZ: f32 = 440.0;

/// Convert a MIDI note (with optional fractional cents) to frequency in Hz.
#[inline]
pub fn midi_to_hz(note: f32) -> f32 {
    A4_HZ * (2.0_f32).powf((note - 69.0) / 12.0)
}

// ---------------------------------------------------------------------------
// Oscillators
// ---------------------------------------------------------------------------

/// The chip waveform families this synth can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    /// Pulse wave with adjustable duty (NES / Gameboy style).
    Pulse,
    /// 4-bit downsampled triangle (NES triangle channel).
    Triangle,
    /// 4-bit user-defined wavetable (Gameboy WAV channel) — we use a sine-ish
    /// stepped wave by default, but the table can be replaced.
    Wave4Bit,
    /// Pseudo-random LFSR noise with selectable period (NES / Gameboy noise).
    Noise,
    /// FM-style two-operator sine pair (very rough Genesis-flavoured tone).
    Fm,
    /// Sawtooth (sometimes useful for SID-flavoured leads).
    Saw,
}

impl Waveform {
    /// Human-readable label for UI.
    pub fn label(self) -> &'static str {
        match self {
            Waveform::Pulse => "Pulse",
            Waveform::Triangle => "Tri (4-bit)",
            Waveform::Wave4Bit => "Wave (4-bit)",
            Waveform::Noise => "Noise",
            Waveform::Fm => "FM 2-op",
            Waveform::Saw => "Saw",
        }
    }
}

/// Pulse / square oscillator with naive (band-unlimited) output. Aliasing is
/// part of the retro charm.
#[derive(Debug, Default, Clone)]
pub struct PulseOsc {
    phase: f32,
}

impl PulseOsc {
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Render one sample.
    /// `duty` is 0..1 (0.5 = square).
    #[inline]
    pub fn tick(&mut self, freq_hz: f32, sample_rate: f32, duty: f32) -> f32 {
        let s = if self.phase < duty { 1.0 } else { -1.0 };
        self.phase += freq_hz / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        s
    }
}

/// 4-bit stepped triangle (NES triangle channel — 32-step ramp).
#[derive(Debug, Default, Clone)]
pub struct TriangleOsc {
    phase: f32,
}

impl TriangleOsc {
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    #[inline]
    pub fn tick(&mut self, freq_hz: f32, sample_rate: f32) -> f32 {
        // 32-step ramp (0..=15..=0), bit-quantised, mapped to -1..1.
        let step = (self.phase * 32.0) as u32;
        let v = if step < 16 { step } else { 31 - step };
        let s = (v as f32 / 15.0) * 2.0 - 1.0;
        self.phase += freq_hz / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        s
    }
}

/// 32-sample, 4-bit wavetable oscillator (Gameboy WAV channel).
#[derive(Debug, Clone)]
pub struct WaveOsc {
    pub table: [u8; 32],
    phase: f32,
}

impl Default for WaveOsc {
    fn default() -> Self {
        // Default = quarter-sine-ish stepped wave.
        let mut table = [0u8; 32];
        for (i, t) in table.iter_mut().enumerate() {
            let s = (i as f32 / 32.0 * TAU).sin();
            *t = ((s * 0.5 + 0.5) * 15.0).round().clamp(0.0, 15.0) as u8;
        }
        Self { table, phase: 0.0 }
    }
}

impl WaveOsc {
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    #[inline]
    pub fn tick(&mut self, freq_hz: f32, sample_rate: f32) -> f32 {
        let i = (self.phase * 32.0) as usize & 31;
        let v = self.table[i] as f32;
        let s = (v / 15.0) * 2.0 - 1.0;
        self.phase += freq_hz / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        s
    }
}

/// LFSR noise generator. `short_period = true` selects the metallic
/// short-period mode (NES noise channel mode 1).
#[derive(Debug, Clone)]
pub struct NoiseOsc {
    reg: u16,
    accum: f32,
    last: f32,
}

impl Default for NoiseOsc {
    fn default() -> Self {
        Self { reg: 1, accum: 0.0, last: -1.0 }
    }
}

impl NoiseOsc {
    pub fn reset(&mut self) {
        self.reg = 1;
        self.accum = 0.0;
        self.last = -1.0;
    }

    #[inline]
    pub fn tick(&mut self, freq_hz: f32, sample_rate: f32, short_period: bool) -> f32 {
        // Step the LFSR at `freq_hz` regardless of sample rate.
        self.accum += freq_hz / sample_rate;
        while self.accum >= 1.0 {
            self.accum -= 1.0;
            let bit_a = self.reg & 1;
            let bit_b = if short_period {
                (self.reg >> 6) & 1
            } else {
                (self.reg >> 1) & 1
            };
            let feedback = bit_a ^ bit_b;
            self.reg >>= 1;
            self.reg |= feedback << 14;
            self.last = if (self.reg & 1) == 0 { 1.0 } else { -1.0 };
        }
        self.last
    }
}

/// Two-operator FM oscillator. Carrier is sine, modulator is sine.
/// `ratio` = modulator freq / carrier freq, `index` = modulation index.
#[derive(Debug, Default, Clone)]
pub struct FmOsc {
    carrier_phase: f32,
    mod_phase: f32,
}

impl FmOsc {
    pub fn reset(&mut self) {
        self.carrier_phase = 0.0;
        self.mod_phase = 0.0;
    }

    #[inline]
    pub fn tick(&mut self, freq_hz: f32, sample_rate: f32, ratio: f32, index: f32) -> f32 {
        let modv = (self.mod_phase * TAU).sin() * index;
        let s = ((self.carrier_phase + modv / TAU) * TAU).sin();
        let cinc = freq_hz / sample_rate;
        self.carrier_phase += cinc;
        self.mod_phase += cinc * ratio;
        if self.carrier_phase >= 1.0 {
            self.carrier_phase -= 1.0;
        }
        if self.mod_phase >= 1.0 {
            self.mod_phase -= 1.0;
        }
        s
    }
}

/// Naive saw (aliased — intentional for retro vibe).
#[derive(Debug, Default, Clone)]
pub struct SawOsc {
    phase: f32,
}

impl SawOsc {
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
    #[inline]
    pub fn tick(&mut self, freq_hz: f32, sample_rate: f32) -> f32 {
        let s = self.phase * 2.0 - 1.0;
        self.phase += freq_hz / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Linear ADSR envelope. Times are in seconds, sustain in 0..1.
#[derive(Debug, Clone)]
pub struct Adsr {
    stage: EnvStage,
    level: f32,
    sample_rate: f32,
    release_decrement: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Adsr {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            stage: EnvStage::Idle,
            level: 0.0,
            sample_rate,
            release_decrement: 0.0,
            attack: 0.005,
            decay: 0.1,
            sustain: 0.7,
            release: 0.15,
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
    }

    pub fn note_on(&mut self) {
        self.stage = EnvStage::Attack;
    }

    pub fn note_off(&mut self) {
        if self.stage != EnvStage::Idle {
            // Pre-compute a linear decrement so the release ends in finite time.
            self.release_decrement = if self.release <= 0.0 {
                1.0
            } else {
                self.level.max(1e-6) / (self.release * self.sample_rate)
            };
            self.stage = EnvStage::Release;
        }
    }

    pub fn is_active(&self) -> bool {
        self.stage != EnvStage::Idle
    }

    #[inline]
    pub fn tick(&mut self) -> f32 {
        match self.stage {
            EnvStage::Idle => self.level = 0.0,
            EnvStage::Attack => {
                let inc = if self.attack <= 0.0 { 1.0 } else { 1.0 / (self.attack * self.sample_rate) };
                self.level += inc;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = EnvStage::Decay;
                }
            }
            EnvStage::Decay => {
                let dec = if self.decay <= 0.0 { 1.0 } else { (1.0 - self.sustain) / (self.decay * self.sample_rate) };
                self.level -= dec;
                if self.level <= self.sustain {
                    self.level = self.sustain;
                    self.stage = EnvStage::Sustain;
                }
            }
            EnvStage::Sustain => self.level = self.sustain,
            EnvStage::Release => {
                self.level -= self.release_decrement;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = EnvStage::Idle;
                }
            }
        }
        self.level
    }
}

// ---------------------------------------------------------------------------
// LFO and pitch sweep
// ---------------------------------------------------------------------------

/// Sine LFO used for vibrato.
#[derive(Debug, Default, Clone)]
pub struct Lfo {
    phase: f32,
}

impl Lfo {
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    #[inline]
    pub fn tick(&mut self, rate_hz: f32, sample_rate: f32) -> f32 {
        let s = (self.phase * TAU).sin();
        self.phase += rate_hz / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        s
    }
}

/// Linear pitch sweep in semitones over the sweep time. Positive = up.
#[derive(Debug, Default, Clone)]
pub struct Sweep {
    elapsed: f32,
}

impl Sweep {
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
    }

    /// Returns the current pitch offset in semitones.
    #[inline]
    pub fn tick(&mut self, sample_rate: f32, semitones: f32, time_s: f32) -> f32 {
        if time_s <= 0.0 {
            return semitones;
        }
        self.elapsed = (self.elapsed + 1.0 / sample_rate).min(time_s);
        semitones * (self.elapsed / time_s)
    }
}

// ---------------------------------------------------------------------------
// Bitcrusher
// ---------------------------------------------------------------------------

/// Bit-depth + sample-rate reduction for that lo-fi sheen.
#[derive(Debug, Default, Clone)]
pub struct BitCrusher {
    hold: f32,
    accum: f32,
}

impl BitCrusher {
    #[inline]
    pub fn process(
        &mut self,
        input: f32,
        sample_rate: f32,
        target_rate: f32,
        bits: f32,
    ) -> f32 {
        let target = target_rate.max(1.0).min(sample_rate);
        self.accum += target / sample_rate;
        if self.accum >= 1.0 {
            self.accum -= 1.0;
            let levels = (2.0_f32).powf(bits.clamp(1.0, 16.0)) - 1.0;
            self.hold = ((input * 0.5 + 0.5) * levels).round() / levels * 2.0 - 1.0;
        }
        self.hold
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_a4_is_440() {
        assert!((midi_to_hz(69.0) - 440.0).abs() < 1e-3);
    }

    #[test]
    fn pulse_oscillates() {
        let mut osc = PulseOsc::default();
        let mut sum = 0.0;
        for _ in 0..1000 {
            sum += osc.tick(440.0, 44_100.0, 0.5);
        }
        // Square wave should average near 0.
        assert!(sum.abs() < 50.0);
    }

    #[test]
    fn adsr_full_cycle_reaches_zero() {
        let mut env = Adsr::new(48_000.0);
        env.attack = 0.001;
        env.decay = 0.001;
        env.sustain = 0.5;
        env.release = 0.001;
        env.note_on();
        for _ in 0..1000 {
            env.tick();
        }
        assert!((env.tick() - 0.5).abs() < 0.01);
        env.note_off();
        for _ in 0..1000 {
            env.tick();
        }
        assert!(!env.is_active());
    }

    #[test]
    fn noise_changes() {
        let mut n = NoiseOsc::default();
        let mut a = 0u32;
        let mut b = 0u32;
        for _ in 0..2000 {
            if n.tick(8000.0, 48_000.0, false) > 0.0 { a += 1 } else { b += 1 }
        }
        assert!(a > 100 && b > 100);
    }
}
