//! Egui editor with a chiptune-flavoured layout: rotary knobs, LED toggles,
//! grouped panels, and an on-screen + QWERTY piano keyboard.

use crate::params::{SynthParams, WaveChoice};
use crate::presets::{apply_preset_with_setter, PRESET_NAMES};
use crate::samples::DrumKind;
use crate::widgets::{apply_style, led_toggle, palette, panel, Knob, VSlider};
use crate::GuiNoteEvent;
use crossbeam_queue::ArrayQueue;
use nih_plug::prelude::{Editor, ParamSetter};
use nih_plug_egui::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui};
use nih_plug_egui::{create_egui_editor, EguiState};
use std::sync::Arc;

/// QWERTY-to-MIDI mapping like a tracker / many soft synths.
const BOTTOM_ROW: &[(char, i32)] = &[
    ('z', 0), ('s', 1), ('x', 2), ('d', 3), ('c', 4), ('v', 5), ('g', 6),
    ('b', 7), ('h', 8), ('n', 9), ('j', 10), ('m', 11),
    (',', 12), ('l', 13), ('.', 14), (';', 15), ('/', 16),
];
const TOP_ROW: &[(char, i32)] = &[
    ('q', 12), ('2', 13), ('w', 14), ('3', 15), ('e', 16), ('r', 17), ('5', 18),
    ('t', 19), ('6', 20), ('y', 21), ('7', 22), ('u', 23), ('i', 24), ('9', 25),
    ('o', 26), ('0', 27), ('p', 28),
];

pub struct EditorState {
    pressed: [bool; 128],
    base_note: i32,
    selected_preset: usize,
    selected_drum: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self { pressed: [false; 128], base_note: 60, selected_preset: 0, selected_drum: 0 }
    }
}

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
            apply_style(ctx);
            ctx.request_repaint();
            handle_keyboard(ctx, state, &note_queue);

            let header_frame = egui::Frame::default()
                .fill(palette::BG_DEEP)
                .inner_margin(egui::Margin { left: 10, right: 10, top: 8, bottom: 6 });
            egui::TopBottomPanel::top("header_panel")
                .frame(header_frame)
                .show(ctx, |ui| {
                    draw_header(ui, &params, setter, state);
                });

            let kb_frame = egui::Frame::default()
                .fill(palette::BG_DEEP)
                .inner_margin(egui::Margin { left: 10, right: 10, top: 4, bottom: 8 });
            egui::TopBottomPanel::bottom("keyboard_panel")
                .frame(kb_frame)
                .show(ctx, |ui| {
                    draw_keyboard(ui, state);
                });

            let main_frame = egui::Frame::default()
                .fill(palette::BG_DEEP)
                .inner_margin(egui::Margin { left: 10, right: 10, top: 4, bottom: 4 });
            egui::CentralPanel::default()
                .frame(main_frame)
                .show(ctx, |ui| {
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            draw_main(ui, &params, setter, state);
                        });
                });
        },
    )
}

fn draw_header(
    ui: &mut Ui,
    params: &SynthParams,
    setter: &ParamSetter,
    state: &mut EditorState,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("min_max_synth")
                .color(palette::ACCENT)
                .size(20.0)
                .strong(),
        );
        ui.label(
            RichText::new("·  retro chiptune voice")
                .color(palette::TEXT_DIM)
                .size(12.0),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Load").on_hover_text("Re-apply selected preset").clicked() {
                apply_preset_with_setter(state.selected_preset, params, setter);
            }
            let current = PRESET_NAMES.get(state.selected_preset).copied().unwrap_or("—");
            egui::ComboBox::from_id_salt("preset_combo")
                .selected_text(current)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for (i, name) in PRESET_NAMES.iter().enumerate() {
                        if ui.selectable_value(&mut state.selected_preset, i, *name).clicked() {
                            apply_preset_with_setter(i, params, setter);
                        }
                    }
                });
            ui.label(RichText::new("PRESET").color(palette::TEXT_DIM).size(11.0));
            ui.add_space(12.0);
            led_toggle(ui, &params.drum_mode, setter, "DRUM MODE");
        });
    });
    ui.add_space(2.0);
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().min.y,
        Stroke::new(1.0, palette::BORDER),
    );
}

fn draw_main(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter, state: &mut EditorState) {
    if params.drum_mode.value() {
        draw_drum_main(ui, params, setter, state);
    } else {
        draw_synth_main(ui, params, setter);
    }
}

