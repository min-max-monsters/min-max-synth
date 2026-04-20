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

    #[id = "bit_dp"]
    pub bit_depth: FloatParam,
    #[id = "bit_rt"]
    pub bit_rate: FloatParam,

    #[id = "tune"]
    pub fine_tune: FloatParam,
    #[id = "octv"]
    pub octave: IntParam,

    #[id = "drum_o"]
    pub drum_mode: BoolParam,
    #[id = "drum_p"]
    pub drum_pitch: BoolParam,

    // Per-drum tweaks: tune (semitones), decay (0..1, 1 = natural), level (0..1).
    #[id = "dr0_t"] pub drum_tune_0: FloatParam,
    #[id = "dr0_d"] pub drum_decay_0: FloatParam,
    #[id = "dr0_l"] pub drum_level_0: FloatParam,
    #[id = "dr1_t"] pub drum_tune_1: FloatParam,
    #[id = "dr1_d"] pub drum_decay_1: FloatParam,
    #[id = "dr1_l"] pub drum_level_1: FloatParam,
    #[id = "dr2_t"] pub drum_tune_2: FloatParam,
    #[id = "dr2_d"] pub drum_decay_2: FloatParam,
    #[id = "dr2_l"] pub drum_level_2: FloatParam,
    #[id = "dr3_t"] pub drum_tune_3: FloatParam,
    #[id = "dr3_d"] pub drum_decay_3: FloatParam,
    #[id = "dr3_l"] pub drum_level_3: FloatParam,
    #[id = "dr4_t"] pub drum_tune_4: FloatParam,
    #[id = "dr4_d"] pub drum_decay_4: FloatParam,
    #[id = "dr4_l"] pub drum_level_4: FloatParam,
    #[id = "dr5_t"] pub drum_tune_5: FloatParam,
    #[id = "dr5_d"] pub drum_decay_5: FloatParam,
    #[id = "dr5_l"] pub drum_level_5: FloatParam,
    #[id = "dr6_t"] pub drum_tune_6: FloatParam,
    #[id = "dr6_d"] pub drum_decay_6: FloatParam,
    #[id = "dr6_l"] pub drum_level_6: FloatParam,
    #[id = "dr7_t"] pub drum_tune_7: FloatParam,
    #[id = "dr7_d"] pub drum_decay_7: FloatParam,
    #[id = "dr7_l"] pub drum_level_7: FloatParam,
}

impl Default for SynthParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(1280, 780),

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

            bit_depth: FloatParam::new("Bit Depth", 16.0, FloatRange::Linear { min: 1.0, max: 16.0 })
                .with_unit(" bits").with_step_size(0.5),
            bit_rate: FloatParam::new(
                "Bit Rate",
                44_100.0,
                FloatRange::Skewed { min: 1_000.0, max: 96_000.0, factor: FloatRange::skew_factor(-1.0) },
            )
            .with_unit(" Hz").with_step_size(100.0),

            fine_tune: FloatParam::new("Fine Tune", 0.0, FloatRange::Linear { min: -100.0, max: 100.0 })
                .with_unit(" cents").with_step_size(1.0),
            octave: IntParam::new("Octave", 0, IntRange::Linear { min: -3, max: 3 }),

            drum_mode: BoolParam::new("Drum Mode", false),
            drum_pitch: BoolParam::new("Drum Pitch Tracks Note", true),

            drum_tune_0: drum_tune("Kick"),    drum_decay_0: drum_decay("Kick"),    drum_level_0: drum_level("Kick"),
            drum_tune_1: drum_tune("Snare"),   drum_decay_1: drum_decay("Snare"),   drum_level_1: drum_level("Snare"),
            drum_tune_2: drum_tune("Hat Cl"),  drum_decay_2: drum_decay("Hat Cl"),  drum_level_2: drum_level("Hat Cl"),
            drum_tune_3: drum_tune("Hat Op"),  drum_decay_3: drum_decay("Hat Op"),  drum_level_3: drum_level("Hat Op"),
            drum_tune_4: drum_tune("Tom"),     drum_decay_4: drum_decay("Tom"),     drum_level_4: drum_level("Tom"),
            drum_tune_5: drum_tune("Clap"),    drum_decay_5: drum_decay("Clap"),    drum_level_5: drum_level("Clap"),
            drum_tune_6: drum_tune("Cowbell"), drum_decay_6: drum_decay("Cowbell"), drum_level_6: drum_level("Cowbell"),
            drum_tune_7: drum_tune("Zap"),     drum_decay_7: drum_decay("Zap"),     drum_level_7: drum_level("Zap"),
        }
    }
}

