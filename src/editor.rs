//! Egui editor with on-screen QWERTY piano keyboard.

use crate::params::SynthParams;
use crate::presets::{apply_preset_with_setter, PRESET_NAMES};
use crate::samples::DrumKind;
use crate::GuiNoteEvent;
use crossbeam_queue::ArrayQueue;
use nih_plug::prelude::{Editor, ParamSetter};
use nih_plug_egui::egui::{self, Color32, RichText, Stroke, Ui};
use nih_plug_egui::{create_egui_editor, widgets, EguiState};
use std::sync::Arc;

/// QWERTY-to-MIDI mapping like a tracker / many soft synths.
/// Bottom row = white keys C..., top row = sharps.
/// Returns the MIDI note offset from the editor's base note.
const BOTTOM_ROW: &[(char, i32)] = &[
    ('z', 0),  // C
    ('s', 1),  // C#
    ('x', 2),  // D
    ('d', 3),  // D#
    ('c', 4),  // E
    ('v', 5),  // F
    ('g', 6),  // F#
    ('b', 7),  // G
    ('h', 8),  // G#
    ('n', 9),  // A
    ('j', 10), // A#
    ('m', 11), // B
    (',', 12), // C+1
    ('l', 13), // C#+1
    ('.', 14), // D+1
    (';', 15), // D#+1
    ('/', 16), // E+1
];
const TOP_ROW: &[(char, i32)] = &[
    ('q', 12), // C+1
    ('2', 13),
    ('w', 14),
    ('3', 15),
    ('e', 16),
    ('r', 17),
    ('5', 18),
    ('t', 19),
    ('6', 20),
    ('y', 21),
    ('7', 22),
    ('u', 23),
    ('i', 24),
    ('9', 25),
    ('o', 26),
    ('0', 27),
    ('p', 28),
];

pub struct EditorState {
    /// Tracks which keys are currently held so we send note-on once.
    pressed: [bool; 128],
    /// MIDI note that the leftmost QWERTY key (`z`) maps to.
    base_note: i32,
    /// Currently selected preset index in the dropdown.
    selected_preset: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self { pressed: [false; 128], base_note: 60, selected_preset: 0 }
    }
}

/// Build the editor.
pub fn create_editor(
    params: Arc<SynthParams>,
    note_queue: Arc<ArrayQueue<GuiNoteEvent>>,
) -> Option<Box<dyn Editor>> {
    let egui_state: Arc<EguiState> = params.editor_state.clone();
    create_egui_editor(
        egui_state,
        EditorState::default(),
        |_, _| {},
        move |ctx, setter, state| {
            ctx.request_repaint();
            handle_keyboard(ctx, state, &note_queue);

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading(RichText::new("min_max_synth").color(Color32::from_rgb(180, 240, 120)));
                ui.label("Retro chiptune voice — NES / Gameboy / Genesis flavours");
                ui.separator();

                draw_presets(ui, &params, setter, state);
                ui.separator();
                draw_oscillator(ui, &params, setter);
                ui.separator();
                draw_envelope(ui, &params, setter);
                ui.separator();
                draw_modulation(ui, &params, setter);
                ui.separator();
                draw_drums(ui, &params, setter);
                ui.separator();
                draw_keyboard_help(ui, state);
            });
        },
    )
}

fn handle_keyboard(
    ctx: &egui::Context,
    state: &mut EditorState,
    note_queue: &Arc<ArrayQueue<GuiNoteEvent>>,
) {
    ctx.input(|i| {
        // Octave shift via PageUp/PageDown so it doesn't conflict with letters.
        if i.key_pressed(egui::Key::PageUp) {
            state.base_note = (state.base_note + 12).min(108);
        }
        if i.key_pressed(egui::Key::PageDown) {
            state.base_note = (state.base_note - 12).max(12);
        }

        // Build a set of currently-held QWERTY notes from text events.
        let mut held = [false; 128];
        for ev in &i.events {
            if let egui::Event::Key { key, pressed, repeat, .. } = ev {
                if *repeat {
                    continue;
                }
                if let Some(offset) = key_to_offset(*key) {
                    let note = (state.base_note + offset).clamp(0, 127) as usize;
                    if *pressed {
                        held[note] = true;
                    }
                }
            }
        }
        // Persist held state across frames using egui's keys_down for a smoother
        // experience (since keyup events can be missed if the window loses focus).
        for &(ch, off) in BOTTOM_ROW.iter().chain(TOP_ROW.iter()) {
            if let Some(key) = char_to_egui_key(ch) {
                if i.key_down(key) {
                    let note = (state.base_note + off).clamp(0, 127) as usize;
                    held[note] = true;
                }
            }
        }

        for n in 0..128 {
            if held[n] && !state.pressed[n] {
                let _ = note_queue.push(GuiNoteEvent::On { note: n as u8, velocity: 0.9 });
            } else if !held[n] && state.pressed[n] {
                let _ = note_queue.push(GuiNoteEvent::Off { note: n as u8 });
            }
            state.pressed[n] = held[n];
        }
    });
}

