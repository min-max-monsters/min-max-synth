//! Standalone runner: launches the synth in its own window with a built-in
//! audio backend so you can play it with the QWERTY keyboard without a host.
//!
//! When no explicit `--sample-rate` or `--backend` is given, this binary
//! queries the default CoreAudio output device for its supported configs and
//! tries combinations of audio layout × sample rate × period size before
//! falling back to the silent dummy backend.

use cpal::traits::{DeviceTrait, HostTrait};
use midir::{MidiInput, MidiInputPort};
use min_max_synth::MinMaxSynth;
use nih_plug::prelude::*;

/// Number of `AudioIOLayout` entries defined on `MinMaxSynth`.
const NUM_LAYOUTS: usize = 3; // stereo (0), mono (1), quad (2)

fn main() {
    let user_args: Vec<String> = std::env::args().collect();

    // If the user explicitly chose a backend or sample rate, honour that
    // without any fallback magic.
    let has_explicit_config = user_args.iter().any(|a| {
        a == "--sample-rate"
            || a == "-r"
            || a == "--backend"
            || a == "-b"
            || a == "--audio-layout"
            || a == "-l"
    });

    if has_explicit_config {
        nih_export_standalone::<MinMaxSynth>();
        return;
    }

    // Forward any extra flags the user passed (e.g. --midi-input, --output-device).
    let mut extra_args: Vec<String> = user_args.iter().skip(1).cloned().collect();

    // Auto-detect a MIDI input if the user didn't specify one.
    let user_set_midi = extra_args.iter().any(|a| a == "--midi-input");
    if !user_set_midi {
        if let Some(name) = pick_midi_input() {
            eprintln!("[standalone] Auto-connecting MIDI input: {name}");
            extra_args.push("--midi-input".to_string());
            extra_args.push(name);
        } else {
            eprintln!("[standalone] No MIDI input devices found (use --midi-input \"\" to list).");
        }
    }

    // Query the default output device for configs it actually supports.
    let combos = query_device_combos();

    if combos.is_empty() {
        eprintln!("[standalone] Could not query audio device, trying default auto …");
        nih_export_standalone::<MinMaxSynth>();
        return;
    }

    for (layout, sr, period) in &combos {
        let args = build_args("core-audio", *layout, *sr, *period, &extra_args);
        eprintln!(
            "[standalone] Trying CoreAudio layout {} at {} Hz / {} samples …",
            layout, sr, period
        );
        if nih_export_standalone_with_args::<MinMaxSynth, _>(args.into_iter()) {
            return;
        }
    }

    // None of the detected configs worked — fall back to `auto`.
    eprintln!("[standalone] All CoreAudio configs failed, falling back to auto …");
    nih_export_standalone::<MinMaxSynth>();
}

/// Channel counts for each audio layout index (must match `AUDIO_IO_LAYOUTS`).
const LAYOUT_CHANNELS: &[u16] = &[2, 1, 4];

/// Preferred layout order: stereo first, then quad, then mono.
const LAYOUT_PRIORITY: &[usize] = &[0, 2, 1];

/// Query the default CoreAudio output device and return a prioritised list of
/// (audio_layout_index, sample_rate, period_size) triples.
fn query_device_combos() -> Vec<(usize, u32, u32)> {
    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None => return Vec::new(),
    };

    let name = device.name().unwrap_or_default();
    eprintln!("[standalone] Default output device: {name}");

    let supported: Vec<cpal::SupportedStreamConfigRange> =
        match device.supported_output_configs() {
            Ok(cfgs) => cfgs.collect(),
            Err(e) => {
                eprintln!("[standalone] Could not query device configs: {e}");
                return Vec::new();
            }
        };

    if supported.is_empty() {
        return Vec::new();
    }

    // Log what the device reports.
    for cfg in &supported {
        eprintln!(
            "[standalone]   channels: {}, sample rates: {}–{} Hz",
            cfg.channels(),
            cfg.min_sample_rate().0,
            cfg.max_sample_rate().0,
        );
    }

    let preferred_rates: &[u32] = &[48000, 44100, 96000, 192000, 88200];
    let preferred_periods: &[u32] = &[512, 256, 1024];

    // Build combos: for each layout (in priority order), collect matching
    // (sample_rate, period_size) pairs.
    let mut combos = Vec::new();
    for &layout_idx in LAYOUT_PRIORITY {
        if layout_idx >= NUM_LAYOUTS {
            continue;
        }
        let ch = LAYOUT_CHANNELS[layout_idx];
        // Does the device have a config with this channel count?
        let matching_cfgs: Vec<_> = supported.iter().filter(|c| c.channels() == ch).collect();
        if matching_cfgs.is_empty() {
            continue;
        }
        for &sr in preferred_rates {
            let rate = cpal::SampleRate(sr);
            let sr_ok = matching_cfgs
                .iter()
                .any(|c| c.min_sample_rate() <= rate && rate <= c.max_sample_rate());
            if sr_ok {
                for &period in preferred_periods {
                    combos.push((layout_idx, sr, period));
                }
            }
        }
    }

    combos
}

fn build_args(
    backend: &str,
    audio_layout: usize,
    sample_rate: u32,
    period_size: u32,
    extra: &[String],
) -> Vec<String> {
    // nih_plug's --audio-layout is 1-based, so add 1 to our 0-based index.
    let mut args = vec![
        "min_max_standalone".to_string(),
        "--backend".to_string(),
        backend.to_string(),
        "--audio-layout".to_string(),
        (audio_layout + 1).to_string(),
        "--sample-rate".to_string(),
        sample_rate.to_string(),
        "--period-size".to_string(),
        period_size.to_string(),
    ];
    args.extend_from_slice(extra);
    args
}

/// Query connected MIDI input devices and return the name of the first
/// "real" one (skipping IAC virtual buses unless they're the only option).
fn pick_midi_input() -> Option<String> {
    let backend = MidiInput::new("min_max_standalone").ok()?;
    let ports: Vec<MidiInputPort> = backend.ports();
    if ports.is_empty() {
        return None;
    }

    let names: Vec<String> = ports
        .iter()
        .filter_map(|p| backend.port_name(p).ok())
        .collect();

    for n in &names {
        eprintln!("[standalone]   MIDI input: {n}");
    }

    // Prefer hardware devices over IAC / virtual buses.
    names
        .iter()
        .find(|n| !n.to_lowercase().contains("iac") && !n.to_lowercase().contains("through"))
        .cloned()
        .or_else(|| names.first().cloned())
}
