# min_max_synth

A retro chiptune synthesizer (NES / Gameboy / Sega Genesis flavours) written in
Rust. Builds simultaneously as a CLAP plugin, a VST3 plugin, and a standalone
application with a built-in QWERTY keyboard for testing.

## Features

- **6 oscillators**: pulse (variable duty), 4-bit triangle, Gameboy 4-bit
  wavetable, LFSR noise (with NES short/long period), 2-op FM, and saw
- **ADSR envelope**, vibrato LFO with depth/rate/delay, pitch sweep, fine tune
  & octave shift
- **Bitcrusher** (1–16 bits) with sample-rate reduction (1 kHz – 96 kHz) for
  baked-in lo-fi grit
- **Embedded percussion samples** (8-bit, 11.025 kHz, generated procedurally —
  no external assets) covering kick, snare, closed/open hat, tom, clap,
  cowbell and zap. In drum mode the keyboard maps to the kit, optionally
  pitch-tracking the played note
- **8-voice polyphony** with oldest-voice stealing
- **Factory presets** for each emulated console
- **Self-contained UI** built with `egui`
- **QWERTY keyboard** (Z S X D C V G B H N J M for the bottom octave, Q 2 W 3
  E R 5 T 6 Y 7 U for the top, PageUp/PageDown to shift octave)

## Build & run

```bash
mise exec -- cargo run --release --bin min_max_standalone
```

The standalone binary will pick a default audio device and open the editor
window. Click on the editor to focus it, then press keys to play notes.

To build the plugins:

```bash
mise exec -- cargo build --release
```

The CLAP plugin is at `target/release/libmin_max_synth.{dylib,so,dll}` (rename
to `.clap` and place in your CLAP plugin directory). VST3 bundling is left to
the [`cargo-nih-plug`](https://github.com/robbert-vdh/nih-plug/tree/master/xtask)
xtask if you need a proper bundle.

## Tests

```bash
mise exec -- cargo test
```
