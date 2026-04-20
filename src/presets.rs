//! Built-in factory presets covering the major retro consoles.

use crate::params::{SynthParams, WaveChoice};
use nih_plug::prelude::ParamSetter;

/// Display names of the built-in presets, in display order.
pub const PRESET_NAMES: &[&str] = &[
    // NES / 2A03
    "NES — Square Lead",
    "NES — Triangle Bass",
    "NES — Noise Hat",
    "NES — Pulse Pluck",
    "NES — Vibrato Lead",
    "NES — PWM Sweep",
    "NES — Arp 'Octave'",
    "NES — Sub Bass",
    // Gameboy / DMG
    "Gameboy — Wave Pad",
    "Gameboy — Pulse Arp",
    "Gameboy — Crunchy Lead",
    "Gameboy — Wobble Bass",
    "Gameboy — Chord Stab",
    // Genesis / FM
    "Genesis — FM Bell",
    "Genesis — FM Bass",
    "Genesis — FM Brass",
    "Genesis — FM Organ",
    "Genesis — FM E-Piano",
    "Genesis — FM Slap",
    // SID / C64
    "SID — Reso Lead",
    "SID — Pulse Bass",
    "SID — Tri Pluck",
    "SID — Noise Crash",
    // Saw / chip-wave
    "Saw — Lo-Fi Lead",
    "Saw — Detuned Stab",
    // SFX / utility
    "Laser / Zap",
    "UI Blip",
    "Coin",
    "Power-Up",
    "Power-Down",
    "Explosion",
    "Jump",
    "Land",
    "Alarm",
    // Drum kits
    "Drum Kit (8-bit)",
    "Drum Kit — Lo-Fi",
    "Drum Kit — Tight",
];

/// Apply preset `idx` (matching `PRESET_NAMES`) using the supplied `ParamSetter`
/// so host automation/undo work correctly.
pub fn apply_preset_with_setter(idx: usize, p: &SynthParams, s: &ParamSetter) {
    reset_to_defaults(p, s);
    match idx {
        0 => nes_square_lead(p, s),
        1 => nes_triangle_bass(p, s),
        2 => nes_noise_hat(p, s),
        3 => nes_pulse_pluck(p, s),
        4 => nes_vibrato_lead(p, s),
        5 => nes_pwm_sweep(p, s),
        6 => nes_arp_octave(p, s),
        7 => nes_sub_bass(p, s),
        8 => gb_wave_pad(p, s),
        9 => gb_pulse_arp(p, s),
        10 => gb_crunchy_lead(p, s),
        11 => gb_wobble_bass(p, s),
        12 => gb_chord_stab(p, s),
        13 => genesis_fm_bell(p, s),
        14 => genesis_fm_bass(p, s),
        15 => genesis_fm_brass(p, s),
        16 => genesis_fm_organ(p, s),
        17 => genesis_fm_epiano(p, s),
        18 => genesis_fm_slap(p, s),
        19 => sid_reso_lead(p, s),
        20 => sid_pulse_bass(p, s),
        21 => sid_tri_pluck(p, s),
        22 => sid_noise_crash(p, s),
        23 => saw_lofi_lead(p, s),
        24 => saw_detuned_stab(p, s),
        25 => laser(p, s),
        26 => ui_blip(p, s),
        27 => coin(p, s),
        28 => power_up(p, s),
        29 => power_down(p, s),
        30 => explosion(p, s),
        31 => jump(p, s),
        32 => land(p, s),
        33 => alarm(p, s),
        34 => drum_kit(p, s),
        35 => drum_kit_lofi(p, s),
        36 => drum_kit_tight(p, s),
        _ => {}
    }
}

/// Reset every non-cosmetic parameter to a known baseline so presets don't
/// inherit leftover state from whatever was previously loaded.
fn reset_to_defaults(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.noise_short, false);
    s.set_parameter(&p.fm_ratio, 2.0);
    s.set_parameter(&p.fm_index, 1.5);
    s.set_parameter(&p.duty_lfo_rate, 4.0);
    s.set_parameter(&p.duty_lfo_depth, 0.0);
    s.set_parameter(&p.vibrato_rate, 5.0);
    s.set_parameter(&p.vibrato_depth, 0.0);
    s.set_parameter(&p.vibrato_delay, 0.0);
    s.set_parameter(&p.sweep_semi, 0.0);
    s.set_parameter(&p.sweep_time, 0.0);
    s.set_parameter(&p.mono, false);
    s.set_parameter(&p.arp_rate, 0.0);
    s.set_parameter(&p.bit_depth, 16.0);
    s.set_parameter(&p.bit_rate, 44_100.0);
    s.set_parameter(&p.fine_tune, 0.0);
    s.set_parameter(&p.octave, 0);
    s.set_parameter(&p.drum_mode, false);
    s.set_parameter(&p.drum_pitch, true);
    // Per-drum params are not reset here; they're slot-specific and the user
    // may have tuned them. The drum-kit presets explicitly reset / set them.
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