fn key_to_offset(key: egui::Key) -> Option<i32> {
    BOTTOM_ROW
        .iter()
        .chain(TOP_ROW.iter())
        .find_map(|(ch, off)| (char_to_egui_key(*ch) == Some(key)).then_some(*off))
}

fn char_to_egui_key(c: char) -> Option<egui::Key> {
    Some(match c {
        'a' => egui::Key::A, 'b' => egui::Key::B, 'c' => egui::Key::C, 'd' => egui::Key::D,
        'e' => egui::Key::E, 'f' => egui::Key::F, 'g' => egui::Key::G, 'h' => egui::Key::H,
        'i' => egui::Key::I, 'j' => egui::Key::J, 'k' => egui::Key::K, 'l' => egui::Key::L,
        'm' => egui::Key::M, 'n' => egui::Key::N, 'o' => egui::Key::O, 'p' => egui::Key::P,
        'q' => egui::Key::Q, 'r' => egui::Key::R, 's' => egui::Key::S, 't' => egui::Key::T,
        'u' => egui::Key::U, 'v' => egui::Key::V, 'w' => egui::Key::W, 'x' => egui::Key::X,
        'y' => egui::Key::Y, 'z' => egui::Key::Z,
        '0' => egui::Key::Num0, '1' => egui::Key::Num1, '2' => egui::Key::Num2,
        '3' => egui::Key::Num3, '4' => egui::Key::Num4, '5' => egui::Key::Num5,
        '6' => egui::Key::Num6, '7' => egui::Key::Num7, '8' => egui::Key::Num8,
        '9' => egui::Key::Num9,
        ',' => egui::Key::Comma, '.' => egui::Key::Period, ';' => egui::Key::Semicolon,
        '/' => egui::Key::Slash,
        _ => return None,
    })
}

fn draw_presets(
    ui: &mut Ui,
    params: &SynthParams,
    setter: &ParamSetter,
    state: &mut EditorState,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preset:").strong());
        let current = PRESET_NAMES
            .get(state.selected_preset)
            .copied()
            .unwrap_or("<none>");
        egui::ComboBox::from_id_salt("preset_combo")
            .selected_text(current)
            .width(220.0)
            .show_ui(ui, |ui| {
                for (i, name) in PRESET_NAMES.iter().enumerate() {
                    if ui
                        .selectable_value(&mut state.selected_preset, i, *name)
                        .clicked()
                    {
                        apply_preset_with_setter(i, params, setter);
                    }
                }
            });
        if ui.button("Load").clicked() {
            apply_preset_with_setter(state.selected_preset, params, setter);
        }
    });
}

fn draw_oscillator(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    ui.label(RichText::new("Oscillator").strong());
    ui.horizontal(|ui| {
        ui.label("Wave");
        ui.add(widgets::ParamSlider::for_param(&params.waveform, setter));
    });
    ui.horizontal(|ui| {
        ui.label("Duty");
        ui.add(widgets::ParamSlider::for_param(&params.pulse_duty, setter));
        ui.label("Noise short:");
        ui.add(widgets::ParamSlider::for_param(&params.noise_short, setter));
    });
    ui.horizontal(|ui| {
        ui.label("FM ratio");
        ui.add(widgets::ParamSlider::for_param(&params.fm_ratio, setter));
        ui.label("FM index");
        ui.add(widgets::ParamSlider::for_param(&params.fm_index, setter));
    });
    ui.horizontal(|ui| {
        ui.label("Octave");
        ui.add(widgets::ParamSlider::for_param(&params.octave, setter));
        ui.label("Fine");
        ui.add(widgets::ParamSlider::for_param(&params.fine_tune, setter));
        ui.label("Gain");
        ui.add(widgets::ParamSlider::for_param(&params.gain, setter));
    });
}

