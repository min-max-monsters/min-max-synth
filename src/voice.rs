//! A single polyphonic voice combining oscillator, ADSR, vibrato, sweep and
//! optional sample playback. Created once and recycled by the voice pool.

use crate::dsp::{
    midi_to_hz, Adsr, FmOsc, Lfo, NoiseOsc, PulseOsc, SawOsc, Sweep, TriangleOsc,
    Waveform, WaveOsc,
};
use crate::samples::{DrumKind, SamplePlayer};

/// Snapshot of synth parameters seen by the voice for a render block.
/// The voice never touches `nih-plug`'s `Params` directly.
#[derive(Debug, Clone, Copy)]
pub struct VoiceParams {
    pub waveform: Waveform,
    pub pulse_duty: f32,
    pub noise_short: bool,
    pub fm_ratio: f32,
    pub fm_index: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub vibrato_rate: f32,
    pub vibrato_depth_semi: f32,
    pub vibrato_delay: f32,
    pub sweep_semi: f32,
    pub sweep_time: f32,
    pub mono: bool,
    pub arp_rate: f32,
    pub bit_depth: f32,
    pub bit_rate_hz: f32,
    pub fine_tune_cents: f32,
    pub octave_shift: i32,
    pub drum_mode: bool,
    pub drum_pitch: bool,
}

#[derive(Debug, Clone)]
pub struct Voice {
    sample_rate: f32,

    // State
    note: Option<u8>,
    velocity: f32,
    age: u64, // for voice stealing
    elapsed_samples: u64,

    // Oscillators (only one active at a time but kept around).
    pulse: PulseOsc,
    triangle: TriangleOsc,
    wave: WaveOsc,
    noise: NoiseOsc,
    fm: FmOsc,
    saw: SawOsc,

    // Modulators
    env: Adsr,
    vibrato: Lfo,
    sweep: Sweep,

    // Sample playback (drum mode)
    sample: SamplePlayer,
    is_drum: bool,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            note: None,
            velocity: 0.0,
            age: 0,
            elapsed_samples: 0,
            pulse: PulseOsc::default(),
            triangle: TriangleOsc::default(),
            wave: WaveOsc::default(),
            noise: NoiseOsc::default(),
            fm: FmOsc::default(),
            saw: SawOsc::default(),
            env: Adsr::new(sample_rate),
            vibrato: Lfo::default(),
            sweep: Sweep::default(),
            sample: SamplePlayer::default(),
            is_drum: false,
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
        self.env.set_sample_rate(sr);
    }

    pub fn is_active(&self) -> bool {
        self.env.is_active() || self.sample.is_active()
    }

    pub fn note(&self) -> Option<u8> {
        self.note
    }

    pub fn age(&self) -> u64 {
        self.age
    }

    pub fn note_on(&mut self, note: u8, velocity: f32, params: &VoiceParams, age: u64) {
        self.note = Some(note);
        self.velocity = velocity.clamp(0.0, 1.0);
        self.age = age;
        self.elapsed_samples = 0;
        self.pulse.reset();
        self.triangle.reset();
        self.wave.reset();
        self.noise.reset();
        self.fm.reset();
        self.saw.reset();
        self.vibrato.reset();
        self.sweep.reset();

        if params.drum_mode {
            self.is_drum = true;
            // Map note to a drum kind: lowest 8 white-ish keys above C2 (36).
            let idx = ((note as i32 - 36).rem_euclid(DrumKind::ALL.len() as i32)) as usize;
            let kind = DrumKind::ALL[idx];
            let rate = if params.drum_pitch {
                // ±2 octaves around the mapped key (centred on C3 = 48).
                let semis = note as f32 - 48.0;
                (2.0_f32).powf(semis / 12.0)
            } else {
                1.0
            };
            self.sample.trigger(kind, rate);
            // Use a tiny percussive envelope so velocity scales sample.
            self.env.attack = 0.0;
            self.env.decay = 0.0;
            self.env.sustain = 1.0;
            self.env.release = 0.001;
            self.env.note_on();
        } else {
            self.is_drum = false;
            self.env.attack = params.attack;
            self.env.decay = params.decay;
            self.env.sustain = params.sustain;
            self.env.release = params.release;
            self.env.note_on();
        }
    }

    pub fn note_off(&mut self) {
        // For drum samples the env is already very short; just let it ring.
        if !self.is_drum {
            self.env.note_off();
        } else {
            self.env.note_off();
        }
    }

    /// Change the playing pitch without retriggering the envelope or
    /// oscillator phases. Used for legato (mono mode) and arpeggio.
    pub fn set_note(&mut self, note: u8) {
        if self.note.is_some() {
            self.note = Some(note);
        }
    }

    /// Is this voice currently sounding a non-drum note?
    pub fn is_pitched_active(&self) -> bool {
        self.note.is_some() && !self.is_drum && self.env.is_active()
    }

    /// Render one sample. The bitcrusher is applied to the bus, not per-voice,
    /// so this returns a clean per-voice signal.
    #[inline]
    pub fn tick(&mut self, params: &VoiceParams) -> f32 {
        let Some(note) = self.note else { return 0.0 };
        let env = self.env.tick();
        if !self.env.is_active() && !self.sample.is_active() {
            self.note = None;
            return 0.0;
        }

        if self.is_drum {
            return self.sample.tick(self.sample_rate) * self.velocity;
        }

        // Pitch chain: base note + octave + fine + vibrato + sweep.
        // Vibrato fades in smoothly over ~80 ms after the configured delay.
        let elapsed_s = self.elapsed_samples as f32 / self.sample_rate;
        self.elapsed_samples = self.elapsed_samples.saturating_add(1);
        let vib_gain = if params.vibrato_depth_semi <= 0.0 {
            0.0
        } else {
            ((elapsed_s - params.vibrato_delay) / 0.08).clamp(0.0, 1.0)
        };
        let vib = self.vibrato.tick(params.vibrato_rate, self.sample_rate)
            * params.vibrato_depth_semi
            * vib_gain;
        let sweep_offset = self.sweep.tick(self.sample_rate, params.sweep_semi, params.sweep_time);
        let n = note as f32
            + params.octave_shift as f32 * 12.0
            + params.fine_tune_cents / 100.0
            + vib
            + sweep_offset;
        let freq = midi_to_hz(n);

        let raw = match params.waveform {
            Waveform::Pulse => self.pulse.tick(freq, self.sample_rate, params.pulse_duty),
            Waveform::Triangle => self.triangle.tick(freq, self.sample_rate),
            Waveform::Wave4Bit => self.wave.tick(freq, self.sample_rate),
            Waveform::Noise => self.noise.tick(freq, self.sample_rate, params.noise_short),
            Waveform::Fm => self.fm.tick(freq, self.sample_rate, params.fm_ratio, params.fm_index),
            Waveform::Saw => self.saw.tick(freq, self.sample_rate),
        };

        raw * env * self.velocity
    }
}
