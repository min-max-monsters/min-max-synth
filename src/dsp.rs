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

/// PolyBLEP band-limiting kernel. Returns the correction value to add (or
/// subtract, mirrored by sign) at a unit-amplitude discontinuity. `t` is the
/// oscillator phase in [0, 1) and `dt` is the per-sample phase increment.
#[inline]
fn poly_blep(mut t: f32, dt: f32) -> f32 {
    if t < dt {
        t /= dt;
        2.0 * t - t * t - 1.0
    } else if t > 1.0 - dt {
        t = (t - 1.0) / dt;
        t * t + 2.0 * t + 1.0
    } else {
        0.0
    }
}

/// Pulse / square oscillator using PolyBLEP to suppress aliasing. The
/// underlying waveform is still a hard square (so the "chip" character is
/// preserved), but the discontinuities are band-limited.
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
        let dt = (freq_hz / sample_rate).max(0.0);
        let duty = duty.clamp(0.01, 0.99);
        let mut s = if self.phase < duty { 1.0 } else { -1.0 };
        // Rising edge at phase=0.
        s += poly_blep(self.phase, dt);
        // Falling edge at phase=duty.
        let mut t2 = self.phase - duty;
        if t2 < 0.0 {
            t2 += 1.0;
        }
        s -= poly_blep(t2, dt);
        self.phase += dt;
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

/// Saw oscillator with PolyBLEP anti-aliasing on the falling edge.
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
        let dt = (freq_hz / sample_rate).max(0.0);
        let mut s = self.phase * 2.0 - 1.0;
        s -= poly_blep(self.phase, dt);
        self.phase += dt;
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

/// "Auto bend": at note-on, the pitch starts offset by `semitones` and
/// linearly returns to 0 over `time_s`. Negative `semitones` bends up to
/// pitch from below; positive bends down. Set `time_s = 0` to disable.
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
        if time_s <= 0.0 || semitones == 0.0 {
            return 0.0;
        }
        self.elapsed = (self.elapsed + 1.0 / sample_rate).min(time_s);
        semitones * (1.0 - self.elapsed / time_s)
    }
}

// ---------------------------------------------------------------------------
// Drum voice (procedural percussion synthesizer)
// ---------------------------------------------------------------------------

/// Per-trigger parameter snapshot for `DrumVoice::tick`.
#[derive(Debug, Clone, Copy)]
pub struct DrumParams {
    /// 0 = off (noise only), 1 = sine, 2 = triangle, 3 = square.
    pub wave: i32,
    /// Target tone frequency in Hz (after pitch envelope settles).
    pub freq: f32,
    /// Second tone frequency multiplier (0 disables the 2nd osc).
    pub ratio: f32,
    /// Noise mix amount, 0..1.
    pub noise: f32,
    /// Pitch offset in semitones at trigger time.
    pub pitch_env: f32,
    /// Time constant in seconds for the pitch offset's exponential decay.
    pub pitch_time: f32,
    /// Time constant in seconds for the amplitude envelope.
    pub decay: f32,
    /// 0..1 amount of multi-attack (clap-style retriggers).
    pub burst: f32,
    /// Output level multiplier.
    pub level: f32,
}

/// Self-contained percussion voice: tone osc(s) + noise + pitch and
/// amplitude envelopes. Designed to be able to recreate every built-in drum
/// sample purely from parameters.
#[derive(Debug, Clone)]
pub struct DrumVoice {
    active: bool,
    elapsed: f32,
    phase1: f32,
    phase2: f32,
    lfsr: u16,
    burst_offsets: [f32; 4],
    burst_count: usize,
}

impl Default for DrumVoice {
    fn default() -> Self {
        Self {
            active: false,
            elapsed: 0.0,
            phase1: 0.0,
            phase2: 0.0,
            lfsr: 0xACE1,
            burst_offsets: [0.0; 4],
            burst_count: 1,
        }
    }
}

