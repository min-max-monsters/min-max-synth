//! Plugin entry point: voice management, MIDI handling, and audio rendering.

#![allow(clippy::new_without_default)]

pub mod dsp;
pub mod editor;
pub mod g2p;
pub mod params;
pub mod preset_bank;
pub mod samples;
pub mod voice;
pub mod widgets;

use crate::dsp::{BitCrusher, OnePoleHP, OnePoleLP};
use crate::params::{LegatoMode, SynthParams};
use crate::voice::Voice;
use crossbeam_queue::ArrayQueue;
use nih_plug::prelude::*;
use std::sync::{Arc, LazyLock};

/// Number of polyphonic voices.
pub const NUM_VOICES: usize = 8;

/// Global note queue for external MIDI connections (used by the standalone
/// binary to forward events from all connected MIDI devices).  The audio
/// thread drains this alongside the per-instance GUI queue.
pub static EXTERNAL_NOTE_QUEUE: LazyLock<ArrayQueue<GuiNoteEvent>> =
    LazyLock::new(|| ArrayQueue::new(1024));

/// Note events produced by the on-screen / QWERTY keyboard in the editor and
/// consumed by the audio thread.
#[derive(Debug, Clone, Copy)]
pub enum GuiNoteEvent {
    On { note: u8, velocity: f32 },
    Off { note: u8 },
}

/// The plugin object held by the host.
pub struct MinMaxSynth {
    params: Arc<SynthParams>,
    voices: Vec<Voice>,
    sample_rate: f32,
    crusher: BitCrusher,
    lp: OnePoleLP,
    hp: OnePoleHP,
    age_counter: u64,
    note_queue: Arc<ArrayQueue<GuiNoteEvent>>,
    /// Stack of currently held MIDI notes (oldest first). Used for mono mode
    /// and the fast arpeggiator.
    held_notes: Vec<u8>,
    /// Phase accumulator for the mono arpeggiator (0..1, advances at arp_rate).
    arp_phase: f32,
    /// Index into `held_notes` for the next arp step.
    arp_index: usize,
}

impl Default for MinMaxSynth {
    fn default() -> Self {
        let sr = 44_100.0;
        Self {
            params: Arc::new(SynthParams::default()),
            voices: (0..NUM_VOICES).map(|_| Voice::new(sr)).collect(),
            sample_rate: sr,
            crusher: BitCrusher::default(),
            lp: OnePoleLP::default(),
            hp: OnePoleHP::default(),
            age_counter: 0,
            note_queue: Arc::new(ArrayQueue::new(256)),
            held_notes: Vec::with_capacity(16),
            arp_phase: 0.0,
            arp_index: 0,
        }
    }
}

impl MinMaxSynth {
    fn next_age(&mut self) -> u64 {
        self.age_counter = self.age_counter.wrapping_add(1);
        self.age_counter
    }

    fn handle_note_on(&mut self, note: u8, velocity: f32) {
        let snapshot = self.params.snapshot();
        let age = self.next_age();

        // Update held-note stack (used by mono/arp). Drum mode bypasses it.
        if !snapshot.drum_mode {
            self.held_notes.retain(|&n| n != note);
            self.held_notes.push(note);
        }

        if snapshot.mono && !snapshot.drum_mode {
            // Mono: reuse voice 0. Behaviour depends on legato_mode:
            //   Retrigger — always restart envelope and oscillator phases.
            //   Legato    — swap pitch instantly without retriggering env.
            //   Glide     — swap pitch with a portamento slide; no retrigger.
            self.arp_index = 0;
            self.arp_phase = 0.0;
            if self.voices[0].is_pitched_active() && snapshot.legato_mode != LegatoMode::Retrigger {
                match snapshot.legato_mode {
                    LegatoMode::Glide => self.voices[0].glide_to(note),
                    _ => self.voices[0].set_note(note),
                }
            } else {
                // Silence any other lingering voices first.
                for v in self.voices.iter_mut().skip(1) {
                    v.note_off();
                }
                self.voices[0].note_on(note, velocity, &snapshot, age);
            }
            return;
        }

        // Polyphonic: find a free voice or steal the oldest one.
        let idx = self
            .voices
            .iter()
            .position(|v| !v.is_active())
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.age())
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            });
        self.voices[idx].note_on(note, velocity, &snapshot, age);
    }

    fn handle_note_off(&mut self, note: u8) {
        let snapshot = self.params.snapshot();
        if !snapshot.drum_mode {
            self.held_notes.retain(|&n| n != note);
        }
        if snapshot.mono && !snapshot.drum_mode {
            // Mono: only release when the stack is empty; otherwise fall back
            // to the previously held note.
            if let Some(&prev) = self.held_notes.last() {
                if snapshot.legato_mode == LegatoMode::Glide {
                    self.voices[0].glide_to(prev);
                } else {
                    self.voices[0].set_note(prev);
                }
            } else {
                self.voices[0].note_off();
            }
            return;
        }
        for v in &mut self.voices {
            if v.note() == Some(note) {
                v.note_off();
            }
        }
    }
}

