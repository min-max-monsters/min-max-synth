//! Egui editor with a chiptune-flavoured layout: rotary knobs, LED toggles,
//! grouped panels, and an on-screen + QWERTY piano keyboard.

use crate::params::{LegatoMode, ModShapeChoice, ModTargetChoice, SynthParams, WaveChoice};
use crate::preset_bank::{
    Category, ParamSnapshot, PresetBank, PresetMeta, System, Voicing,
};
use crate::samples::DrumKind;
use crate::dsp::{Phoneme, NUM_PHONEMES};
use crate::g2p::text_to_phonemes;
use crate::widgets::{apply_style, led_toggle, palette, panel, Knob, VSlider};
use crate::GuiNoteEvent;
use crossbeam_queue::ArrayQueue;
use nih_plug::prelude::{Editor, ParamSetter};
use nih_plug_egui::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui};
use nih_plug_egui::{create_egui_editor, EguiState};
use nih_plug_egui::resizable_window::ResizableWindow;
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
    selected_drum: usize,
    // Preset browser
    pub bank: PresetBank,
    pub browser_open: bool,
    // Speech text input (G2P)
    pub speech_text: String,
    // Save dialog
    pub save_dialog_open: bool,
    pub save_name: String,
    pub save_system: System,
    pub save_category: Category,
}

impl Default for EditorState {
    fn default() -> Self {
        let mut bank = PresetBank::new();
        bank.load_user_presets();
        Self {
            pressed: [false; 128],
            base_note: 60,
            selected_drum: 0,
            bank,
            browser_open: false,
            speech_text: String::new(),
            save_dialog_open: false,
            save_name: String::new(),
            save_system: System::Generic,
            save_category: Category::Lead,
        }
    }
}

