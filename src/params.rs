//! NIH-plug `Params` definitions for the synth.

use crate::dsp::Waveform;
use crate::voice::VoiceParams;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

/// Waveform variants exposed as a discrete enum parameter.
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveChoice {
    #[id = "pulse"]
    #[name = "Pulse"]
    Pulse,
    #[id = "tri4"]
    #[name = "Triangle 4-bit"]
    Triangle,
    #[id = "wav4"]
    #[name = "Wave 4-bit"]
    Wave,
    #[id = "noise"]
    #[name = "Noise"]
    Noise,
    #[id = "fm"]
    #[name = "FM 2-op"]
    Fm,
    #[id = "saw"]
    #[name = "Saw"]
    Saw,
}

impl WaveChoice {
    pub fn to_dsp(self) -> Waveform {
        match self {
            WaveChoice::Pulse => Waveform::Pulse,
            WaveChoice::Triangle => Waveform::Triangle,
            WaveChoice::Wave => Waveform::Wave4Bit,
            WaveChoice::Noise => Waveform::Noise,
            WaveChoice::Fm => Waveform::Fm,
            WaveChoice::Saw => Waveform::Saw,
        }
    }
}

/// Mono note-transition behaviour. Only meaningful when `mono` is on.
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegatoMode {
    /// Each new note retriggers the envelope and oscillator phases (no legato).
    #[id = "retrig"]
    #[name = "Retrigger"]
    Retrigger,
    /// New notes change pitch instantly without retriggering the envelope.
    #[id = "legato"]
    #[name = "Legato"]
    Legato,
    /// Like Legato, but the pitch slides smoothly to the new note.
    #[id = "glide"]
    #[name = "Glide"]
    Glide,
}