impl Plugin for MinMaxSynth {
    const NAME: &'static str = "min_max_synth";
    const VENDOR: &'static str = "Persy";
    const URL: &'static str = "https://example.com";
    const EMAIL: &'static str = "noreply@example.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        // Layout 0 — stereo (preferred for most devices / DAW hosts).
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: std::num::NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        // Layout 1 — mono.
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: std::num::NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
        // Layout 2 — quad (e.g. 4-channel interfaces like UMC404HD).
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: std::num::NonZeroU32::new(4),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create_editor(self.params.clone(), self.note_queue.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        for v in &mut self.voices {
            v.set_sample_rate(self.sample_rate);
        }
        true
    }

    fn reset(&mut self) {
        for v in &mut self.voices {
            *v = Voice::new(self.sample_rate);
        }
        self.crusher = BitCrusher::default();
        self.lp.reset();
        self.hp.reset();
        self.lp.reset();
        self.hp.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Drain GUI keyboard events first.
        while let Some(ev) = self.note_queue.pop() {
            match ev {
                GuiNoteEvent::On { note, velocity } => self.handle_note_on(note, velocity),
                GuiNoteEvent::Off { note } => self.handle_note_off(note),
            }
        }
        // Drain external MIDI events (standalone multi-device connections).
        while let Some(ev) = EXTERNAL_NOTE_QUEUE.pop() {
            match ev {
                GuiNoteEvent::On { note, velocity } => self.handle_note_on(note, velocity),
                GuiNoteEvent::Off { note } => self.handle_note_off(note),
            }
        }

        let snapshot = self.params.snapshot();
        let mut next_event = context.next_event();
        let num_samples = buffer.samples();

        for (sample_idx, channels) in buffer.iter_samples().enumerate() {
            // Process MIDI events scheduled at this sample.
            while let Some(ev) = next_event {
                if ev.timing() as usize != sample_idx {
                    break;
                }
                match ev {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        self.handle_note_on(note, velocity);
                    }
                    NoteEvent::NoteOff { note, .. } => {
                        self.handle_note_off(note);
                    }
                    _ => {}
                }
                next_event = context.next_event();
            }

            let gain = self.params.gain.smoothed.next();

            // Mono arpeggiator: while two or more notes are held in mono mode
            // and the arp rate is non-zero, cycle voice 0 through the held
            // stack at `arp_rate` Hz. The envelope keeps running so it sounds
            // like a fast chord shimmer (Magical 8bit-style).
            if snapshot.mono
                && !snapshot.drum_mode
                && snapshot.arp_rate > 0.0
                && self.held_notes.len() >= 2
                && self.voices[0].is_pitched_active()
            {
                self.arp_phase += snapshot.arp_rate / self.sample_rate;
                while self.arp_phase >= 1.0 {
                    self.arp_phase -= 1.0;
                    self.arp_index = (self.arp_index + 1) % self.held_notes.len();
                    let n = self.held_notes[self.arp_index];
                    self.voices[0].set_note(n);
                }
            }

            let mut mix = 0.0_f32;
            for v in &mut self.voices {
                mix += v.tick(&snapshot);
            }
            // Apply the bitcrusher once on the bus, not per-voice — otherwise
            // its sample-rate-reduction accumulator would advance N× per
            // sample with N voices and turn into noise.
            mix = self.crusher.process(
                mix,
                self.sample_rate,
                snapshot.bit_rate_hz,
                snapshot.bit_depth,
            );
            // Output-stage RC filters (post-DAC, like real hardware).
            mix = self.lp.process(mix, snapshot.lp_cutoff, self.sample_rate);
            mix = self.hp.process(mix, snapshot.hp_cutoff, self.sample_rate);
            // Give the bus enough headroom that a few simultaneous full-scale
            // pulse voices don't immediately clip, then apply a cheap soft
            // clipper so anything still over the rails distorts musically
            // instead of as a hard square.
            let headroom = (NUM_VOICES as f32).sqrt();
            let out = soft_clip(mix * gain / headroom);
            for s in channels {
                *s = out;
            }
        }

        let _ = num_samples;
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for MinMaxSynth {
    const CLAP_ID: &'static str = "com.persy.min_max_synth";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Retro chiptune synthesizer with embedded percussion samples");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for MinMaxSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"PersyMinMaxSyn01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(MinMaxSynth);
nih_export_vst3!(MinMaxSynth);

/// Cheap tanh-like soft clipper. Linear up to ~|0.5|, smoothly saturates
/// toward ±1.0 above that. Avoids the harsh edge of `clamp()` while staying
/// allocation- and branch-free.
#[inline]
fn soft_clip(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}