pub fn create_editor(
    params: Arc<SynthParams>,
    note_queue: Arc<ArrayQueue<GuiNoteEvent>>,
) -> Option<Box<dyn Editor>> {
    let egui_state: Arc<EguiState> = params.editor_state.clone();
    let egui_state_inner = egui_state.clone();
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
                .inner_margin(egui::Margin { left: 6, right: 6, top: 4, bottom: 4 });
            egui::TopBottomPanel::top("header_panel")
                .frame(header_frame)
                .show(ctx, |ui| {
                    draw_header(ui, &params, setter, state);
                });

            let kb_frame = egui::Frame::default()
                .fill(palette::BG_DEEP)
                .inner_margin(egui::Margin { left: 6, right: 6, top: 2, bottom: 4 });
            egui::TopBottomPanel::bottom("keyboard_panel")
                .frame(kb_frame)
                .show(ctx, |ui| {
                    draw_keyboard(ui, state);
                });

            // Preset browser side panel
            if state.browser_open {
                let browser_frame = egui::Frame::default()
                    .fill(palette::BG_MID)
                    .inner_margin(egui::Margin::same(6))
                    .stroke(Stroke::new(1.0, palette::BORDER));
                egui::SidePanel::left("preset_browser")
                    .frame(browser_frame)
                    .default_width(280.0)
                    .min_width(240.0)
                    .max_width(360.0)
                    .resizable(true)
                    .show(ctx, |ui| {
                        draw_preset_browser(ui, &params, setter, state);
                    });
            }

            // Save dialog window
            if state.save_dialog_open {
                draw_save_dialog(ctx, &params, setter, state);
            }

            let main_frame = egui::Frame::default()
                .fill(palette::BG_DEEP)
                .inner_margin(egui::Margin { left: 6, right: 6, top: 2, bottom: 2 });
            ResizableWindow::new("resize")
                .min_size([600.0, 400.0])
                .show(ctx, &egui_state_inner, |ui| {
                    main_frame.show(ui, |ui| {
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
            RichText::new(format!("min_max_synth v{}", env!("MIN_MAX_VERSION")))
                .color(palette::ACCENT)
                .size(14.0)
                .strong(),
        );
        ui.add_space(4.0);
        // Mode selector: SPEECH / DRUM / SYNTH
        {
            let is_drum = params.drum_mode.value();
            let is_speech = params.speech_mode.value();
            let mode = if is_speech { 2 } else if is_drum { 1 } else { 0 };
            let labels = ["SYNTH", "DRUM", "SPEECH"];
            for (i, label) in labels.iter().enumerate() {
                let sel = i == mode;
                let color = if sel { palette::ACCENT } else { palette::TEXT_DIM };
                let btn = egui::Button::new(
                    RichText::new(*label).color(color).size(10.0).strong(),
                )
                .fill(if sel { palette::BG_MID } else { Color32::TRANSPARENT })
                .stroke(Stroke::new(if sel { 1.0 } else { 0.5 }, if sel { palette::ACCENT } else { palette::BORDER }));
                if ui.add(btn).clicked() {
                    match i {
                        0 => {
                            setter.begin_set_parameter(&params.drum_mode);
                            setter.set_parameter(&params.drum_mode, false);
                            setter.end_set_parameter(&params.drum_mode);
                            setter.begin_set_parameter(&params.speech_mode);
                            setter.set_parameter(&params.speech_mode, false);
                            setter.end_set_parameter(&params.speech_mode);
                        }
                        1 => {
                            setter.begin_set_parameter(&params.drum_mode);
                            setter.set_parameter(&params.drum_mode, true);
                            setter.end_set_parameter(&params.drum_mode);
                            setter.begin_set_parameter(&params.speech_mode);
                            setter.set_parameter(&params.speech_mode, false);
                            setter.end_set_parameter(&params.speech_mode);
                        }
                        _ => {
                            setter.begin_set_parameter(&params.drum_mode);
                            setter.set_parameter(&params.drum_mode, false);
                            setter.end_set_parameter(&params.drum_mode);
                            setter.begin_set_parameter(&params.speech_mode);
                            setter.set_parameter(&params.speech_mode, true);
                            setter.end_set_parameter(&params.speech_mode);
                        }
                    }
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("💾").on_hover_text("Save preset").clicked() {
                state.save_dialog_open = !state.save_dialog_open;
            }
            let browse_label = if state.browser_open { "✕" } else { "☰" };
            if ui.button(browse_label).on_hover_text("Browser").clicked() {
                state.browser_open = !state.browser_open;
            }
            if ui.button("▶").on_hover_text("Next preset").clicked() {
                state.bank.next();
                apply_bank_preset(state, params, setter);
            }
            let name = state
                .bank
                .current_entry()
                .map(|e| format!("[{}] {}", e.meta.system.label(), e.name))
                .unwrap_or_else(|| "—".to_string());
            ui.label(
                RichText::new(&name)
                    .color(palette::TEXT)
                    .monospace()
                    .size(11.0),
            );
            if ui.button("◀").on_hover_text("Previous preset").clicked() {
                state.bank.prev();
                apply_bank_preset(state, params, setter);
            }
        });
    });
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().min.y,
        Stroke::new(1.0, palette::BORDER),
    );
}

fn draw_main(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter, state: &mut EditorState) {
    if params.speech_mode.value() {
        draw_speech_main(ui, params, setter, state);
    } else if params.drum_mode.value() {
        draw_drum_main(ui, params, setter, state);
    } else {
        draw_synth_main(ui, params, setter);
    }
}

fn draw_synth_main(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 3.0);
    ui.horizontal_top(|ui| {
        // Left column: oscillator + duty lfo + amp.
        ui.vertical(|ui| {
            panel(ui, "OSCILLATOR", palette::ACCENT, |ui| {
                draw_waveform_picker(ui, params, setter);
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.pulse_duty, setter).with_label("DUTY"));
                    ui.add(Knob::new(&params.fm_ratio, setter).with_label("FM RATIO"));
                    ui.add(Knob::new(&params.fm_index, setter).with_label("FM IDX"));
                });
                led_toggle(ui, &params.noise_short, setter, "Metallic noise");
            });
            ui.add_space(3.0);
            panel(ui, "DUTY LFO", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.duty_lfo_rate, setter).with_label("RATE"));
                    ui.add(Knob::new(&params.duty_lfo_depth, setter).with_label("DEPTH"));
                });
            });
            ui.add_space(3.0);
            panel(ui, "AMP", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.octave, setter).with_label("OCT"));
                    ui.add(Knob::new(&params.fine_tune, setter).with_label("FINE"));
                    ui.add(Knob::new(&params.gain, setter).with_label("GAIN").with_diameter(46.0));
                });
            });
        });

        ui.add_space(4.0);

        // Middle column: envelope + bitcrush + output filter.
        ui.vertical(|ui| {
            panel(ui, "ENVELOPE", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.attack, setter).with_label("A"));
                    ui.add(Knob::new(&params.decay, setter).with_label("D"));
                    ui.add(Knob::new(&params.sustain, setter).with_label("S"));
                    ui.add(Knob::new(&params.release, setter).with_label("R"));
                });
                draw_adsr_visual(ui, params);
            });
            ui.add_space(3.0);
            panel(ui, "BITCRUSH", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.bit_depth, setter).with_label("BITS"));
                    ui.add(Knob::new(&params.bit_rate, setter).with_label("RATE"));
                });
            });
            ui.add_space(3.0);
            panel(ui, "OUTPUT FILTER", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.lp_cutoff, setter).with_label("LP CUT"));
                    ui.add(Knob::new(&params.hp_cutoff, setter).with_label("HP CUT"));
                });
                draw_filter_system_buttons(ui, params, setter);
            });
        });

        ui.add_space(4.0);

        // Right column: vibrato + auto bend + mod env + mono/arp.
        ui.vertical(|ui| {
            panel(ui, "VIBRATO", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.vibrato_rate, setter).with_label("RATE"));
                    ui.add(Knob::new(&params.vibrato_depth, setter).with_label("DEPTH"));
                    ui.add(Knob::new(&params.vibrato_delay, setter).with_label("DELAY"));
                });
            });
            ui.add_space(3.0);
            panel(ui, "AUTO BEND", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.sweep_semi, setter).with_label("AMOUNT"));
                    ui.add(Knob::new(&params.sweep_time, setter).with_label("TIME"));
                });
            });
            ui.add_space(3.0);
            panel(ui, "MOD ENV", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.mod_amount, setter).with_label("AMOUNT"));
                    ui.add(Knob::new(&params.mod_delay, setter).with_label("DELAY"));
                    ui.add(Knob::new(&params.mod_time, setter).with_label("TIME"));
                });
                draw_mod_target_buttons(ui, params, setter);
                draw_mod_shape_buttons(ui, params, setter);
            });
            ui.add_space(3.0);
            panel(ui, "MONO / ARP", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    led_toggle(ui, &params.mono, setter, "Monophonic");
                    ui.add(Knob::new(&params.arp_rate, setter).with_label("ARP RATE"));
                    ui.add(Knob::new(&params.glide_time, setter).with_label("GLIDE"));
                });
                draw_legato_buttons(ui, params, setter);
            });
        });
    });
}