#[derive(Params)]
pub struct SynthParams {
    /// Persisted GUI window size.
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "wave"]
    pub waveform: EnumParam<WaveChoice>,

    #[id = "duty"]
    pub pulse_duty: FloatParam,

    #[id = "noise_s"]
    pub noise_short: BoolParam,

    #[id = "fm_rat"]
    pub fm_ratio: FloatParam,
    #[id = "fm_idx"]
    pub fm_index: FloatParam,

    #[id = "atk"]
    pub attack: FloatParam,
    #[id = "dec"]
    pub decay: FloatParam,
    #[id = "sus"]
    pub sustain: FloatParam,
    #[id = "rel"]
    pub release: FloatParam,

    #[id = "duty_lfo_rt"]
    pub duty_lfo_rate: FloatParam,
    #[id = "duty_lfo_dp"]
    pub duty_lfo_depth: FloatParam,

    #[id = "vib_rt"]
    pub vibrato_rate: FloatParam,
    #[id = "vib_dp"]
    pub vibrato_depth: FloatParam,
    #[id = "vib_dl"]
    pub vibrato_delay: FloatParam,

    #[id = "swp_st"]
    pub sweep_semi: FloatParam,
    #[id = "swp_tm"]
    pub sweep_time: FloatParam,

    #[id = "mono"]
    pub mono: BoolParam,
    #[id = "arp_rt"]
    pub arp_rate: FloatParam,
    #[id = "legato"]
    pub legato_mode: EnumParam<LegatoMode>,
    #[id = "glide_t"]
    pub glide_time: FloatParam,

    #[id = "bit_dp"]
    pub bit_depth: FloatParam,
    #[id = "bit_rt"]
    pub bit_rate: FloatParam,
    #[id = "lp_hz"]
    pub lp_cutoff: FloatParam,
    #[id = "hp_hz"]
    pub hp_cutoff: FloatParam,

    #[id = "tune"]
    pub fine_tune: FloatParam,
    #[id = "octv"]
    pub octave: IntParam,

    #[id = "drum_o"]
    pub drum_mode: BoolParam,
    #[id = "drum_p"]
    pub drum_pitch: BoolParam,

    #[id = "speech"]
    pub speech_mode: BoolParam,
    #[id = "phon"]
    pub phoneme: IntParam,
    #[id = "buzz"]
    pub speech_buzz: FloatParam,

    // Speech sequencer: 8 phoneme slots + timing.
    #[id = "sq_len"]
    pub speech_seq_len: IntParam,
    #[id = "sq_ms"]
    pub speech_step_ms: FloatParam,
    #[id = "sq_lp"]
    pub speech_seq_loop: BoolParam,
    #[id = "sq0"] pub sq0: IntParam,
    #[id = "sq1"] pub sq1: IntParam,
    #[id = "sq2"] pub sq2: IntParam,
    #[id = "sq3"] pub sq3: IntParam,
    #[id = "sq4"] pub sq4: IntParam,
    #[id = "sq5"] pub sq5: IntParam,
    #[id = "sq6"] pub sq6: IntParam,
    #[id = "sq7"] pub sq7: IntParam,

    // Per-drum: 9 params each (wave/freq/ratio/noise/pitch_env/pitch_time/decay/burst/level).
    #[id="d0w"] pub d0_wave: IntParam,
    #[id="d0f"] pub d0_freq: FloatParam,
    #[id="d0r"] pub d0_ratio: FloatParam,
    #[id="d0n"] pub d0_noise: FloatParam,
    #[id="d0pe"] pub d0_pitch_env: FloatParam,
    #[id="d0pt"] pub d0_pitch_time: FloatParam,
    #[id="d0d"] pub d0_decay: FloatParam,
    #[id="d0b"] pub d0_burst: FloatParam,
    #[id="d0l"] pub d0_level: FloatParam,

    #[id="d1w"] pub d1_wave: IntParam,
    #[id="d1f"] pub d1_freq: FloatParam,
    #[id="d1r"] pub d1_ratio: FloatParam,
    #[id="d1n"] pub d1_noise: FloatParam,
    #[id="d1pe"] pub d1_pitch_env: FloatParam,
    #[id="d1pt"] pub d1_pitch_time: FloatParam,
    #[id="d1d"] pub d1_decay: FloatParam,
    #[id="d1b"] pub d1_burst: FloatParam,
    #[id="d1l"] pub d1_level: FloatParam,

    #[id="d2w"] pub d2_wave: IntParam,
    #[id="d2f"] pub d2_freq: FloatParam,
    #[id="d2r"] pub d2_ratio: FloatParam,
    #[id="d2n"] pub d2_noise: FloatParam,
    #[id="d2pe"] pub d2_pitch_env: FloatParam,
    #[id="d2pt"] pub d2_pitch_time: FloatParam,
    #[id="d2d"] pub d2_decay: FloatParam,
    #[id="d2b"] pub d2_burst: FloatParam,
    #[id="d2l"] pub d2_level: FloatParam,

    #[id="d3w"] pub d3_wave: IntParam,
    #[id="d3f"] pub d3_freq: FloatParam,
    #[id="d3r"] pub d3_ratio: FloatParam,
    #[id="d3n"] pub d3_noise: FloatParam,
    #[id="d3pe"] pub d3_pitch_env: FloatParam,
    #[id="d3pt"] pub d3_pitch_time: FloatParam,
    #[id="d3d"] pub d3_decay: FloatParam,
    #[id="d3b"] pub d3_burst: FloatParam,
    #[id="d3l"] pub d3_level: FloatParam,

    #[id="d4w"] pub d4_wave: IntParam,
    #[id="d4f"] pub d4_freq: FloatParam,
    #[id="d4r"] pub d4_ratio: FloatParam,
    #[id="d4n"] pub d4_noise: FloatParam,
    #[id="d4pe"] pub d4_pitch_env: FloatParam,
    #[id="d4pt"] pub d4_pitch_time: FloatParam,
    #[id="d4d"] pub d4_decay: FloatParam,
    #[id="d4b"] pub d4_burst: FloatParam,
    #[id="d4l"] pub d4_level: FloatParam,

    #[id="d5w"] pub d5_wave: IntParam,
    #[id="d5f"] pub d5_freq: FloatParam,
    #[id="d5r"] pub d5_ratio: FloatParam,
    #[id="d5n"] pub d5_noise: FloatParam,
    #[id="d5pe"] pub d5_pitch_env: FloatParam,
    #[id="d5pt"] pub d5_pitch_time: FloatParam,
    #[id="d5d"] pub d5_decay: FloatParam,
    #[id="d5b"] pub d5_burst: FloatParam,
    #[id="d5l"] pub d5_level: FloatParam,

    #[id="d6w"] pub d6_wave: IntParam,
    #[id="d6f"] pub d6_freq: FloatParam,
    #[id="d6r"] pub d6_ratio: FloatParam,
    #[id="d6n"] pub d6_noise: FloatParam,
    #[id="d6pe"] pub d6_pitch_env: FloatParam,
    #[id="d6pt"] pub d6_pitch_time: FloatParam,
    #[id="d6d"] pub d6_decay: FloatParam,
    #[id="d6b"] pub d6_burst: FloatParam,
    #[id="d6l"] pub d6_level: FloatParam,

    #[id="d7w"] pub d7_wave: IntParam,
    #[id="d7f"] pub d7_freq: FloatParam,
    #[id="d7r"] pub d7_ratio: FloatParam,
    #[id="d7n"] pub d7_noise: FloatParam,
    #[id="d7pe"] pub d7_pitch_env: FloatParam,
    #[id="d7pt"] pub d7_pitch_time: FloatParam,
    #[id="d7d"] pub d7_decay: FloatParam,
    #[id="d7b"] pub d7_burst: FloatParam,
    #[id="d7l"] pub d7_level: FloatParam,
}