// --- NES additions ----------------------------------------------------------

fn nes_pulse_pluck(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.125);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 90.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 60.0);
}

fn nes_vibrato_lead(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.attack, 4.0);
    s.set_parameter(&p.decay, 200.0);
    s.set_parameter(&p.sustain, 0.85);
    s.set_parameter(&p.release, 200.0);
    s.set_parameter(&p.vibrato_rate, 6.5);
    s.set_parameter(&p.vibrato_depth, 0.35);
    s.set_parameter(&p.vibrato_delay, 200.0);
}

fn nes_pwm_sweep(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.duty_lfo_rate, 0.6);
    s.set_parameter(&p.duty_lfo_depth, 0.85);
    s.set_parameter(&p.attack, 10.0);
    s.set_parameter(&p.decay, 400.0);
    s.set_parameter(&p.sustain, 0.8);
    s.set_parameter(&p.release, 300.0);
}

fn nes_arp_octave(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.25);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 60.0);
    s.set_parameter(&p.sustain, 0.6);
    s.set_parameter(&p.release, 50.0);
    s.set_parameter(&p.mono, true);
    s.set_parameter(&p.arp_rate, 24.0);
}

fn nes_sub_bass(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Triangle);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 100.0);
    s.set_parameter(&p.sustain, 0.95);
    s.set_parameter(&p.release, 80.0);
    s.set_parameter(&p.octave, -2);
}

// --- Gameboy additions ------------------------------------------------------

fn gb_crunchy_lead(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.75);
    s.set_parameter(&p.attack, 1.0);
    s.set_parameter(&p.decay, 150.0);
    s.set_parameter(&p.sustain, 0.7);
    s.set_parameter(&p.release, 120.0);
    s.set_parameter(&p.bit_depth, 6.0);
    s.set_parameter(&p.bit_rate, 16_000.0);
}

fn gb_wobble_bass(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.duty_lfo_rate, 6.0);
    s.set_parameter(&p.duty_lfo_depth, 0.5);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 250.0);
    s.set_parameter(&p.sustain, 0.7);
    s.set_parameter(&p.release, 100.0);
    s.set_parameter(&p.octave, -1);
    s.set_parameter(&p.bit_depth, 6.0);
    s.set_parameter(&p.bit_rate, 16_000.0);
}

fn gb_chord_stab(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Wave);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 200.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 200.0);
    s.set_parameter(&p.bit_depth, 4.0);
    s.set_parameter(&p.bit_rate, 22_050.0);
}

// --- Genesis / FM additions -------------------------------------------------

fn genesis_fm_brass(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Fm);
    s.set_parameter(&p.fm_ratio, 1.0);
    s.set_parameter(&p.fm_index, 3.0);
    s.set_parameter(&p.attack, 30.0);
    s.set_parameter(&p.decay, 200.0);
    s.set_parameter(&p.sustain, 0.85);
    s.set_parameter(&p.release, 200.0);
}

fn genesis_fm_organ(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Fm);
    s.set_parameter(&p.fm_ratio, 2.0);
    s.set_parameter(&p.fm_index, 1.5);
    s.set_parameter(&p.attack, 5.0);
    s.set_parameter(&p.decay, 50.0);
    s.set_parameter(&p.sustain, 1.0);
    s.set_parameter(&p.release, 100.0);
    s.set_parameter(&p.vibrato_rate, 5.5);
    s.set_parameter(&p.vibrato_depth, 0.08);
    s.set_parameter(&p.vibrato_delay, 250.0);
}

fn genesis_fm_epiano(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Fm);
    s.set_parameter(&p.fm_ratio, 1.0);
    s.set_parameter(&p.fm_index, 5.0);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 700.0);
    s.set_parameter(&p.sustain, 0.2);
    s.set_parameter(&p.release, 600.0);
}

fn genesis_fm_slap(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Fm);
    s.set_parameter(&p.fm_ratio, 0.5);
    s.set_parameter(&p.fm_index, 6.0);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 120.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 80.0);
    s.set_parameter(&p.octave, -1);
}

// --- SID --------------------------------------------------------------------

fn sid_reso_lead(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Saw);
    s.set_parameter(&p.attack, 5.0);
    s.set_parameter(&p.decay, 250.0);
    s.set_parameter(&p.sustain, 0.7);
    s.set_parameter(&p.release, 200.0);
    s.set_parameter(&p.vibrato_rate, 5.0);
    s.set_parameter(&p.vibrato_depth, 0.2);
    s.set_parameter(&p.vibrato_delay, 150.0);
    s.set_parameter(&p.bit_depth, 12.0);
}