fn draw_speech_main(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter, state: &mut EditorState) {
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 3.0);
    ui.horizontal_top(|ui| {
        // Left column: phoneme selector + sequencer.
        ui.vertical(|ui| {
            panel(ui, "PHONEME", palette::RED, |ui| {
                let current = params.phoneme.value() as usize;
                let phon_row = |ui: &mut Ui, label: &str, range: std::ops::Range<usize>| {
                    ui.label(RichText::new(label).color(palette::TEXT_DIM).size(9.0));
                    ui.horizontal_wrapped(|ui| {
                        for i in range {
                            let p = Phoneme::from_index(i);
                            let sel = i == current;
                            let color = if sel { palette::ACCENT } else { palette::TEXT };
                            let btn = egui::Button::new(
                                RichText::new(p.label()).color(color).size(11.0).strong(),
                            )
                            .min_size(egui::vec2(30.0, 18.0))
                            .fill(if sel { palette::BG_MID } else { Color32::TRANSPARENT })
                            .stroke(Stroke::new(
                                if sel { 1.0 } else { 0.5 },
                                if sel { palette::ACCENT } else { palette::BORDER },
                            ));
                            if ui.add(btn).clicked() {
                                setter.begin_set_parameter(&params.phoneme);
                                setter.set_parameter(&params.phoneme, i as i32);
                                setter.end_set_parameter(&params.phoneme);
                            }
                        }
                    });
                };
                phon_row(ui, "Vowels", 0..10);
                phon_row(ui, "Nasals / Liquids", 10..14);
                phon_row(ui, "Fricatives / Dental", 14..19);
                phon_row(ui, "Stops / HH / TT", 19..26);
                phon_row(ui, "Diphthongs / Semivowels", 26..32);
                phon_row(ui, "NG / CH / TH / DH / Silence", 32..36);
            });
            ui.add_space(3.0);
            panel(ui, "SEQUENCER", palette::RED, |ui| {
                let seq_len = params.speech_seq_len.value() as usize;
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.speech_seq_len, setter).with_label("STEPS"));
                    ui.add(Knob::new(&params.speech_step_ms, setter).with_label("SPEED"));
                    led_toggle(ui, &params.speech_seq_loop, setter, "LOOP");
                });
                if seq_len > 0 {
                    ui.add_space(2.0);
                    ui.horizontal_wrapped(|ui| {
                        for i in 0..seq_len {
                            let phon_idx = params.sq(i).value() as usize;
                            let p = Phoneme::from_index(phon_idx);
                            let btn = egui::Button::new(
                                RichText::new(p.label()).color(palette::ACCENT).size(11.0).strong(),
                            )
                            .min_size(egui::vec2(30.0, 20.0))
                            .fill(palette::BG_MID)
                            .stroke(Stroke::new(1.0, palette::ACCENT));
                            if ui.add(btn).on_hover_text("Click to cycle phoneme").clicked() {
                                let next = (phon_idx + 1) % NUM_PHONEMES;
                                setter.begin_set_parameter(params.sq(i));
                                setter.set_parameter(params.sq(i), next as i32);
                                setter.end_set_parameter(params.sq(i));
                            }
                        }
                    });
                }
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    let words: &[(&str, &[usize])] = &[
                        ("MIN",      &[10, 2, 11]),
                        ("MAX",      &[10, 4, 22, 14]),
                        ("MONSTERS", &[10, 0, 11, 14, 25, 9, 17]),
                        ("HELLO",    &[24, 3, 12, 6]),
                        ("WORLD",    &[30, 9, 12, 20]),
                        ("YEAH",     &[31, 4]),
                        ("COOL",     &[22, 7, 12]),
                        ("ROBOT",    &[13, 6, 19, 0, 25]),
                        ("FIRE",     &[16, 26, 9]),
                        ("NO",       &[11, 6]),
                        ("YES",      &[31, 3, 14]),
                        ("BEEP",     &[19, 1, 1, 29]),
                    ];
                    for (label, phonemes) in words {
                        let btn = egui::Button::new(
                            RichText::new(*label).color(palette::TEXT).size(9.0),
                        )
                        .min_size(egui::vec2(36.0, 16.0))
                        .fill(palette::BG_DEEP);
                        if ui.add(btn).clicked() {
                            let len = phonemes.len().min(16);
                            setter.begin_set_parameter(&params.speech_seq_len);
                            setter.set_parameter(&params.speech_seq_len, len as i32);
                            setter.end_set_parameter(&params.speech_seq_len);
                            for (i, &ph) in phonemes.iter().take(16).enumerate() {
                                setter.begin_set_parameter(params.sq(i));
                                setter.set_parameter(params.sq(i), ph as i32);
                                setter.end_set_parameter(params.sq(i));
                            }
                        }
                    }
                });
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.speech_text)
                        .desired_width(240.0)
                        .hint_text("type word or [AH EE]")
                        .font(FontId::monospace(11.0))
                        .text_color(palette::ACCENT),
                );
                if response.changed() {
                    let phonemes = text_to_phonemes(&state.speech_text, 16);
                    let len = phonemes.len().min(16);
                    setter.begin_set_parameter(&params.speech_seq_len);
                    setter.set_parameter(&params.speech_seq_len, len as i32);
                    setter.end_set_parameter(&params.speech_seq_len);
                    for (i, &ph) in phonemes.iter().enumerate() {
                        setter.begin_set_parameter(params.sq(i));
                        setter.set_parameter(params.sq(i), ph as i32);
                        setter.end_set_parameter(params.sq(i));
                    }
                }
            });
        });

        ui.add_space(4.0);

        // Right column: voice controls + modulation.
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            panel(ui, "VOICE", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.speech_buzz, setter).with_label("BUZZ"));
                    ui.add(Knob::new(&params.gain, setter).with_label("GAIN"));
                    ui.add(Knob::new(&params.octave, setter).with_label("OCT"));
                    ui.add(Knob::new(&params.fine_tune, setter).with_label("FINE"));
                });
            });
            panel(ui, "ENVELOPE", palette::ACCENT, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.attack, setter).with_label("A"));
                    ui.add(Knob::new(&params.decay, setter).with_label("D"));
                    ui.add(Knob::new(&params.sustain, setter).with_label("S"));
                    ui.add(Knob::new(&params.release, setter).with_label("R"));
                });
            });
            panel(ui, "VIBRATO / MONO", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.vibrato_rate, setter).with_label("RATE"));
                    ui.add(Knob::new(&params.vibrato_depth, setter).with_label("DEPTH"));
                    ui.add_space(4.0);
                    led_toggle(ui, &params.mono, setter, "Mono");
                    ui.add(Knob::new(&params.arp_rate, setter).with_label("ARP"));
                    ui.add(Knob::new(&params.glide_time, setter).with_label("GLIDE"));
                });
            });
            panel(ui, "MOD ENV", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.mod_amount, setter).with_label("AMOUNT"));
                    ui.add(Knob::new(&params.mod_delay, setter).with_label("DELAY"));
                    ui.add(Knob::new(&params.mod_time, setter).with_label("TIME"));
                });
                draw_mod_target_buttons(ui, params, setter);
                draw_mod_shape_buttons(ui, params, setter);
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
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 3.0);
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

        ui.add_space(6.0);

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
            ui.add_space(3.0);
            panel(ui, "BUS", palette::BLUE, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.gain, setter).with_label("GAIN").with_diameter(46.0));
                    ui.add(Knob::new(&params.bit_depth, setter).with_label("BITS"));
                    ui.add(Knob::new(&params.bit_rate, setter).with_label("RATE"));
                });
                ui.horizontal(|ui| {
                    ui.add(Knob::new(&params.lp_cutoff, setter).with_label("LP CUT"));
                    ui.add(Knob::new(&params.hp_cutoff, setter).with_label("HP CUT"));
                });
                draw_filter_system_buttons(ui, params, setter);
            });
        });
    });
}