impl Default for SynthParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(1280, 880),

            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(-9.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-60.0),
                    max: util::db_to_gain(0.0),
                    factor: FloatRange::gain_skew_factor(-60.0, 0.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(20.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            waveform: EnumParam::new("Waveform", WaveChoice::Pulse),

            pulse_duty: FloatParam::new("Pulse Duty", 0.5, FloatRange::Linear { min: 0.05, max: 0.95 })
                .with_step_size(0.01)
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            noise_short: BoolParam::new("Noise Short Period", false),

            fm_ratio: FloatParam::new("FM Ratio", 2.0, FloatRange::Linear { min: 0.25, max: 8.0 })
                .with_step_size(0.01),
            fm_index: FloatParam::new("FM Index", 1.5, FloatRange::Linear { min: 0.0, max: 10.0 })
                .with_step_size(0.01),

            attack: ms("Attack", 5.0, 0.0, 2000.0),
            decay: ms("Decay", 100.0, 0.0, 4000.0),
            sustain: FloatParam::new("Sustain", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            release: ms("Release", 150.0, 0.0, 4000.0),

            duty_lfo_rate: FloatParam::new("Duty LFO Rate", 4.0, FloatRange::Linear { min: 0.05, max: 20.0 })
                .with_unit(" Hz").with_step_size(0.05),
            duty_lfo_depth: FloatParam::new("Duty LFO Depth", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01)
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            vibrato_rate: FloatParam::new("Vib Rate", 5.0, FloatRange::Linear { min: 0.1, max: 20.0 })
                .with_unit(" Hz").with_step_size(0.1),
            vibrato_depth: FloatParam::new("Vib Depth", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" semis").with_step_size(0.01),
            vibrato_delay: ms("Vib Delay", 0.0, 0.0, 2000.0),

            sweep_semi: FloatParam::new("Auto Bend", 0.0, FloatRange::Linear { min: -36.0, max: 36.0 })
                .with_unit(" semis").with_step_size(0.1),
            sweep_time: ms("Bend Time", 0.0, 0.0, 2000.0),

            mono: BoolParam::new("Monophonic", false),
            arp_rate: FloatParam::new(
                "Arp Rate",
                0.0,
                FloatRange::Linear { min: 0.0, max: 32.0 },
            )
            .with_unit(" Hz")
            .with_step_size(0.5),

            legato_mode: EnumParam::new("Legato", LegatoMode::Retrigger),
            glide_time: FloatParam::new(
                "Glide Time",
                60.0,
                FloatRange::Skewed { min: 1.0, max: 2000.0, factor: FloatRange::skew_factor(-2.0) },
            )
            .with_unit(" ms")
            .with_step_size(1.0),

            bit_depth: FloatParam::new("Bit Depth", 16.0, FloatRange::Linear { min: 1.0, max: 16.0 })
                .with_unit(" bits").with_step_size(0.5),
            bit_rate: FloatParam::new(
                "Bit Rate",
                44_100.0,
                FloatRange::Skewed { min: 1_000.0, max: 96_000.0, factor: FloatRange::skew_factor(-1.0) },
            )
            .with_unit(" Hz").with_step_size(100.0),

            // Output-stage filters (6 dB/oct, one-pole RC).
            // LP at max (20 kHz) = effectively off; HP at min (0) = off.
            lp_cutoff: FloatParam::new(
                "LP Cutoff",
                20_000.0,
                FloatRange::Skewed { min: 1_000.0, max: 20_000.0, factor: FloatRange::skew_factor(-1.0) },
            )
            .with_unit(" Hz")
            .with_step_size(500.0),

            hp_cutoff: FloatParam::new(
                "HP Cutoff",
                0.0,
                FloatRange::Linear { min: 0.0, max: 500.0 },
            )
            .with_unit(" Hz")
            .with_step_size(1.0),

            fine_tune: FloatParam::new("Fine Tune", 0.0, FloatRange::Linear { min: -100.0, max: 100.0 })
                .with_unit(" cents").with_step_size(1.0),
            octave: IntParam::new("Octave", 0, IntRange::Linear { min: -3, max: 3 }),

            drum_mode: BoolParam::new("Drum Mode", false),
            drum_pitch: BoolParam::new("Drum Pitch Tracks Note", true),

            speech_mode: BoolParam::new("Speech Mode", false),
            phoneme: IntParam::new("Phoneme", 0, IntRange::Linear { min: 0, max: 35 }),
            speech_buzz: FloatParam::new("Buzz", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),

            speech_seq_len: IntParam::new("Seq Length", 0, IntRange::Linear { min: 0, max: 8 }),
            speech_step_ms: FloatParam::new(
                "Step Time",
                120.0,
                FloatRange::Skewed { min: 30.0, max: 500.0, factor: FloatRange::skew_factor(-1.0) },
            )
            .with_unit(" ms")
            .with_step_size(1.0),
            speech_seq_loop: BoolParam::new("Seq Loop", false),
            sq0: IntParam::new("Seq 1", 0, IntRange::Linear { min: 0, max: 35 }),
            sq1: IntParam::new("Seq 2", 1, IntRange::Linear { min: 0, max: 35 }),
            sq2: IntParam::new("Seq 3", 6, IntRange::Linear { min: 0, max: 35 }),
            sq3: IntParam::new("Seq 4", 7, IntRange::Linear { min: 0, max: 35 }),
            sq4: IntParam::new("Seq 5", 0, IntRange::Linear { min: 0, max: 35 }),
            sq5: IntParam::new("Seq 6", 0, IntRange::Linear { min: 0, max: 35 }),
            sq6: IntParam::new("Seq 7", 0, IntRange::Linear { min: 0, max: 35 }),
            sq7: IntParam::new("Seq 8", 0, IntRange::Linear { min: 0, max: 35 }),

            // Defaults are tuned to recreate the original 8 embedded samples.
            // (See src/samples.rs for the originals; numbers below are derived
            // from the same DSP recipes.)
            // Kick: sine, 45 Hz target, +17 semis pitch decay over 40 ms.
            d0_wave: d_wave("Kick", 1),
            d0_freq: d_freq("Kick", 45.0),
            d0_ratio: d_ratio("Kick", 0.0),
            d0_noise: d_noise("Kick", 0.0),
            d0_pitch_env: d_pitch_env("Kick", 17.0),
            d0_pitch_time: d_pitch_time("Kick", 40.0),
            d0_decay: d_decay("Kick", 250.0),
            d0_burst: d_burst("Kick", 0.0),
            d0_level: d_level("Kick", 1.0),

            // Snare: 200 Hz sine + noise.
            d1_wave: d_wave("Snare", 1),
            d1_freq: d_freq("Snare", 200.0),
            d1_ratio: d_ratio("Snare", 0.0),
            d1_noise: d_noise("Snare", 0.7),
            d1_pitch_env: d_pitch_env("Snare", 0.0),
            d1_pitch_time: d_pitch_time("Snare", 1.0),
            d1_decay: d_decay("Snare", 120.0),
            d1_burst: d_burst("Snare", 0.0),
            d1_level: d_level("Snare", 1.0),

            // Hat closed: noise only, very short decay.
            d2_wave: d_wave("Hat Cl", 0),
            d2_freq: d_freq("Hat Cl", 100.0),
            d2_ratio: d_ratio("Hat Cl", 0.0),
            d2_noise: d_noise("Hat Cl", 1.0),
            d2_pitch_env: d_pitch_env("Hat Cl", 0.0),
            d2_pitch_time: d_pitch_time("Hat Cl", 1.0),
            d2_decay: d_decay("Hat Cl", 30.0),
            d2_burst: d_burst("Hat Cl", 0.0),
            d2_level: d_level("Hat Cl", 0.7),

            // Hat open: noise only, long decay.
            d3_wave: d_wave("Hat Op", 0),
            d3_freq: d_freq("Hat Op", 100.0),
            d3_ratio: d_ratio("Hat Op", 0.0),
            d3_noise: d_noise("Hat Op", 1.0),
            d3_pitch_env: d_pitch_env("Hat Op", 0.0),
            d3_pitch_time: d_pitch_time("Hat Op", 1.0),
            d3_decay: d_decay("Hat Op", 250.0),
            d3_burst: d_burst("Hat Op", 0.0),
            d3_level: d_level("Hat Op", 0.7),

            // Tom: sine, 90 Hz target, +12 semis pitch decay over 80 ms.
            d4_wave: d_wave("Tom", 1),
            d4_freq: d_freq("Tom", 90.0),
            d4_ratio: d_ratio("Tom", 0.0),
            d4_noise: d_noise("Tom", 0.0),
            d4_pitch_env: d_pitch_env("Tom", 12.0),
            d4_pitch_time: d_pitch_time("Tom", 80.0),
            d4_decay: d_decay("Tom", 300.0),
            d4_burst: d_burst("Tom", 0.0),
            d4_level: d_level("Tom", 1.0),

            // Clap: noise + multi-attack burst.
            d5_wave: d_wave("Clap", 0),
            d5_freq: d_freq("Clap", 100.0),
            d5_ratio: d_ratio("Clap", 0.0),
            d5_noise: d_noise("Clap", 1.0),
            d5_pitch_env: d_pitch_env("Clap", 0.0),
            d5_pitch_time: d_pitch_time("Clap", 1.0),
            d5_decay: d_decay("Clap", 200.0),
            d5_burst: d_burst("Clap", 1.0),
            d5_level: d_level("Clap", 0.8),

            // Cowbell: square at 540 + 800/540 ratio second osc.
            d6_wave: d_wave("Cowbell", 3),
            d6_freq: d_freq("Cowbell", 540.0),
            d6_ratio: d_ratio("Cowbell", 800.0 / 540.0),
            d6_noise: d_noise("Cowbell", 0.0),
            d6_pitch_env: d_pitch_env("Cowbell", 0.0),
            d6_pitch_time: d_pitch_time("Cowbell", 1.0),
            d6_decay: d_decay("Cowbell", 150.0),
            d6_burst: d_burst("Cowbell", 0.0),
            d6_level: d_level("Cowbell", 0.7),

            // Zap: square sweep from very high down to 80 Hz.
            d7_wave: d_wave("Zap", 3),
            d7_freq: d_freq("Zap", 80.0),
            d7_ratio: d_ratio("Zap", 0.0),
            d7_noise: d_noise("Zap", 0.0),
            d7_pitch_env: d_pitch_env("Zap", 50.0),
            d7_pitch_time: d_pitch_time("Zap", 100.0),
            d7_decay: d_decay("Zap", 250.0),
            d7_burst: d_burst("Zap", 0.0),
            d7_level: d_level("Zap", 1.0),
        }
    }
}