fn sid_pulse_bass(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.3);
    s.set_parameter(&p.duty_lfo_rate, 0.4);
    s.set_parameter(&p.duty_lfo_depth, 0.4);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 200.0);
    s.set_parameter(&p.sustain, 0.8);
    s.set_parameter(&p.release, 100.0);
    s.set_parameter(&p.octave, -1);
}

fn sid_tri_pluck(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Triangle);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 350.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 200.0);
}

fn sid_noise_crash(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Noise);
    s.set_parameter(&p.noise_short, false);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 600.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 400.0);
}

// --- Saw --------------------------------------------------------------------

fn saw_lofi_lead(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Saw);
    s.set_parameter(&p.attack, 2.0);
    s.set_parameter(&p.decay, 150.0);
    s.set_parameter(&p.sustain, 0.75);
    s.set_parameter(&p.release, 100.0);
    s.set_parameter(&p.bit_depth, 6.0);
    s.set_parameter(&p.bit_rate, 12_000.0);
}

fn saw_detuned_stab(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Saw);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 250.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 200.0);
    s.set_parameter(&p.fine_tune, 12.0);
    s.set_parameter(&p.vibrato_rate, 4.0);
    s.set_parameter(&p.vibrato_depth, 0.05);
}

// --- SFX --------------------------------------------------------------------

fn ui_blip(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 40.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 20.0);
    s.set_parameter(&p.octave, 1);
}

fn coin(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 200.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 100.0);
    s.set_parameter(&p.sweep_semi, -7.0);
    s.set_parameter(&p.sweep_time, 80.0);
    s.set_parameter(&p.octave, 1);
}

fn power_up(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.25);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 400.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 200.0);
    s.set_parameter(&p.sweep_semi, -24.0);
    s.set_parameter(&p.sweep_time, 400.0);
    s.set_parameter(&p.mono, true);
    s.set_parameter(&p.arp_rate, 18.0);
}

fn power_down(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 600.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 200.0);
    s.set_parameter(&p.sweep_semi, 24.0);
    s.set_parameter(&p.sweep_time, 600.0);
    s.set_parameter(&p.mono, true);
    s.set_parameter(&p.arp_rate, 14.0);
}

fn explosion(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Noise);
    s.set_parameter(&p.noise_short, false);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 800.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 400.0);
    s.set_parameter(&p.bit_depth, 4.0);
    s.set_parameter(&p.bit_rate, 8_000.0);
}

fn jump(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 150.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 80.0);
    s.set_parameter(&p.sweep_semi, -12.0);
    s.set_parameter(&p.sweep_time, 150.0);
}

fn land(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Noise);
    s.set_parameter(&p.noise_short, true);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 80.0);
    s.set_parameter(&p.sustain, 0.0);
    s.set_parameter(&p.release, 40.0);
}

fn alarm(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.waveform, WaveChoice::Pulse);
    s.set_parameter(&p.pulse_duty, 0.5);
    s.set_parameter(&p.attack, 0.0);
    s.set_parameter(&p.decay, 100.0);
    s.set_parameter(&p.sustain, 1.0);
    s.set_parameter(&p.release, 40.0);
    s.set_parameter(&p.vibrato_rate, 8.0);
    s.set_parameter(&p.vibrato_depth, 1.0);
    s.set_parameter(&p.octave, 1);
}

// --- Drum kit variants ------------------------------------------------------

fn drum_kit_lofi(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.drum_mode, true);
    s.set_parameter(&p.drum_pitch, false);
    s.set_parameter(&p.bit_depth, 4.0);
    s.set_parameter(&p.bit_rate, 6_000.0);
    // Lower, fatter kick.
    s.set_parameter(p.d_freq(0), 38.0);
    s.set_parameter(p.d_decay(0), 320.0);
    // Tighter hats.
    s.set_parameter(p.d_decay(2), 20.0);
    s.set_parameter(p.d_decay(3), 120.0);
}

fn drum_kit_tight(p: &SynthParams, s: &ParamSetter) {
    s.set_parameter(&p.drum_mode, true);
    s.set_parameter(&p.drum_pitch, false);
    s.set_parameter(&p.bit_depth, 12.0);
    s.set_parameter(&p.bit_rate, 22_050.0);
    s.set_parameter(p.d_freq(0), 55.0);
    s.set_parameter(p.d_decay(0), 150.0);
    s.set_parameter(p.d_decay(1), 80.0);
    s.set_parameter(p.d_decay(2), 18.0);
    s.set_parameter(p.d_decay(3), 150.0);
    s.set_parameter(p.d_level(5), 1.2); // louder clap
}