// ---------------------------------------------------------------------------
// Preset browser & save dialog
// ---------------------------------------------------------------------------

/// Apply the currently selected bank preset to the plugin params.
fn apply_bank_preset(state: &EditorState, params: &SynthParams, setter: &ParamSetter) {
    if let Some(entry) = state.bank.current_entry() {
        entry.snapshot.apply(params, setter);
    }
}

/// The left side-panel preset browser: filter buttons, search, scrollable list.
fn draw_preset_browser(
    ui: &mut Ui,
    params: &SynthParams,
    setter: &ParamSetter,
    state: &mut EditorState,
) {
    ui.label(RichText::new("PRESET BROWSER").color(palette::ACCENT).size(12.0).strong());
    ui.add_space(4.0);

    // Search bar
    let search_response = ui.add(
        egui::TextEdit::singleline(&mut state.bank.search_text)
            .hint_text("Search…")
            .desired_width(f32::INFINITY)
            .text_color(palette::TEXT),
    );
    if search_response.changed() {
        state.bank.refilter();
    }
    ui.add_space(4.0);

    // System filter
    ui.label(RichText::new("SYSTEM").color(palette::TEXT_DIM).size(9.0));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);
        ui.spacing_mut().button_padding = egui::vec2(3.0, 1.0);
        // "All" button
        let all_sel = state.bank.filter_system.is_none();
        if tag_button(ui, "ALL", all_sel).clicked() {
            state.bank.filter_system = None;
            state.bank.refilter();
        }
        for &sys in System::ALL {
            let sel = state.bank.filter_system == Some(sys);
            if tag_button(ui, sys.label(), sel).clicked() {
                state.bank.filter_system = if sel { None } else { Some(sys) };
                state.bank.refilter();
            }
        }
    });
    ui.add_space(2.0);

    // Category filter
    ui.label(RichText::new("CATEGORY").color(palette::TEXT_DIM).size(9.0));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);
        ui.spacing_mut().button_padding = egui::vec2(3.0, 1.0);
        let all_sel = state.bank.filter_category.is_none();
        if tag_button(ui, "ALL", all_sel).clicked() {
            state.bank.filter_category = None;
            state.bank.refilter();
        }
        for &cat in Category::ALL {
            let sel = state.bank.filter_category == Some(cat);
            if tag_button(ui, cat.label(), sel).clicked() {
                state.bank.filter_category = if sel { None } else { Some(cat) };
                state.bank.refilter();
            }
        }
    });
    ui.add_space(2.0);

    // Voicing filter
    ui.label(RichText::new("VOICING").color(palette::TEXT_DIM).size(9.0));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);
        ui.spacing_mut().button_padding = egui::vec2(3.0, 1.0);
        let all_sel = state.bank.filter_voicing.is_none();
        if tag_button(ui, "ALL", all_sel).clicked() {
            state.bank.filter_voicing = None;
            state.bank.refilter();
        }
        for &voi in Voicing::ALL {
            let sel = state.bank.filter_voicing == Some(voi);
            if tag_button(ui, voi.label(), sel).clicked() {
                state.bank.filter_voicing = if sel { None } else { Some(voi) };
                state.bank.refilter();
            }
        }
    });
    ui.add_space(2.0);

    // Source filter (factory / user)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if tag_button(ui, "FACTORY", state.bank.show_factory).clicked() {
            state.bank.show_factory = !state.bank.show_factory;
            state.bank.refilter();
        }
        if tag_button(ui, "USER", state.bank.show_user).clicked() {
            state.bank.show_user = !state.bank.show_user;
            state.bank.refilter();
        }
        ui.label(
            RichText::new(format!("{} presets", state.bank.filtered.len()))
                .color(palette::TEXT_DIM)
                .size(9.0),
        );
    });

    ui.add_space(4.0);
    ui.separator();

    // Scrollable preset list
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut clicked_idx: Option<usize> = None;
            let mut delete_idx: Option<usize> = None;
            // Collect what we need to avoid borrow conflicts.
            let items: Vec<(usize, usize, String, bool, bool)> = state
                .bank
                .filtered
                .iter()
                .enumerate()
                .map(|(fi, &ei)| {
                    let e = &state.bank.entries[ei];
                    let sel = fi == state.bank.selected;
                    let display_name = format!("[{}] {}", e.meta.system.label(), e.name);
                    (fi, ei, display_name, e.is_factory, sel)
                })
                .collect();
            let avail_w = ui.available_width();
            for (fi, entry_idx, name, is_factory, selected) in &items {
                let bg = if *selected { palette::ACCENT_DIM } else { Color32::TRANSPARENT };
                let fg = if *selected {
                    palette::ACCENT
                } else if *is_factory {
                    palette::TEXT
                } else {
                    Color32::from_rgb(180, 200, 255)
                };
                let label_text = if *is_factory {
                    name.clone()
                } else {
                    format!("⬡ {}", name)
                };
                let resp = ui.add(
                    egui::Button::new(
                        RichText::new(&label_text).color(fg).monospace().size(10.0),
                    )
                    .fill(bg)
                    .stroke(Stroke::NONE)
                    .min_size(egui::vec2(avail_w, 18.0)),
                );
                if resp.clicked() {
                    clicked_idx = Some(*fi);
                }
                if !is_factory {
                    resp.context_menu(|ui| {
                        if ui.button("Delete preset").clicked() {
                            delete_idx = Some(*entry_idx);
                            ui.close_menu();
                        }
                    });
                }
            }
            if let Some(fi) = clicked_idx {
                state.bank.selected = fi;
                apply_bank_preset(state, params, setter);
            }
            if let Some(ei) = delete_idx {
                state.bank.delete_user_preset(ei);
            }
        });
}