fn d_wave(name: &str, default: i32) -> IntParam {
    IntParam::new(&format!("{name} Wave"), default, IntRange::Linear { min: 0, max: 3 })
}

fn d_freq(name: &str, default: f32) -> FloatParam {
    FloatParam::new(
        &format!("{name} Freq"),
        default,
        FloatRange::Skewed { min: 20.0, max: 4000.0, factor: FloatRange::skew_factor(-2.0) },
    )
    .with_unit(" Hz")
    .with_step_size(1.0)
}

fn d_ratio(name: &str, default: f32) -> FloatParam {
    FloatParam::new(
        &format!("{name} Ratio"),
        default,
        FloatRange::Linear { min: 0.0, max: 3.0 },
    )
    .with_step_size(0.01)
}

fn d_noise(name: &str, default: f32) -> FloatParam {
    FloatParam::new(
        &format!("{name} Noise"),
        default,
        FloatRange::Linear { min: 0.0, max: 1.0 },
    )
    .with_step_size(0.01)
}

fn d_pitch_env(name: &str, default: f32) -> FloatParam {
    FloatParam::new(
        &format!("{name} Pitch Env"),
        default,
        FloatRange::Linear { min: -60.0, max: 60.0 },
    )
    .with_unit(" semis")
    .with_step_size(0.5)
}

