//! Built-in factory presets covering the major retro consoles.

use crate::params::{SynthParams, WaveChoice};
use nih_plug::prelude::ParamSetter;

/// Display names of the built-in presets, in display order.
pub const PRESET_NAMES: &[&str] = &[
    "NES — Square Lead",
    "NES — Triangle Bass",
    "NES — Noise Hat",
    "Gameboy — Wave Pad",
    "Gameboy — Pulse Arp",
    "Genesis — FM Bell",
    "Genesis — FM Bass",
    "Drum Kit (8-bit)",
    "Laser / Zap",
];

/// Apply preset `idx` (matching `PRESET_NAMES`) using the supplied `ParamSetter`
/// so host automation/undo work correctly.
pub fn apply_preset_with_setter(idx: usize, p: &SynthParams, s: &ParamSetter) {
    match idx {
        0 => nes_square_lead(p, s),
        1 => nes_triangle_bass(p, s),
        2 => nes_noise_hat(p, s),
        3 => gb_wave_pad(p, s),
        4 => gb_pulse_arp(p, s),
        5 => genesis_fm_bell(p, s),
        6 => genesis_fm_bass(p, s),
        7 => drum_kit(p, s),
        8 => laser(p, s),
        _ => {}
    }
}

fn nes_square_lead(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.attack, 2.0);
    s.set_parameter(&p.decay, 60.0);
    s.set_parameter(&p.sustain, 0.85);
    s.set_parameter(&p.release, 80.0);
    s.set_parameter(&p.vibrato_rate, 6.0);
    s.set_parameter(&p.vibrato_depth, 0.15);
    s.set_parameter(&p.bit_depth, 16.0);
    s.set_parameter(&p.bit_rate, 44_100.0);
    s.set_parameter(&p.octave, 0);
    s.set_parameter(&p.drum_mode, false);
}

fn nes_triangle_bass(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Triangle);
    s.set_parameter(&p.attack, 1.0);
    s.set_parameter(&p.decay, 200.0);
    s.set_parameter(&p.sustain, 0.9);
    s.set_parameter(&p.release, 120.0);
    s.set_parameter(&p.vibrato_depth, 0.0);
    s.set_parameter(&p.octave, -1);
    s.set_parameter(&p.drum_mode, false);
}

fn nes_noise_hat(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Noise);
    s.set_parameter(&p.noise_short, false);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 60.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 30.0);
    s.set_parameter(&p.drum_mode, false);
}

fn gb_wave_pad(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Wave);
    s.set_parameter(&p.attack, 80.0);
    s.set_parameter(&p.decay, 400.0);
    s.set_parameter(&p.sustain, 0.7);
    s.set_parameter(&p.release, 400.0);
    s.set_parameter(&p.vibrato_rate, 4.0);
    s.set_parameter(&p.vibrato_depth, 0.05);
    s.set_parameter(&p.bit_depth, 4.0);
    s.set_parameter(&p.bit_rate, 22_050.0);
    s.set_parameter(&p.drum_mode, false);
}

fn gb_pulse_arp(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.25);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 80.0);
    s.set_parameter(&p.sustain, 0.5);
    s.set_parameter(&p.release, 60.0);
    s.set_parameter(&p.bit_depth, 8.0);
    s.set_parameter(&p.bit_rate, 22_050.0);
    s.set_parameter(&p.drum_mode, false);
    s.set_parameter(&p.mono, true);
    s.set_parameter(&p.arp_rate, 16.0);
}

fn genesis_fm_bell(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Fm);
    s.set_parameter(&p.fm_ratio, 3.5);
    s.set_parameter(&p.fm_index, 4.0);
    s.set_parameter(&p.attack, 1.0);
    s.set_parameter(&p.decay, 800.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 800.0);
    s.set_parameter(&p.bit_depth, 12.0);
    s.set_parameter(&p.bit_rate, 32_000.0);
    s.set_parameter(&p.drum_mode, false);
}

fn genesis_fm_bass(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Fm);
    s.set_parameter(&p.fm_ratio, 1.0);
    s.set_parameter(&p.fm_index, 2.0);
    s.set_parameter(&p.attack, 1.0);
    s.set_parameter(&p.decay, 200.0);
    s.set_parameter(&p.sustain, 0.6);
    s.set_parameter(&p.release, 150.0);
    s.set_parameter(&p.octave, -1);
    s.set_parameter(&p.bit_depth, 12.0);
    s.set_parameter(&p.bit_rate, 32_000.0);
    s.set_parameter(&p.drum_mode, false);
}

fn drum_kit(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.drum_mode, true);
    s.set_parameter(&p.drum_pitch, false);
    s.set_parameter(&p.bit_depth, 8.0);
    s.set_parameter(&p.bit_rate, 11_025.0);
}

fn laser(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 200.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 50.0);
    s.set_parameter(&p.sweep_semi, -36.0);
    s.set_parameter(&p.sweep_time, 200.0);
    s.set_parameter(&p.drum_mode, false);
}