/// Draw the save-preset dialog as a floating egui window.
fn draw_save_dialog(
    ctx: &egui::Context,
    params: &SynthParams,
    _setter: &ParamSetter,
    state: &mut EditorState,
) {
    let mut open = state.save_dialog_open;
    egui::Window::new("Save Preset")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.save_name)
                        .desired_width(200.0)
                        .text_color(palette::TEXT),
                );
            });
            ui.add_space(4.0);

            // System picker
            ui.horizontal(|ui| {
                ui.label(RichText::new("System:").size(11.0));
                for &sys in System::ALL {
                    let sel = state.save_system == sys;
                    if tag_button(ui, sys.label(), sel).clicked() {
                        state.save_system = sys;
                    }
                }
            });

            // Category picker
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Category:").size(11.0));
                for &cat in Category::ALL {
                    let sel = state.save_category == cat;
                    if tag_button(ui, cat.label(), sel).clicked() {
                        state.save_category = cat;
                    }
                }
            });

            // Voicing picker removed — inferred from params automatically.

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let name_ok = !state.save_name.trim().is_empty();
                if ui
                    .add_enabled(name_ok, egui::Button::new("Save"))
                    .clicked()
                {
                    let snap = ParamSnapshot::capture(params);
                    let meta = PresetMeta {
                        system: state.save_system,
                        category: state.save_category,
                        voicing: snap.infer_voicing(),
                    };
                    state
                        .bank
                        .save_user_preset(state.save_name.trim(), meta, snap);
                    state.save_dialog_open = false;
                }
                if ui.button("Cancel").clicked() {
                    state.save_dialog_open = false;
                }
            });
        });
    state.save_dialog_open = open && state.save_dialog_open;
}