fn d_pitch_time(name: &str, default_ms: f32) -> FloatParam {
    FloatParam::new(
        &format!("{name} Pitch Time"),
        default_ms,
        FloatRange::Skewed { min: 1.0, max: 1000.0, factor: FloatRange::skew_factor(-1.0) },
    )
    .with_unit(" ms")
    .with_step_size(0.5)
}

fn d_decay(name: &str, default_ms: f32) -> FloatParam {
    FloatParam::new(
        &format!("{name} Decay"),
        default_ms,
        FloatRange::Skewed { min: 5.0, max: 2000.0, factor: FloatRange::skew_factor(-1.0) },
    )
    .with_unit(" ms")
    .with_step_size(0.5)
}

fn d_burst(name: &str, default: f32) -> FloatParam {
    FloatParam::new(
        &format!("{name} Burst"),
        default,
        FloatRange::Linear { min: 0.0, max: 1.0 },
    )
    .with_step_size(0.01)
}

fn d_level(name: &str, default: f32) -> FloatParam {
    FloatParam::new(
        &format!("{name} Level"),
        default,
        FloatRange::Linear { min: 0.0, max: 2.0 },
    )
    .with_step_size(0.01)
}

fn ms(name: &str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-1.0) },
    )
    .with_unit(" ms")
    .with_step_size(0.1)
}

