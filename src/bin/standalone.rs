//! Standalone runner: launches the synth in its own window with a built-in
//! audio backend so you can play it with the QWERTY keyboard without a host.
//!
//! When no explicit `--sample-rate` or `--backend` is given, this binary
//! queries the default output device for its supported configs and
//! tries combinations of audio layout × sample rate × period size before
//! falling back to the silent dummy backend.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
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
    // Keep MIDI connections alive for the lifetime of main() — dropping
    // them would close the ports.
    let _midi_connections = if !user_set_midi {
        // Force-initialize the global note queue on the main thread so the
        // LazyLock allocation doesn't happen on the real-time audio thread
        // (which would trip nih-plug's assert_no_alloc guard).
        let _ = &*min_max_synth::EXTERNAL_NOTE_QUEUE;

        // Connect to ALL MIDI inputs via midir, forwarding events to the
        // plugin's global queue.  This way every controller works out of
        // the box without a terminal prompt.
        connect_all_midi_inputs()
    } else {
        Vec::new()
    };

    // Query the default output device for configs it actually supports.
    let combos = query_device_combos();

    if combos.is_empty() {
        eprintln!("[standalone] Could not query audio device, trying default auto …");
        nih_export_standalone::<MinMaxSynth>();
        return;
    }

    let native_backend = if cfg!(target_os = "macos") {
        "core-audio"
    } else if cfg!(target_os = "windows") {
        "wasapi"
    } else {
        "auto"
    };

    for (layout, sr, period) in &combos {
        let args = build_args(native_backend, *layout, *sr, *period, &extra_args);
        eprintln!(
            "[standalone] Trying {native_backend} layout {} at {} Hz / {} samples …",
            layout, sr, period
        );
        if nih_export_standalone_with_args::<MinMaxSynth, _>(args.into_iter()) {
            return;
        }
    }

    // None of the detected configs worked — fall back to auto.
    eprintln!("[standalone] All {native_backend} configs failed, falling back to auto …");
    nih_export_standalone::<MinMaxSynth>();
}

/// Channel counts for each audio layout index (must match `AUDIO_IO_LAYOUTS`).
const LAYOUT_CHANNELS: &[u16] = &[2, 1, 4];

/// Preferred layout order: stereo first, then quad, then mono.
const LAYOUT_PRIORITY: &[usize] = &[0, 2, 1];

/// Query the default output device and return a prioritised list of
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
                if cfg!(target_os = "windows") {
                    // WASAPI ignores the requested buffer size and delivers
                    // its own native period.  nih-plug panics when the actual
                    // count exceeds the configured period, so we probe the
                    // real WASAPI buffer size once and use that.
                    let period = probe_wasapi_buffer_size(&device, ch, sr);
                    combos.push((layout_idx, sr, period));
                } else {
                    for &period in &[512u32, 256, 1024] {
                        combos.push((layout_idx, sr, period));
                    }
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

/// Probe the real buffer size WASAPI will deliver by briefly opening a cpal
/// stream with `BufferSize::Fixed(512)`.  WASAPI may return a completely
/// different frame count; we capture the **maximum** across several callbacks.
/// Falls back to 4096 if the probe fails.
fn probe_wasapi_buffer_size(device: &cpal::Device, channels: u16, sample_rate: u32) -> u32 {
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    let config = cpal::StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Fixed(512),
    };

    let max_frames = Arc::new(AtomicU32::new(0));
    let mf = max_frames.clone();
    let ch = channels as u32;

    let stream = match device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let frames = data.len() as u32 / ch;
            mf.fetch_max(frames, Ordering::Relaxed);
        },
        |err| eprintln!("[standalone] probe error: {err}"),
        None,
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[standalone] Could not probe buffer size: {e}");
            return 4096;
        }
    };

    if stream.play().is_err() {
        return 4096;
    }

    // Let several callbacks fire so we see the steady-state size.
    std::thread::sleep(std::time::Duration::from_millis(500));
    drop(stream);

    let observed = max_frames.load(Ordering::Relaxed);
    if observed > 0 {
        eprintln!("[standalone] Probed WASAPI buffer size: {observed} frames");
        observed
    } else {
        eprintln!("[standalone] Probe returned 0, defaulting to 4096");
        4096
    }
}

/// Connect to every available MIDI input device via midir and forward note
/// events to the plugin's global `EXTERNAL_NOTE_QUEUE`.  Returns the live
/// connections (dropping them would close the ports).
fn connect_all_midi_inputs() -> Vec<midir::MidiInputConnection<()>> {
    use min_max_synth::{GuiNoteEvent, EXTERNAL_NOTE_QUEUE};

    let backend = match MidiInput::new("min_max_standalone_scan") {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };
    let ports: Vec<MidiInputPort> = backend.ports();
    if ports.is_empty() {
        eprintln!("[standalone] No MIDI input devices found.");
        return Vec::new();
    }

    let names: Vec<String> = ports
        .iter()
        .filter_map(|p| backend.port_name(p).ok())
        .collect();

    let mut connections = Vec::new();
    for (port, name) in ports.iter().zip(names.iter()) {
        // Skip virtual / loopback ports.
        let low = name.to_lowercase();
        if low.contains("iac") || low.contains("through") {
            continue;
        }

        // Each MidiInput can only open one port, so create a fresh one.
        let input = match MidiInput::new(&format!("min_max_{name}")) {
            Ok(i) => i,
            Err(_) => continue,
        };

        match input.connect(
            port,
            &format!("min_max_{name}"),
            move |_stamp, message, _| {
                // Parse raw MIDI bytes into NoteOn / NoteOff.
                if message.len() >= 3 {
                    let status = message[0] & 0xF0;
                    let note = message[1];
                    let vel = message[2];
                    match status {
                        0x90 if vel > 0 => {
                            let _ = EXTERNAL_NOTE_QUEUE.push(GuiNoteEvent::On {
                                note,
                                velocity: vel as f32 / 127.0,
                            });
                        }
                        0x80 | 0x90 => {
                            let _ = EXTERNAL_NOTE_QUEUE.push(GuiNoteEvent::Off { note });
                        }
                        _ => {}
                    }
                }
            },
            (),
        ) {
            Ok(conn) => {
                eprintln!("[standalone] Connected MIDI input: {name}");
                connections.push(conn);
            }
            Err(e) => {
                eprintln!("[standalone] Failed to connect MIDI input {name}: {e}");
            }
        }
    }

    if connections.is_empty() {
        eprintln!("[standalone] No MIDI inputs connected.");
    }

    connections
}