fn draw_envelope(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    ui.label(RichText::new("Envelope (ADSR)").strong());
    ui.horizontal(|ui| {
        ui.label("A");
        ui.add(widgets::ParamSlider::for_param(&params.attack, setter));
        ui.label("D");
        ui.add(widgets::ParamSlider::for_param(&params.decay, setter));
        ui.label("S");
        ui.add(widgets::ParamSlider::for_param(&params.sustain, setter));
        ui.label("R");
        ui.add(widgets::ParamSlider::for_param(&params.release, setter));
    });
}

fn draw_modulation(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    ui.label(RichText::new("Vibrato / Sweep / Bitcrush").strong());
    ui.horizontal(|ui| {
        ui.label("Vib rate");
        ui.add(widgets::ParamSlider::for_param(&params.vibrato_rate, setter));
        ui.label("Vib depth");
        ui.add(widgets::ParamSlider::for_param(&params.vibrato_depth, setter));
        ui.label("Vib delay");
        ui.add(widgets::ParamSlider::for_param(&params.vibrato_delay, setter));
    });
    ui.horizontal(|ui| {
        ui.label("Sweep semi");
        ui.add(widgets::ParamSlider::for_param(&params.sweep_semi, setter));
        ui.label("Sweep time");
        ui.add(widgets::ParamSlider::for_param(&params.sweep_time, setter));
    });
    ui.horizontal(|ui| {
        ui.label("Bit depth");
        ui.add(widgets::ParamSlider::for_param(&params.bit_depth, setter));
        ui.label("Bit rate");
        ui.add(widgets::ParamSlider::for_param(&params.bit_rate, setter));
    });
}

fn draw_drums(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    ui.label(RichText::new("Drum Mode").strong());
    ui.horizontal(|ui| {
        ui.label("Enabled");
        ui.add(widgets::ParamSlider::for_param(&params.drum_mode, setter));
        ui.label("Pitch tracks note");
        ui.add(widgets::ParamSlider::for_param(&params.drum_pitch, setter));
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Drum slots (mapped from C2 upward):");
        for d in DrumKind::ALL {
            ui.add(egui::Label::new(
                RichText::new(format!("{}: {}", d as u8, d.label()))
                    .color(Color32::from_rgb(120, 200, 255)),
            ));
        }
    });
}

fn draw_keyboard_help(ui: &mut Ui, state: &EditorState) {
    ui.label(RichText::new("QWERTY Keyboard").strong());
    ui.label(format!(
        "Base note: MIDI {} ({})  •  PageUp / PageDown to shift octave",
        state.base_note,
        midi_to_name(state.base_note as u8)
    ));
    ui.label("Bottom row: Z S X D C V G B H N J M , L . ; /");
    ui.label("Top row:    Q 2 W 3 E R 5 T 6 Y 7 U I 9 O 0 P");

    // Simple visual keyboard.
    let (rect, _resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(540.0), 60.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let n_white = 14;
    let w = rect.width() / n_white as f32;
    let mut white_idx = 0;
    let white_steps = [0, 2, 4, 5, 7, 9, 11];
    for octave in 0..2 {
        for &s in &white_steps {
            let x = rect.min.x + white_idx as f32 * w;
            let key_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.min.y),
                egui::vec2(w - 1.0, rect.height()),
            );
            let semis = octave * 12 + s;
            let active = state.pressed[(state.base_note + semis).clamp(0, 127) as usize];
            let fill = if active { Color32::from_rgb(180, 240, 120) } else { Color32::WHITE };
            painter.rect_filled(key_rect, 1.0, fill);
            painter.rect_stroke(
                key_rect,
                1.0,
                Stroke::new(1.0, Color32::BLACK),
                egui::StrokeKind::Middle,
            );
            white_idx += 1;
        }
    }
    // Black keys overlay.
    let black_offsets = [1, 3, 6, 8, 10];
    let mut overall = 0;
    for octave in 0..2 {
        for (i, &s) in white_steps.iter().enumerate() {
            if i < 6 && black_offsets.contains(&(s + 1)) {
                let x = rect.min.x + (overall + 1) as f32 * w - w * 0.3;
                let key_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.min.y),
                    egui::vec2(w * 0.6, rect.height() * 0.6),
                );
                let semis = octave * 12 + s + 1;
                let active = state.pressed[(state.base_note + semis).clamp(0, 127) as usize];
                let fill = if active { Color32::from_rgb(120, 200, 80) } else { Color32::BLACK };
                painter.rect_filled(key_rect, 1.0, fill);
            }
            overall += 1;
        }
    }
}

fn midi_to_name(n: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (n / 12) as i32 - 1;
    format!("{}{}", NAMES[(n % 12) as usize], octave)
}