impl SynthParams {
    /// Take a thread-safe snapshot for the audio thread.
    pub fn snapshot(&self) -> VoiceParams {
        VoiceParams {
            waveform: self.waveform.value().to_dsp(),
            pulse_duty: self.pulse_duty.value(),
            noise_short: self.noise_short.value(),
            fm_ratio: self.fm_ratio.value(),
            fm_index: self.fm_index.value(),
            attack: self.attack.value() / 1000.0,
            decay: self.decay.value() / 1000.0,
            sustain: self.sustain.value(),
            release: self.release.value() / 1000.0,
            duty_lfo_rate: self.duty_lfo_rate.value(),
            duty_lfo_depth: self.duty_lfo_depth.value(),
            vibrato_rate: self.vibrato_rate.value(),
            vibrato_depth_semi: self.vibrato_depth.value(),
            vibrato_delay: self.vibrato_delay.value() / 1000.0,
            sweep_semi: self.sweep_semi.value(),
            sweep_time: self.sweep_time.value() / 1000.0,
            mono: self.mono.value(),
            arp_rate: self.arp_rate.value(),
            legato_mode: self.legato_mode.value(),
            glide_time: self.glide_time.value() / 1000.0,
            bit_depth: self.bit_depth.value(),
            bit_rate_hz: self.bit_rate.value(),
            lp_cutoff: self.lp_cutoff.value(),
            hp_cutoff: self.hp_cutoff.value(),
            fine_tune_cents: self.fine_tune.value(),
            octave_shift: self.octave.value(),
            drum_mode: self.drum_mode.value(),
            drum_pitch: self.drum_pitch.value(),
            speech_mode: self.speech_mode.value(),
            phoneme: self.phoneme.value() as usize,
            speech_buzz: self.speech_buzz.value(),
            speech_seq: [
                self.sq0.value() as usize, self.sq1.value() as usize,
                self.sq2.value() as usize, self.sq3.value() as usize,
                self.sq4.value() as usize, self.sq5.value() as usize,
                self.sq6.value() as usize, self.sq7.value() as usize,
            ],
            speech_seq_len: self.speech_seq_len.value() as usize,
            speech_step_ms: self.speech_step_ms.value(),
            speech_seq_loop: self.speech_seq_loop.value(),
            drum_wave: [
                self.d0_wave.value(), self.d1_wave.value(),
                self.d2_wave.value(), self.d3_wave.value(),
                self.d4_wave.value(), self.d5_wave.value(),
                self.d6_wave.value(), self.d7_wave.value(),
            ],
            drum_freq: [
                self.d0_freq.value(), self.d1_freq.value(),
                self.d2_freq.value(), self.d3_freq.value(),
                self.d4_freq.value(), self.d5_freq.value(),
                self.d6_freq.value(), self.d7_freq.value(),
            ],
            drum_ratio: [
                self.d0_ratio.value(), self.d1_ratio.value(),
                self.d2_ratio.value(), self.d3_ratio.value(),
                self.d4_ratio.value(), self.d5_ratio.value(),
                self.d6_ratio.value(), self.d7_ratio.value(),
            ],
            drum_noise: [
                self.d0_noise.value(), self.d1_noise.value(),
                self.d2_noise.value(), self.d3_noise.value(),
                self.d4_noise.value(), self.d5_noise.value(),
                self.d6_noise.value(), self.d7_noise.value(),
            ],
            drum_pitch_env: [
                self.d0_pitch_env.value(), self.d1_pitch_env.value(),
                self.d2_pitch_env.value(), self.d3_pitch_env.value(),
                self.d4_pitch_env.value(), self.d5_pitch_env.value(),
                self.d6_pitch_env.value(), self.d7_pitch_env.value(),
            ],
            drum_pitch_time: [
                self.d0_pitch_time.value() / 1000.0, self.d1_pitch_time.value() / 1000.0,
                self.d2_pitch_time.value() / 1000.0, self.d3_pitch_time.value() / 1000.0,
                self.d4_pitch_time.value() / 1000.0, self.d5_pitch_time.value() / 1000.0,
                self.d6_pitch_time.value() / 1000.0, self.d7_pitch_time.value() / 1000.0,
            ],
            drum_decay: [
                self.d0_decay.value() / 1000.0, self.d1_decay.value() / 1000.0,
                self.d2_decay.value() / 1000.0, self.d3_decay.value() / 1000.0,
                self.d4_decay.value() / 1000.0, self.d5_decay.value() / 1000.0,
                self.d6_decay.value() / 1000.0, self.d7_decay.value() / 1000.0,
            ],
            drum_burst: [
                self.d0_burst.value(), self.d1_burst.value(),
                self.d2_burst.value(), self.d3_burst.value(),
                self.d4_burst.value(), self.d5_burst.value(),
                self.d6_burst.value(), self.d7_burst.value(),
            ],
            drum_level: [
                self.d0_level.value(), self.d1_level.value(),
                self.d2_level.value(), self.d3_level.value(),
                self.d4_level.value(), self.d5_level.value(),
                self.d6_level.value(), self.d7_level.value(),
            ],
        }
    }