fn drum_tune(name: &str) -> FloatParam {
    FloatParam::new(
        &format!("{name} Tune"),
        0.0,
        FloatRange::Linear { min: -24.0, max: 24.0 },
    )
    .with_unit(" semis")
    .with_step_size(0.1)
}

fn drum_decay(name: &str) -> FloatParam {
    FloatParam::new(
        &format!("{name} Decay"),
        1.0,
        FloatRange::Linear { min: 0.05, max: 1.5 },
    )
    .with_step_size(0.01)
}

fn drum_level(name: &str) -> FloatParam {
    FloatParam::new(
        &format!("{name} Level"),
        1.0,
        FloatRange::Linear { min: 0.0, max: 1.5 },
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
            bit_depth: self.bit_depth.value(),
            bit_rate_hz: self.bit_rate.value(),
            fine_tune_cents: self.fine_tune.value(),
            octave_shift: self.octave.value(),
            drum_mode: self.drum_mode.value(),
            drum_pitch: self.drum_pitch.value(),
            drum_tune: [
                self.drum_tune_0.value(), self.drum_tune_1.value(),
                self.drum_tune_2.value(), self.drum_tune_3.value(),
                self.drum_tune_4.value(), self.drum_tune_5.value(),
                self.drum_tune_6.value(), self.drum_tune_7.value(),
            ],
            drum_decay: [
                self.drum_decay_0.value(), self.drum_decay_1.value(),
                self.drum_decay_2.value(), self.drum_decay_3.value(),
                self.drum_decay_4.value(), self.drum_decay_5.value(),
                self.drum_decay_6.value(), self.drum_decay_7.value(),
            ],
            drum_level: [
                self.drum_level_0.value(), self.drum_level_1.value(),
                self.drum_level_2.value(), self.drum_level_3.value(),
                self.drum_level_4.value(), self.drum_level_5.value(),
                self.drum_level_6.value(), self.drum_level_7.value(),
            ],
        }
    }

    pub fn drum_tune(&self, i: usize) -> &FloatParam {
        match i {
            0 => &self.drum_tune_0, 1 => &self.drum_tune_1,
            2 => &self.drum_tune_2, 3 => &self.drum_tune_3,
            4 => &self.drum_tune_4, 5 => &self.drum_tune_5,
            6 => &self.drum_tune_6, _ => &self.drum_tune_7,
        }
    }
    pub fn drum_decay(&self, i: usize) -> &FloatParam {
        match i {
            0 => &self.drum_decay_0, 1 => &self.drum_decay_1,
            2 => &self.drum_decay_2, 3 => &self.drum_decay_3,
            4 => &self.drum_decay_4, 5 => &self.drum_decay_5,
            6 => &self.drum_decay_6, _ => &self.drum_decay_7,
        }
    }
    pub fn drum_level(&self, i: usize) -> &FloatParam {
        match i {
            0 => &self.drum_level_0, 1 => &self.drum_level_1,
            2 => &self.drum_level_2, 3 => &self.drum_level_3,
            4 => &self.drum_level_4, 5 => &self.drum_level_5,
            6 => &self.drum_level_6, _ => &self.drum_level_7,
        }
    }
}