fn draw_synth_main(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    ui.horizontal_top(|ui| {
        // Left column: oscillator + amp.
        ui.vertical(|ui| {
            panel(ui, "OSCILLATOR", palette::ACCENT, |ui| {
                draw_waveform_picker(ui, params, setter);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.pulse_duty, setter).with_label("DUTY"));
                    ui.add(Knob::new(&params.fm_ratio, setter).with_label("FM RATIO"));
                    ui.add(Knob::new(&params.fm_index, setter).with_label("FM IDX"));
                });
                ui.add_space(2.0);
                led_toggle(ui, &params.noise_short, setter, "Metallic noise");
            });
            ui.add_space(6.0);
            panel(ui, "DUTY LFO", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.duty_lfo_rate, setter).with_label("RATE"));
                    ui.add(Knob::new(&params.duty_lfo_depth, setter).with_label("DEPTH"));
                });
            });
            ui.add_space(6.0);
            panel(ui, "AMP", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.octave, setter).with_label("OCT"));
                    ui.add(Knob::new(&params.fine_tune, setter).with_label("FINE"));
                    ui.add(Knob::new(&params.gain, setter).with_label("GAIN").with_diameter(54.0));
                });
            });
        });

        ui.add_space(8.0);

        // Middle column: envelope.
        ui.vertical(|ui| {
            panel(ui, "ENVELOPE", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.attack, setter).with_label("A"));
                    ui.add(Knob::new(&params.decay, setter).with_label("D"));
                    ui.add(Knob::new(&params.sustain, setter).with_label("S"));
                    ui.add(Knob::new(&params.release, setter).with_label("R"));
                });
                ui.add_space(2.0);
                draw_adsr_visual(ui, params);
            });
        });

        ui.add_space(8.0);

        // Right column: modulation + bitcrush.
        ui.vertical(|ui| {
            panel(ui, "VIBRATO", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.vibrato_rate, setter).with_label("RATE"));
                    ui.add(Knob::new(&params.vibrato_depth, setter).with_label("DEPTH"));
                    ui.add(Knob::new(&params.vibrato_delay, setter).with_label("DELAY"));
                });
            });
            ui.add_space(6.0);
            panel(ui, "AUTO BEND", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.sweep_semi, setter).with_label("AMOUNT"));
                    ui.add(Knob::new(&params.sweep_time, setter).with_label("TIME"));
                });
            });
            ui.add_space(6.0);
            panel(ui, "MONO / ARP", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    led_toggle(ui, &params.mono, setter, "Monophonic");
                    ui.add(Knob::new(&params.arp_rate, setter).with_label("ARP RATE"));
                });
            });
            ui.add_space(6.0);
            panel(ui, "BITCRUSH", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.bit_depth, setter).with_label("BITS"));
                    ui.add(Knob::new(&params.bit_rate, setter).with_label("RATE"));
                });
            });
        });
    });
}

fn draw_drum_main(
    ui: &mut Ui,
    params: &SynthParams,
    setter: &ParamSetter,
    state: &mut EditorState,
) {
    ui.horizontal_top(|ui| {
        // Left: 8-channel mixer. Each strip is a vertical level fader plus a
        // selector button below it. The selected strip is highlighted with the
        // accent colour.
        ui.vertical(|ui| {
            panel(ui, "DRUM MIXER", palette::RED, |ui| {
                ui.horizontal(|ui| {
                    led_toggle(ui, &params.drum_pitch, setter, "Pitch tracks key");
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for (i, d) in DrumKind::ALL.iter().enumerate() {
                        let selected = state.selected_drum == i;
                        ui.vertical(|ui| {
                            let accent = if selected { palette::ACCENT } else { palette::ACCENT_DIM };
                            ui.add(
                                VSlider::new(params.d_level(i), setter)
                                    .with_label(d.label())
                                    .with_height(140.0)
                                    .with_accent(accent),
                            );
                            let bg = if selected { palette::ACCENT_DIM } else { palette::BG_PANEL_HI };
                            let fg = if selected { palette::ACCENT } else { palette::TEXT_DIM };
                            let btn = egui::Button::new(
                                RichText::new(format!("{}", i + 1))
                                    .color(fg)
                                    .monospace()
                                    .size(11.0),
                            )
                            .fill(bg)
                            .stroke(Stroke::new(1.0, palette::BORDER))
                            .min_size(egui::vec2(36.0, 18.0));
                            if ui.add(btn).clicked() {
                                state.selected_drum = i;
                            }
                        });
                        ui.add_space(2.0);
                    }
                });
                ui.add_space(2.0);
                draw_drum_legend(ui);
            });
        });

        ui.add_space(8.0);

        // Right: bespoke synth controls for the selected drum + bus FX.
        ui.vertical(|ui| {
            let i = state.selected_drum;
            let title = format!(
                "DRUM {} · {}",
                i + 1,
                DrumKind::ALL[i].label()
            );
            panel(ui, &title, palette::ACCENT, |ui| {
                draw_drum_wave_picker(ui, params, setter, i);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add(Knob::new(params.d_freq(i), setter).with_label("FREQ"));
                    ui.add(Knob::new(params.d_ratio(i), setter).with_label("RATIO"));
                    ui.add(Knob::new(params.d_noise(i), setter).with_label("NOISE"));
                    ui.add(Knob::new(params.d_burst(i), setter).with_label("BURST"));
                });
                ui.horizontal(|ui| {
                    ui.add(Knob::new(params.d_pitch_env(i), setter).with_label("P.ENV"));
                    ui.add(Knob::new(params.d_pitch_time(i), setter).with_label("P.TIME"));
                    ui.add(Knob::new(params.d_decay(i), setter).with_label("DECAY"));
                });
            });
            ui.add_space(6.0);
            panel(ui, "BUS", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.gain, setter).with_label("GAIN").with_diameter(54.0));
                    ui.add(Knob::new(&params.bit_depth, setter).with_label("BITS"));
                    ui.add(Knob::new(&params.bit_rate, setter).with_label("RATE"));
                });
            });
        });
    });
}