    pub fn d_wave(&self, i: usize) -> &IntParam {
        match i { 0 => &self.d0_wave, 1 => &self.d1_wave, 2 => &self.d2_wave, 3 => &self.d3_wave,
                  4 => &self.d4_wave, 5 => &self.d5_wave, 6 => &self.d6_wave, _ => &self.d7_wave }
    }
    pub fn d_freq(&self, i: usize) -> &FloatParam {
        match i { 0 => &self.d0_freq, 1 => &self.d1_freq, 2 => &self.d2_freq, 3 => &self.d3_freq,
                  4 => &self.d4_freq, 5 => &self.d5_freq, 6 => &self.d6_freq, _ => &self.d7_freq }
    }
    pub fn d_ratio(&self, i: usize) -> &FloatParam {
        match i { 0 => &self.d0_ratio, 1 => &self.d1_ratio, 2 => &self.d2_ratio, 3 => &self.d3_ratio,
                  4 => &self.d4_ratio, 5 => &self.d5_ratio, 6 => &self.d6_ratio, _ => &self.d7_ratio }
    }
    pub fn d_noise(&self, i: usize) -> &FloatParam {
        match i { 0 => &self.d0_noise, 1 => &self.d1_noise, 2 => &self.d2_noise, 3 => &self.d3_noise,
                  4 => &self.d4_noise, 5 => &self.d5_noise, 6 => &self.d6_noise, _ => &self.d7_noise }
    }
    pub fn d_pitch_env(&self, i: usize) -> &FloatParam {
        match i { 0 => &self.d0_pitch_env, 1 => &self.d1_pitch_env, 2 => &self.d2_pitch_env, 3 => &self.d3_pitch_env,
                  4 => &self.d4_pitch_env, 5 => &self.d5_pitch_env, 6 => &self.d6_pitch_env, _ => &self.d7_pitch_env }
    }
    pub fn d_pitch_time(&self, i: usize) -> &FloatParam {
        match i { 0 => &self.d0_pitch_time, 1 => &self.d1_pitch_time, 2 => &self.d2_pitch_time, 3 => &self.d3_pitch_time,
                  4 => &self.d4_pitch_time, 5 => &self.d5_pitch_time, 6 => &self.d6_pitch_time, _ => &self.d7_pitch_time }
    }
    pub fn d_decay(&self, i: usize) -> &FloatParam {
        match i { 0 => &self.d0_decay, 1 => &self.d1_decay, 2 => &self.d2_decay, 3 => &self.d3_decay,
                  4 => &self.d4_decay, 5 => &self.d5_decay, 6 => &self.d6_decay, _ => &self.d7_decay }
    }
    pub fn d_burst(&self, i: usize) -> &FloatParam {
        match i { 0 => &self.d0_burst, 1 => &self.d1_burst, 2 => &self.d2_burst, 3 => &self.d3_burst,
                  4 => &self.d4_burst, 5 => &self.d5_burst, 6 => &self.d6_burst, _ => &self.d7_burst }
    }
    pub fn d_level(&self, i: usize) -> &FloatParam {
        match i { 0 => &self.d0_level, 1 => &self.d1_level, 2 => &self.d2_level, 3 => &self.d3_level,
                  4 => &self.d4_level, 5 => &self.d5_level, 6 => &self.d6_level, _ => &self.d7_level }
    }
    pub fn sq(&self, i: usize) -> &IntParam {
        match i { 0 => &self.sq0, 1 => &self.sq1, 2 => &self.sq2, 3 => &self.sq3,
                  4 => &self.sq4, 5 => &self.sq5, 6 => &self.sq6, _ => &self.sq7 }
    }
}