/// Reusable tag filter button.
fn tag_button(ui: &mut Ui, label: &str, selected: bool) -> egui::Response {
    let bg = if selected { palette::ACCENT_DIM } else { palette::BG_PANEL_HI };
    let fg = if selected { palette::ACCENT } else { palette::TEXT_DIM };
    ui.add(
        egui::Button::new(RichText::new(label).color(fg).monospace().size(9.0))
            .fill(bg)
            .stroke(Stroke::new(1.0, palette::BORDER))
            .min_size(egui::vec2(0.0, 14.0)),
    )
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

/// Three-way Retrigger / Legato / Glide selector for mono mode.
fn draw_legato_buttons(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    let choices = [
        (LegatoMode::Retrigger, "RETRIG"),
        (LegatoMode::Legato, "LEGATO"),
        (LegatoMode::Glide, "GLIDE"),
    ];
    let current = params.legato_mode.value();
    ui.horizontal(|ui| {
        for (variant, label) in choices {
            let selected = current == variant;
            let bg = if selected { palette::ACCENT_DIM } else { palette::BG_PANEL_HI };
            let fg = if selected { palette::ACCENT } else { palette::TEXT_DIM };
            let btn = egui::Button::new(RichText::new(label).color(fg).monospace().size(9.0))
                .fill(bg)
                .stroke(Stroke::new(1.0, palette::BORDER))
                .min_size(egui::vec2(54.0, 18.0));
            if ui.add(btn).clicked() {
                setter.begin_set_parameter(&params.legato_mode);
                setter.set_parameter(&params.legato_mode, variant);
                setter.end_set_parameter(&params.legato_mode);
            }
        }
    });
}

fn draw_mod_target_buttons(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    let choices = [
        (ModTargetChoice::Pitch, "PITCH"),
        (ModTargetChoice::Duty, "DUTY"),
        (ModTargetChoice::FmIndex, "FM IDX"),
    ];
    let current = params.mod_target.value();
    ui.horizontal(|ui| {
        ui.label(RichText::new("TGT").monospace().size(9.0).color(palette::TEXT_DIM));
        for (variant, label) in choices {
            let selected = current == variant;
            let bg = if selected { palette::ACCENT_DIM } else { palette::BG_PANEL_HI };
            let fg = if selected { palette::ACCENT } else { palette::TEXT_DIM };
            let btn = egui::Button::new(RichText::new(label).color(fg).monospace().size(9.0))
                .fill(bg)
                .stroke(Stroke::new(1.0, palette::BORDER))
                .min_size(egui::vec2(48.0, 18.0));
            if ui.add(btn).clicked() {
                setter.begin_set_parameter(&params.mod_target);
                setter.set_parameter(&params.mod_target, variant);
                setter.end_set_parameter(&params.mod_target);
            }
        }
    });
}

fn draw_mod_shape_buttons(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    let choices = [
        (ModShapeChoice::Step, "STEP"),
        (ModShapeChoice::Linear, "LINEAR"),
    ];
    let current = params.mod_shape.value();
    ui.horizontal(|ui| {
        ui.label(RichText::new("SHP").monospace().size(9.0).color(palette::TEXT_DIM));
        for (variant, label) in choices {
            let selected = current == variant;
            let bg = if selected { palette::ACCENT_DIM } else { palette::BG_PANEL_HI };
            let fg = if selected { palette::ACCENT } else { palette::TEXT_DIM };
            let btn = egui::Button::new(RichText::new(label).color(fg).monospace().size(9.0))
                .fill(bg)
                .stroke(Stroke::new(1.0, palette::BORDER))
                .min_size(egui::vec2(54.0, 18.0));
            if ui.add(btn).clicked() {
                setter.begin_set_parameter(&params.mod_shape);
                setter.set_parameter(&params.mod_shape, variant);
                setter.end_set_parameter(&params.mod_shape);
            }
        }
    });
}

/// Small buttons that set LP + HP cutoffs to hardware-authentic values for a
/// given retro system.  The currently-matching system (if any) is highlighted.
fn draw_filter_system_buttons(ui: &mut Ui, params: &SynthParams, setter: &ParamSetter) {
    // (label, lp_hz, hp_hz)
    const SYSTEMS: &[(&str, f32, f32)] = &[
        ("NES",   14_000.0, 37.0),
        ("GB",     8_000.0, 20.0),
        ("SNES",  16_000.0, 15.0),
        ("GEN",   20_000.0, 10.0),
        ("SID",   18_000.0, 15.0),
        ("AMIGA",  7_000.0, 30.0),
        ("PCSPK",  5_000.0, 100.0),
        ("2600",   6_000.0, 40.0),
        ("MSX",   12_000.0, 20.0),
        ("ZX",    10_000.0, 50.0),
        ("ARC",   16_000.0, 20.0),
        ("OFF",   20_000.0,  0.0),
    ];
    let cur_lp = params.lp_cutoff.value();
    let cur_hp = params.hp_cutoff.value();
    let draw_row = |ui: &mut Ui, systems: &[(&str, f32, f32)]| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(1.0, 0.0);
            ui.spacing_mut().button_padding = egui::vec2(3.0, 1.0);
            for &(label, lp, hp) in systems {
                let selected = (cur_lp - lp).abs() < 1.0 && (cur_hp - hp).abs() < 1.0;
                let bg = if selected { palette::ACCENT_DIM } else { palette::BG_PANEL_HI };
                let fg = if selected { palette::ACCENT } else { palette::TEXT_DIM };
                let btn = egui::Button::new(RichText::new(label).color(fg).monospace().size(8.0))
                    .fill(bg)
                    .stroke(Stroke::new(0.5, palette::BORDER))
                    .min_size(egui::vec2(0.0, 13.0));
                if ui.add(btn).clicked() {
                    setter.begin_set_parameter(&params.lp_cutoff);
                    setter.set_parameter(&params.lp_cutoff, lp);
                    setter.end_set_parameter(&params.lp_cutoff);
                    setter.begin_set_parameter(&params.hp_cutoff);
                    setter.set_parameter(&params.hp_cutoff, hp);
                    setter.end_set_parameter(&params.hp_cutoff);
                }
            }
        });
    };
    draw_row(ui, &SYSTEMS[..6]);
    draw_row(ui, &SYSTEMS[6..]);
}

fn draw_adsr_visual(ui: &mut Ui, params: &SynthParams) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(200.0, 50.0), Sense::hover());
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
        ui.add_space(2.0);
        draw_piano(ui, state);
    });
}

fn draw_piano(ui: &mut Ui, state: &EditorState) {
    let n_white = 21; // 3 octaves
    let height = 60.0;
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
    // Don't send MIDI when a text field has keyboard focus.
    if ctx.wants_keyboard_input() {
        // Release any currently held notes so they don't stick.
        for n in 0..128 {
            if state.pressed[n] {
                let _ = note_queue.push(GuiNoteEvent::Off { note: n as u8 });
                state.pressed[n] = false;
            }
        }
        return;
    }
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