fn draw_waveform_picker(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    use WaveChoice::*;
    let choices = [
        (Pulse, "PULSE"),
        (Triangle, "TRI"),
        (Wave, "WAVE"),
        (Noise, "NOISE"),
        (Fm, "FM"),
        (Saw, "SAW"),
    ];
    let current = params.waveform.value();
    ui.horizontal_wrapped(|ui| {
        for (variant, label) in choices {
            let selected = current == variant;
            let bg = if selected { palette::ACCENT_DIM } else { palette::BG_PANEL_HI };
            let fg = if selected { palette::ACCENT } else { palette::TEXT_DIM };
            let btn = egui::Button::new(RichText::new(label).color(fg).monospace())
                .fill(bg)
                .stroke(Stroke::new(1.0, palette::BORDER))
                .min_size(egui::vec2(48.0, 22.0));
            if ui.add(btn).clicked() {
                setter.begin_set_parameter(&params.waveform);
                setter.set_parameter(&params.waveform, variant);
                setter.end_set_parameter(&params.waveform);
            }
        }
    });
}

fn draw_adsr_visual(ui: &mut Ui, params: &SynthParams) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(260.0, 70.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, palette::BG_DEEP);
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0, palette::BORDER), egui::StrokeKind::Inside);

    // Time-shape: A,D,Hold,R. Use parameter values, log-scale times into pixels.
    let a = params.attack.value().clamp(0.0, 5.0);
    let d = params.decay.value().clamp(0.0, 5.0);
    let s = params.sustain.value().clamp(0.0, 1.0);
    let r = params.release.value().clamp(0.0, 5.0);
    let total = (a + d + 0.5 + r).max(0.001);
    let w = rect.width() - 4.0;
    let h = rect.height() - 6.0;
    let x0 = rect.min.x + 2.0;
    let y0 = rect.max.y - 3.0;
    let yp = |level: f32| y0 - h * level.clamp(0.0, 1.0);
    let xa = x0 + w * (a / total);
    let xd = xa + w * (d / total);
    let xh = xd + w * (0.5 / total);
    let xr = rect.max.x - 2.0;

    let pts = vec![
        Pos2::new(x0, y0),
        Pos2::new(xa, yp(1.0)),
        Pos2::new(xd, yp(s)),
        Pos2::new(xh, yp(s)),
        Pos2::new(xr, y0),
    ];
    // Filled area under the curve.
    let mut fill_pts = pts.clone();
    fill_pts.push(Pos2::new(xr, y0));
    fill_pts.push(Pos2::new(x0, y0));
    painter.add(egui::Shape::convex_polygon(
        fill_pts,
        Color32::from_rgba_unmultiplied(180, 240, 120, 30),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::line(pts, Stroke::new(1.5, palette::ACCENT)));
}