impl DrumVoice {
    pub fn trigger(&mut self, burst: f32) {
        self.active = true;
        self.elapsed = 0.0;
        self.phase1 = 0.0;
        self.phase2 = 0.0;
        self.lfsr = 0xACE1;
        // burst 0 -> 1 onset, burst 1 -> 4 onsets ~12-15ms apart.
        let count = (1.0 + burst.clamp(0.0, 1.0) * 3.0).round() as usize;
        self.burst_count = count.clamp(1, 4);
        for i in 0..self.burst_count {
            // Slight jitter so it sounds like hands, not metronome.
            self.burst_offsets[i] = i as f32 * 0.012 + i as f32 * 0.003 * burst;
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    #[inline]
    fn osc(phase: f32, w: i32) -> f32 {
        match w {
            0 => 0.0,
            1 => (phase * TAU).sin(),
            2 => 1.0 - 4.0 * (phase - 0.5).abs(),
            _ => if phase < 0.5 { 1.0 } else { -1.0 },
        }
    }

    #[inline]
    pub fn tick(&mut self, dp: &DrumParams, sample_rate: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let t = self.elapsed;
        self.elapsed += 1.0 / sample_rate;

        // Pitch envelope: pitch_off = pitch_env * exp(-t / pitch_time).
        let pt = dp.pitch_time.max(0.0001);
        let pitch_off = dp.pitch_env * (-t / pt).exp();
        let nyq = sample_rate * 0.45;
        let freq = (dp.freq * (2.0_f32).powf(pitch_off / 12.0)).clamp(1.0, nyq);

        // Tone(s)
        let tone = if dp.wave == 0 {
            0.0
        } else {
            let t1 = Self::osc(self.phase1, dp.wave);
            self.phase1 += freq / sample_rate;
            if self.phase1 >= 1.0 {
                self.phase1 -= 1.0;
            }
            let t2 = if dp.ratio > 0.001 {
                let f2 = (freq * dp.ratio).clamp(1.0, nyq);
                let v = Self::osc(self.phase2, dp.wave);
                self.phase2 += f2 / sample_rate;
                if self.phase2 >= 1.0 {
                    self.phase2 -= 1.0;
                }
                v * 0.7
            } else {
                0.0
            };
            t1 + t2
        };

        // Noise via 15-bit LFSR (same shape as samples.rs).
        let bit = ((self.lfsr ^ (self.lfsr >> 1)) & 1) as u16;
        self.lfsr = (self.lfsr >> 1) | (bit << 14);
        let noise = if (self.lfsr & 1) == 0 { 1.0 } else { -1.0 };

        let mix = tone + noise * dp.noise;

        // Amplitude envelope = sum of exponential decays at burst onsets.
        let decay_s = dp.decay.max(0.001);
        let mut env = 0.0_f32;
        for i in 0..self.burst_count {
            let dt = t - self.burst_offsets[i];
            if dt >= 0.0 {
                let scale = 1.0 - i as f32 * 0.15;
                env += scale * (-dt / decay_s).exp();
            }
        }

        // Auto-deactivate once well past the last burst and amplitude is
        // negligible. Saves CPU for one-shot voices.
        let last_onset = self.burst_offsets[self.burst_count.saturating_sub(1)];
        if t > last_onset + decay_s * 8.0 && env < 0.0005 {
            self.active = false;
        }

        mix * env * dp.level
    }
}

// ---------------------------------------------------------------------------
// One-pole filters (authentic 6 dB/oct RC rolloff)
// ---------------------------------------------------------------------------

/// One-pole lowpass filter — mimics the analog RC filter on retro DAC output
/// stages. 6 dB/octave slope, zero resonance, dirt cheap.
#[derive(Debug, Clone)]
pub struct OnePoleLP {
    y: f32,
}

impl Default for OnePoleLP {
    fn default() -> Self {
        Self { y: 0.0 }
    }
}

impl OnePoleLP {
    /// Process one sample. `cutoff` is in Hz.
    /// When cutoff >= sample_rate * 0.5 the filter is effectively bypassed.
    #[inline]
    pub fn process(&mut self, input: f32, cutoff: f32, sample_rate: f32) -> f32 {
        if cutoff >= sample_rate * 0.5 {
            self.y = input;
            return input;
        }
        let alpha = 1.0 - (-TAU * cutoff / sample_rate).exp();
        self.y += alpha * (input - self.y);
        self.y
    }

    pub fn reset(&mut self) {
        self.y = 0.0;
    }
}

/// One-pole highpass filter — mimics the DC-blocking capacitor on the NES
/// mixer (~37 Hz) and similar retro output stages. 6 dB/octave slope.
#[derive(Debug, Clone)]
pub struct OnePoleHP {
    x_prev: f32,
    y: f32,
}

impl Default for OnePoleHP {
    fn default() -> Self {
        Self { x_prev: 0.0, y: 0.0 }
    }
}

impl OnePoleHP {
    /// Process one sample. `cutoff` is in Hz.
    /// When cutoff <= 1.0 the filter is effectively bypassed.
    #[inline]
    pub fn process(&mut self, input: f32, cutoff: f32, sample_rate: f32) -> f32 {
        if cutoff <= 1.0 {
            self.x_prev = input;
            self.y = input;
            return input;
        }
        let alpha = (-TAU * cutoff / sample_rate).exp();
        self.y = alpha * (self.y + input - self.x_prev);
        self.x_prev = input;
        self.y
    }

    pub fn reset(&mut self) {
        self.x_prev = 0.0;
        self.y = 0.0;
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
// Formant speech synthesizer (Speak & Spell / early TTS inspired)
// ---------------------------------------------------------------------------

/// Number of formant resonators per voice.
const NUM_FORMANTS: usize = 4;

/// Number of built-in phonemes.
pub const NUM_PHONEMES: usize = 36;

/// Phoneme identity for the speech mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phoneme {
    // Vowels (0–9)
    Ah = 0,   // "father"
    Ee = 1,   // "see"
    Ih = 2,   // "sit"
    Eh = 3,   // "bed"
    Ae = 4,   // "cat"
    Uh = 5,   // "but"
    Oh = 6,   // "go"
    Oo = 7,   // "boot"
    Aw = 8,   // "law"
    Er = 9,   // "her"
    // Nasals (10–11)
    Mm = 10,
    Nn = 11,
    // Liquids (12–13)
    Ll = 12,
    Rr = 13,
    // Fricatives (14–16)
    Ss = 14,
    Sh = 15,
    Ff = 16,
    // Voiced fricatives (17–18)
    Zz = 17,
    Vv = 18,
    // Stops / plosives (19–22) — modelled as brief silence + noise burst
    Bb = 19,  // voiced bilabial
    Dd = 20,  // voiced alveolar
    Gg = 21,  // voiced velar
    Kk = 22,  // unvoiced velar
    // Silence (23) — for gaps between consonants
    Sil = 23,
    // Aspirate / glottal (24)
    Hh = 24,  // breathy "h" sound
    // Unvoiced alveolar stop (25)
    Tt = 25,
    // Diphthongs (26–28)
    Ay = 26,  // "my"
    Ow = 27,  // "now"
    Ey = 28,  // "day"
    // Unvoiced bilabial stop (29)
    Pp = 29,
    // Semivowels (30–31)
    Ww = 30,  // "we"
    Yy = 31,  // "yes"
    // Velar nasal (32)
    Ng = 32,  // "sing"
    // Affricate (33)
    Ch = 33,  // "church"
    // Dental fricatives (34–35)
    Th = 34,  // "thin" (unvoiced)
    Dh = 35,  // "the" (voiced)
}

/// Phoneme spec: (freq, bw, amplitude) for each of 4 formants, plus
/// voicing (0.0 = noise, 1.0 = voiced) and overall gain.
#[derive(Debug, Clone, Copy)]
struct PhonemeSpec {
    formants: [(f32, f32, f32); NUM_FORMANTS], // (freq, bandwidth, amplitude)
    voiced: f32,
    gain: f32,
}

impl Phoneme {
    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Self::Ah,  1 => Self::Ee,  2 => Self::Ih,  3 => Self::Eh,
            4 => Self::Ae,  5 => Self::Uh,  6 => Self::Oh,  7 => Self::Oo,
            8 => Self::Aw,  9 => Self::Er,  10 => Self::Mm, 11 => Self::Nn,
            12 => Self::Ll, 13 => Self::Rr, 14 => Self::Ss, 15 => Self::Sh,
            16 => Self::Ff, 17 => Self::Zz, 18 => Self::Vv, 19 => Self::Bb,
            20 => Self::Dd, 21 => Self::Gg, 22 => Self::Kk, 23 => Self::Sil,
            24 => Self::Hh, 25 => Self::Tt, 26 => Self::Ay, 27 => Self::Ow,
            28 => Self::Ey, 29 => Self::Pp, 30 => Self::Ww, 31 => Self::Yy,
            32 => Self::Ng, 33 => Self::Ch, 34 => Self::Th, 35 => Self::Dh,
            _ => Self::Sil,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ah => "AH", Self::Ee => "EE", Self::Ih => "IH", Self::Eh => "EH",
            Self::Ae => "AE", Self::Uh => "UH", Self::Oh => "OH", Self::Oo => "OO",
            Self::Aw => "AW", Self::Er => "ER", Self::Mm => "MM", Self::Nn => "NN",
            Self::Ll => "LL", Self::Rr => "RR", Self::Ss => "SS", Self::Sh => "SH",
            Self::Ff => "FF", Self::Zz => "ZZ", Self::Vv => "VV", Self::Bb => "BB",
            Self::Dd => "DD", Self::Gg => "GG", Self::Kk => "KK", Self::Sil => " _ ",
            Self::Hh => "HH", Self::Tt => "TT", Self::Ay => "AY",
            Self::Ow => "OW", Self::Ey => "EY", Self::Pp => "PP",
            Self::Ww => "WW", Self::Yy => "YY", Self::Ng => "NG",
            Self::Ch => "CH", Self::Th => "TH", Self::Dh => "DH",
        }
    }

    /// Full phoneme specification with per-formant amplitudes.
    /// Format: (freq_hz, bandwidth_hz, amplitude) × 4.
    fn spec(self) -> PhonemeSpec {
        match self {
            // Vowels — F1/F2 are the defining formants. F3/F4 add presence.
            // Amplitudes: F1 loudest, F2 next, F3/F4 for air/brightness.
            Self::Ah => PhonemeSpec { formants: [(730.0, 90.0, 1.0),  (1090.0, 110.0, 0.7),  (2440.0, 200.0, 0.2), (3400.0, 300.0, 0.07)], voiced: 1.0, gain: 1.0 },
            Self::Ee => PhonemeSpec { formants: [(270.0, 60.0, 1.0),  (2290.0, 100.0, 0.5),  (3010.0, 200.0, 0.15),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 1.0 },
            Self::Ih => PhonemeSpec { formants: [(390.0, 60.0, 1.0),  (1990.0, 100.0, 0.5),  (2550.0, 200.0, 0.15),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.9 },
            Self::Eh => PhonemeSpec { formants: [(530.0, 70.0, 1.0),  (1840.0, 100.0, 0.6),  (2480.0, 200.0, 0.15),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.95 },
            Self::Ae => PhonemeSpec { formants: [(660.0, 80.0, 1.0),  (1720.0, 100.0, 0.6),  (2410.0, 200.0, 0.15),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.95 },
            Self::Uh => PhonemeSpec { formants: [(520.0, 70.0, 1.0),  (1190.0, 100.0, 0.6),  (2390.0, 200.0, 0.15),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.9 },
            Self::Oh => PhonemeSpec { formants: [(570.0, 70.0, 1.0),  (840.0, 100.0, 0.7),   (2410.0, 200.0, 0.12),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.95 },
            Self::Oo => PhonemeSpec { formants: [(300.0, 60.0, 1.0),  (870.0, 100.0, 0.5),   (2240.0, 200.0, 0.1), (3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.85 },
            Self::Aw => PhonemeSpec { formants: [(590.0, 70.0, 1.0),  (880.0, 100.0, 0.7),   (2540.0, 200.0, 0.12),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.95 },
            Self::Er => PhonemeSpec { formants: [(490.0, 70.0, 1.0),  (1350.0, 100.0, 0.5),  (1690.0, 200.0, 0.3), (3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.85 },
            // Nasals — low F1, muffled
            Self::Mm => PhonemeSpec { formants: [(200.0, 80.0, 1.0),  (1000.0, 200.0, 0.15), (2200.0, 300.0, 0.05),(3400.0, 400.0, 0.02)], voiced: 1.0, gain: 0.5 },
            Self::Nn => PhonemeSpec { formants: [(200.0, 80.0, 1.0),  (1400.0, 200.0, 0.2),  (2200.0, 300.0, 0.05),(3400.0, 400.0, 0.02)], voiced: 1.0, gain: 0.5 },
            // Liquids — gentle formants
            Self::Ll => PhonemeSpec { formants: [(350.0, 80.0, 1.0),  (1000.0, 150.0, 0.4),  (2400.0, 200.0, 0.1), (3400.0, 300.0, 0.03)], voiced: 1.0, gain: 0.6 },
            Self::Rr => PhonemeSpec { formants: [(350.0, 80.0, 1.0),  (1060.0, 150.0, 0.35), (1380.0, 200.0, 0.25),(3400.0, 300.0, 0.03)], voiced: 1.0, gain: 0.6 },
            // Fricatives — noise-excited, high-frequency energy
            Self::Ss => PhonemeSpec { formants: [(4000.0, 500.0, 0.6),(6000.0, 600.0, 0.8),  (8000.0, 800.0, 0.3), (10000.0, 1000.0, 0.1)], voiced: 0.0, gain: 0.15 },
            Self::Sh => PhonemeSpec { formants: [(2500.0, 400.0, 0.7),(4000.0, 500.0, 0.6),  (6000.0, 700.0, 0.3), (8000.0, 900.0, 0.1)],  voiced: 0.0, gain: 0.15 },
            Self::Ff => PhonemeSpec { formants: [(1500.0, 600.0, 0.3),(3500.0, 700.0, 0.4),  (5500.0, 800.0, 0.3), (7500.0, 1000.0, 0.1)], voiced: 0.0, gain: 0.12 },
            // Voiced fricatives
            Self::Zz => PhonemeSpec { formants: [(200.0, 80.0, 0.6),  (4000.0, 500.0, 0.3), (6000.0, 600.0, 0.4), (8000.0, 800.0, 0.15)], voiced: 0.5, gain: 0.15 },
            Self::Vv => PhonemeSpec { formants: [(200.0, 80.0, 0.7),  (1500.0, 600.0, 0.2), (3500.0, 700.0, 0.3), (5500.0, 800.0, 0.15)], voiced: 0.5, gain: 0.12 },
            // Stops — brief noise bursts at characteristic frequencies
            Self::Bb => PhonemeSpec { formants: [(200.0, 100.0, 0.8), (800.0, 200.0, 0.3),  (1200.0, 300.0, 0.15),(2500.0, 400.0, 0.05)], voiced: 0.6, gain: 0.35 },
            Self::Dd => PhonemeSpec { formants: [(200.0, 100.0, 0.6), (1600.0, 300.0, 0.5), (2600.0, 400.0, 0.3), (3500.0, 500.0, 0.1)],  voiced: 0.5, gain: 0.35 },
            Self::Gg => PhonemeSpec { formants: [(200.0, 100.0, 0.5), (1500.0, 200.0, 0.6), (2500.0, 300.0, 0.4), (3500.0, 400.0, 0.1)],  voiced: 0.4, gain: 0.35 },
            Self::Kk => PhonemeSpec { formants: [(800.0, 300.0, 0.3), (1500.0, 300.0, 0.5), (2500.0, 400.0, 0.5), (3500.0, 500.0, 0.15)], voiced: 0.0, gain: 0.25 },
            // Aspirate — breathy noise shaped by wide-band formants
            Self::Hh => PhonemeSpec { formants: [(500.0, 400.0, 0.3), (1500.0, 500.0, 0.3), (2500.0, 600.0, 0.25),(3500.0, 700.0, 0.15)], voiced: 0.0, gain: 0.12 },
            // Unvoiced alveolar stop
            Self::Tt => PhonemeSpec { formants: [(300.0, 200.0, 0.3), (3000.0, 400.0, 0.6), (4500.0, 500.0, 0.4), (6000.0, 600.0, 0.15)], voiced: 0.0, gain: 0.25 },
            // Diphthongs — midpoint formants between component vowels
            Self::Ay => PhonemeSpec { formants: [(560.0, 80.0, 1.0),  (1480.0, 110.0, 0.6),  (2500.0, 200.0, 0.15),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.95 },
            Self::Ow => PhonemeSpec { formants: [(630.0, 80.0, 1.0),  (980.0, 110.0, 0.65),  (2400.0, 200.0, 0.12),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.9 },
            Self::Ey => PhonemeSpec { formants: [(400.0, 70.0, 1.0),  (2060.0, 100.0, 0.55), (2750.0, 200.0, 0.15),(3400.0, 300.0, 0.05)], voiced: 1.0, gain: 0.95 },
            // Unvoiced bilabial stop — low-freq burst
            Self::Pp => PhonemeSpec { formants: [(400.0, 200.0, 0.4), (800.0, 300.0, 0.3),  (1200.0, 400.0, 0.15),(2500.0, 500.0, 0.05)], voiced: 0.0, gain: 0.25 },
            // Semivowels — like vowels but quieter, meant as transitions
            Self::Ww => PhonemeSpec { formants: [(300.0, 60.0, 0.8),  (600.0, 100.0, 0.4),   (2400.0, 200.0, 0.08),(3400.0, 300.0, 0.03)], voiced: 1.0, gain: 0.5 },
            Self::Yy => PhonemeSpec { formants: [(260.0, 60.0, 0.8),  (2200.0, 100.0, 0.4),  (3000.0, 200.0, 0.12),(3400.0, 300.0, 0.04)], voiced: 1.0, gain: 0.5 },
            // Velar nasal — like NN but further back
            Self::Ng => PhonemeSpec { formants: [(200.0, 80.0, 1.0),  (1100.0, 200.0, 0.15), (2100.0, 300.0, 0.05),(3400.0, 400.0, 0.02)], voiced: 1.0, gain: 0.45 },
            // Affricate — like TT+SH combined
            Self::Ch => PhonemeSpec { formants: [(300.0, 200.0, 0.2), (2500.0, 400.0, 0.5),  (4000.0, 500.0, 0.5), (6000.0, 700.0, 0.2)],  voiced: 0.0, gain: 0.18 },
            // Dental fricatives — broad gentle noise
            Self::Th => PhonemeSpec { formants: [(1400.0, 600.0, 0.2),(3500.0, 700.0, 0.3),  (6000.0, 800.0, 0.25),(8000.0, 1000.0, 0.1)], voiced: 0.0, gain: 0.10 },
            Self::Dh => PhonemeSpec { formants: [(200.0, 80.0, 0.5),  (1400.0, 600.0, 0.15),(3500.0, 700.0, 0.2), (6000.0, 800.0, 0.1)],  voiced: 0.6, gain: 0.12 },
            // Silence
            Self::Sil => PhonemeSpec { formants: [(200.0, 100.0, 0.0),(1000.0, 200.0, 0.0), (2000.0, 300.0, 0.0), (3000.0, 400.0, 0.0)],  voiced: 1.0, gain: 0.0 },
        }
    }
}

/// Second-order resonator (bandpass) for formant synthesis.
#[derive(Debug, Clone, Copy)]
struct Resonator {
    y1: f32,
    y2: f32,
    a1: f32,
    a2: f32,
    peak_gain: f32,
}

impl Default for Resonator {
    fn default() -> Self {
        Self { y1: 0.0, y2: 0.0, a1: 0.0, a2: 0.0, peak_gain: 1.0 }
    }
}

impl Resonator {
    /// Recompute coefficients for given formant frequency and bandwidth.
    fn set_freq_bw(&mut self, freq: f32, bw: f32, sample_rate: f32) {
        let omega = TAU * freq / sample_rate;
        let r = (-std::f32::consts::PI * bw / sample_rate).exp();
        self.a1 = 2.0 * r * omega.cos();
        self.a2 = -(r * r);
        // Normalize peak gain so the resonator has unity gain at its center
        // frequency. Without this, narrow-bandwidth resonators produce
        // huge peaks that dominate the mix.
        self.peak_gain = (1.0 - r * r).max(0.001);
    }

    #[inline]
    fn tick(&mut self, input: f32) -> f32 {
        let y = input * self.peak_gain + self.a1 * self.y1 + self.a2 * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        // Prevent blowup.
        if y.abs() > 10.0 {
            self.y1 *= 0.3;
            self.y2 *= 0.3;
        }
        y
    }

    fn reset(&mut self) {
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// Parallel formant speech synthesizer.
///
/// Excitation is fed into 4 resonators **in parallel**, each with its own
/// amplitude. The outputs are summed. This gives direct control over the
/// spectral envelope — F1 and F2 define the vowel, F3/F4 add air.
#[derive(Debug, Clone)]
pub struct LpcSynth {
    formants: [Resonator; NUM_FORMANTS],
    // Interpolated formant state: (freq, bw, amp).
    cur: [(f32, f32, f32); NUM_FORMANTS],
    tgt: [(f32, f32, f32); NUM_FORMANTS],
    cur_gain: f32,
    tgt_gain: f32,
    cur_voiced: f32,
    tgt_voiced: f32,
    interp_t: f32,
    // Excitation.
    pitch_phase: f32,
    glottal_state: f32, // single-pole filter state for glottal shaping
    lfsr: u32,
    sample_rate: f32,
    current_phoneme: usize,
}

impl Default for LpcSynth {
    fn default() -> Self {
        let sp = Phoneme::Ah.spec();
        let fb: [(f32, f32, f32); NUM_FORMANTS] = sp.formants;
        Self {
            formants: [Resonator::default(); NUM_FORMANTS],
            cur: fb,
            tgt: fb,
            cur_gain: sp.gain,
            tgt_gain: sp.gain,
            cur_voiced: sp.voiced,
            tgt_voiced: sp.voiced,
            interp_t: 1.0,
            pitch_phase: 0.0,
            glottal_state: 0.0,
            lfsr: 0xACE1,
            sample_rate: 44100.0,
            current_phoneme: 0,
        }
    }
}

impl LpcSynth {
    pub fn reset(&mut self) {
        for f in &mut self.formants {
            f.reset();
        }
        self.pitch_phase = 0.0;
        self.glottal_state = 0.0;
        self.lfsr = 0xACE1;
        self.interp_t = 1.0;
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
    }

    /// Set the target phoneme. Smoothly interpolates from the current state.
    pub fn set_phoneme(&mut self, idx: usize) {
        let idx = idx.min(NUM_PHONEMES - 1);
        if idx != self.current_phoneme || self.interp_t >= 1.0 {
            // Snapshot current interpolated values.
            let t = self.interp_t.clamp(0.0, 1.0);
            for i in 0..NUM_FORMANTS {
                self.cur[i].0 += t * (self.tgt[i].0 - self.cur[i].0);
                self.cur[i].1 += t * (self.tgt[i].1 - self.cur[i].1);
                self.cur[i].2 += t * (self.tgt[i].2 - self.cur[i].2);
            }
            self.cur_gain += t * (self.tgt_gain - self.cur_gain);
            self.cur_voiced += t * (self.tgt_voiced - self.cur_voiced);

            let sp = Phoneme::from_index(idx).spec();
            self.tgt = sp.formants;
            self.tgt_gain = sp.gain;
            self.tgt_voiced = sp.voiced;
            self.interp_t = 0.0;
            self.current_phoneme = idx;
        }
    }

    /// Render one sample.
    /// `pitch_hz` — fundamental frequency from MIDI.
    /// `buzz` — 0.0 = use phoneme voicing, 1.0 = force noise.
    #[inline]
    pub fn tick(&mut self, pitch_hz: f32, buzz: f32) -> f32 {
        // Advance interpolation (~30ms).
        let interp_rate = 1.0 / (0.03 * self.sample_rate);
        self.interp_t = (self.interp_t + interp_rate).min(1.0);
        let t = self.interp_t;

        // Update resonator coefficients.
        let mut amps = [0.0_f32; NUM_FORMANTS];
        for i in 0..NUM_FORMANTS {
            let freq = (self.cur[i].0 + t * (self.tgt[i].0 - self.cur[i].0))
                .min(self.sample_rate * 0.45);
            let bw = self.cur[i].1 + t * (self.tgt[i].1 - self.cur[i].1);
            amps[i] = self.cur[i].2 + t * (self.tgt[i].2 - self.cur[i].2);
            self.formants[i].set_freq_bw(freq, bw, self.sample_rate);
        }

        let gain = self.cur_gain + t * (self.tgt_gain - self.cur_gain);
        let phoneme_voiced = self.cur_voiced + t * (self.tgt_voiced - self.cur_voiced);
        let voiced_mix = phoneme_voiced * (1.0 - buzz);

        // --- Glottal excitation ---
        // Rosenberg-style glottal pulse: quick rise, slower fall.
        let pitch_inc = (pitch_hz / self.sample_rate).clamp(0.0001, 0.5);
        self.pitch_phase += pitch_inc;
        if self.pitch_phase >= 1.0 {
            self.pitch_phase -= 1.0;
        }
        // Open phase = 60% of the period, closed phase = 40%.
        let glottal_raw = if self.pitch_phase < 0.1 {
            // Rising edge.
            self.pitch_phase / 0.1
        } else if self.pitch_phase < 0.6 {
            // Falling edge.
            1.0 - (self.pitch_phase - 0.1) / 0.5
        } else {
            0.0
        };
        // Differentiate to get the glottal flow derivative (what the vocal
        // tract actually "sees"). Use a simple first-difference.
        let glottal_excitation = glottal_raw - self.glottal_state;
        self.glottal_state = glottal_raw;

        // Unvoiced: white noise.
        let bit = ((self.lfsr ^ (self.lfsr >> 1)) & 1) as u32;
        self.lfsr = (self.lfsr >> 1) | (bit << 14);
        let noise = if (self.lfsr & 1) == 0 { 1.0 } else { -1.0 };

        let excitation = voiced_mix * glottal_excitation * 4.0
            + (1.0 - voiced_mix) * noise * 0.12;

        // --- Parallel formant filtering ---
        // Each resonator gets the same excitation; outputs are amplitude-
        // weighted and summed. This preserves F1/F2 distinction.
        let mut signal = 0.0_f32;
        for i in 0..NUM_FORMANTS {
            signal += amps[i] * self.formants[i].tick(excitation);
        }

        (signal * gain).clamp(-2.0, 2.0)
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