fn draw_drum_wave_picker(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter, i: usize) {
    let labels = ["OFF", "SINE", "TRI", "SQR"];
    let param = params.d_wave(i);
    let current = param.value();
    ui.horizontal(|ui| {
        for (v, label) in labels.iter().enumerate() {
            let v = v as i32;
            let selected = current == v;
            let bg = if selected { palette::ACCENT_DIM } else { palette::BG_PANEL_HI };
            let fg = if selected { palette::ACCENT } else { palette::TEXT_DIM };
            let btn = egui::Button::new(RichText::new(*label).color(fg).monospace().size(11.0))
                .fill(bg)
                .stroke(Stroke::new(1.0, palette::BORDER))
                .min_size(egui::vec2(48.0, 20.0));
            if ui.add(btn).clicked() {
                setter.begin_set_parameter(param);
                setter.set_parameter(param, v);
                setter.end_set_parameter(param);
            }
        }
    });
}

fn draw_drum_legend(ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("Mapped from C2:")
                .color(palette::TEXT_DIM)
                .size(11.0),
        );
        for (i, d) in DrumKind::ALL.iter().enumerate() {
            ui.label(
                RichText::new(format!("{}·{}", i, d.label()))
                    .color(palette::BLUE)
                    .monospace()
                    .size(11.0),
            );
        }
    });
}

fn draw_keyboard(ui: &mut Ui, state: &EditorState) {
    panel(ui, "KEYBOARD", palette::BLUE, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "Base {} ({})",
                    state.base_note,
                    midi_to_name(state.base_note as u8)
                ))
                .color(palette::TEXT)
                .monospace(),
            );
            ui.label(
                RichText::new("•  PageUp / PageDown shift octave  •  Z…/  Q…P  for keys")
                    .color(palette::TEXT_DIM)
                    .size(11.0),
            );
        });
        ui.add_space(4.0);
        draw_piano(ui, state);
    });
}

fn draw_piano(ui: &mut Ui, state: &EditorState) {
    let n_white = 21; // 3 octaves
    let height = 80.0;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    let key_w = rect.width() / n_white as f32;
    // White keys.
    let white_steps = [0, 2, 4, 5, 7, 9, 11];
    let mut white_idx = 0;
    for octave in 0..3 {
        for &s in &white_steps {
            let semis = octave * 12 + s;
            let active = state.pressed[(state.base_note + semis).clamp(0, 127) as usize];
            let x = rect.min.x + white_idx as f32 * key_w;
            let r = Rect::from_min_size(Pos2::new(x, rect.min.y), egui::vec2(key_w - 1.0, height));
            let fill = if active { palette::ACCENT } else { Color32::from_rgb(235, 235, 235) };
            painter.rect_filled(r, 1.0, fill);
            painter.rect_stroke(r, 1.0, Stroke::new(1.0, palette::BORDER), egui::StrokeKind::Inside);
            // Note label (only C's).
            if s == 0 {
                let octv = (state.base_note + semis) / 12 - 1;
                painter.text(
                    Pos2::new(r.center().x, r.max.y - 8.0),
                    Align2::CENTER_BOTTOM,
                    format!("C{}", octv),
                    FontId::monospace(9.0),
                    Color32::from_rgb(120, 120, 120),
                );
            }
            white_idx += 1;
        }
    }
    // Black keys overlay.
    let mut white_idx = 0;
    for octave in 0..3 {
        for (i, &s) in white_steps.iter().enumerate() {
            // A black key sits to the upper right of this white key when this is C/D/F/G/A.
            let has_black = matches!(i, 0 | 1 | 3 | 4 | 5);
            if has_black && white_idx + 1 < n_white {
                let x = rect.min.x + (white_idx + 1) as f32 * key_w - key_w * 0.32;
                let r = Rect::from_min_size(
                    Pos2::new(x, rect.min.y),
                    egui::vec2(key_w * 0.64, height * 0.6),
                );
                let semis = octave * 12 + s + 1;
                let active = state.pressed[(state.base_note + semis).clamp(0, 127) as usize];
                let fill = if active { palette::ACCENT_DIM } else { Color32::from_rgb(20, 20, 20) };
                painter.rect_filled(r, 1.0, fill);
                painter.rect_stroke(r, 1.0, Stroke::new(1.0, palette::BORDER), egui::StrokeKind::Inside);
            }
            white_idx += 1;
        }
    }
}

fn handle_keyboard(
    ctx: &egui::Context,
    state: &mut EditorState,
    note_queue: &Arc<ArrayQueue<GuiNoteEvent>>,
) {
    ctx.input(|i| {
        if i.key_pressed(egui::Key::PageUp) {
            state.base_note = (state.base_note + 12).min(108);
        }
        if i.key_pressed(egui::Key::PageDown) {
            state.base_note = (state.base_note - 12).max(12);
        }

        let mut held = [false; 128];
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

fn midi_to_name(n: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (n / 12) as i32 - 1;
    format!("{}{}", NAMES[(n % 12) as usize], octave)
}
