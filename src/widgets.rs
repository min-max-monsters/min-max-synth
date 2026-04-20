//! Custom egui widgets: a chiptune-flavoured rotary knob, an LED toggle and a
//! grouped panel.

use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    self, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};
use std::f32::consts::TAU;

/// Color palette for the retro UI.
pub mod palette {
    use nih_plug_egui::egui::Color32;
    pub const BG_DEEP: Color32 = Color32::from_rgb(14, 18, 22);
    pub const BG_PANEL: Color32 = Color32::from_rgb(24, 30, 38);
    pub const BG_PANEL_HI: Color32 = Color32::from_rgb(34, 42, 52);
    pub const BORDER: Color32 = Color32::from_rgb(60, 76, 92);
    pub const ACCENT: Color32 = Color32::from_rgb(180, 240, 120);
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(80, 120, 60);
    pub const TRACK: Color32 = Color32::from_rgb(46, 56, 68);
    pub const TEXT: Color32 = Color32::from_rgb(220, 230, 235);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(150, 165, 175);
    pub const RED: Color32 = Color32::from_rgb(230, 80, 80);
    pub const BLUE: Color32 = Color32::from_rgb(120, 200, 255);
}

/// Vertical pixels of mouse drag for a full 0..1 sweep.
const DRAG_PIXELS_FOR_FULL_RANGE: f32 = 200.0;
/// Same, but while shift is held for finer adjustment.
const DRAG_PIXELS_FOR_FULL_RANGE_FINE: f32 = 1500.0;

/// Sweep arc start/end angles, in radians, measured clockwise from "up".
/// 0.0 = straight down (we draw it that way for a knob with the gap at the bottom).
const ARC_START_DEG: f32 = 135.0;
const ARC_END_DEG: f32 = 405.0; // wraps past 360 so 0..1 covers 270 degrees

/// A circular knob bound to a NIH-plug parameter.
pub struct Knob<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,
    label: Option<&'a str>,
    diameter: f32,
}

impl<'a, P: Param> Knob<'a, P> {
    pub fn new(param: &'a P, setter: &'a ParamSetter<'a>) -> Self {
        Self { param, setter, label: None, diameter: 44.0 }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_diameter(mut self, d: f32) -> Self {
        self.diameter = d;
        self
    }

    fn set_normalized(&self, n: f32) {
        let n = n.clamp(0.0, 1.0);
        let v = self.param.preview_plain(n);
        self.setter.set_parameter(self.param, v);
    }

    fn show(self, ui: &mut Ui) -> Response {
        let label_h = if self.label.is_some() { 14.0 } else { 0.0 };
        let value_h = 14.0;
        let total = vec2(self.diameter + 12.0, self.diameter + label_h + value_h + 6.0);
        let (rect, mut resp) = ui.allocate_exact_size(total, Sense::click_and_drag());

        // Layout sub-rectangles top to bottom: label, knob, value.
        let mut cursor_y = rect.min.y;
        let label_rect = Rect::from_min_size(
            Pos2::new(rect.min.x, cursor_y),
            vec2(rect.width(), label_h),
        );
        cursor_y += label_h + 2.0;
        let knob_rect = Rect::from_min_size(
            Pos2::new(rect.center().x - self.diameter * 0.5, cursor_y),
            vec2(self.diameter, self.diameter),
        );
        cursor_y += self.diameter + 2.0;
        let value_rect = Rect::from_min_size(
            Pos2::new(rect.min.x, cursor_y),
            vec2(rect.width(), value_h),
        );

        // Interaction
        if resp.drag_started() {
            self.setter.begin_set_parameter(self.param);
            // Stash starting normalized value for absolute-style dragging.
            let id = resp.id;
            ui.memory_mut(|m| {
                m.data
                    .insert_temp(id, self.param.modulated_normalized_value())
            });
        }
        if resp.dragged() {
            let id = resp.id;
            let start: f32 = ui
                .memory(|m| m.data.get_temp(id))
                .unwrap_or_else(|| self.param.modulated_normalized_value());
            let drag_total = -resp.drag_delta().y; // up = increase
            let pixels_for_full = if ui.input(|i| i.modifiers.shift) {
                DRAG_PIXELS_FOR_FULL_RANGE_FINE
            } else {
                DRAG_PIXELS_FOR_FULL_RANGE
            };
            // Accumulate total drag since drag start by re-reading drag_delta each frame.
            // (drag_delta is per-frame, so we keep a running offset in memory too.)
            let acc_id = id.with("acc");
            let acc: f32 = ui.memory(|m| m.data.get_temp(acc_id)).unwrap_or(0.0);
            let new_acc = acc + drag_total;
            ui.memory_mut(|m| m.data.insert_temp(acc_id, new_acc));
            let n = (start + new_acc / pixels_for_full).clamp(0.0, 1.0);
            self.set_normalized(n);
            resp.mark_changed();
        }
        if resp.drag_stopped() {
            let id = resp.id;
            ui.memory_mut(|m| {
                m.data.remove::<f32>(id);
                m.data.remove::<f32>(id.with("acc"));
            });
            self.setter.end_set_parameter(self.param);
        }
        if resp.double_clicked() || (resp.clicked() && ui.input(|i| i.modifiers.command)) {
            self.setter.begin_set_parameter(self.param);
            self.set_normalized(self.param.default_normalized_value());
            self.setter.end_set_parameter(self.param);
            resp.mark_changed();
        }
        // Scroll-wheel adjustment when hovered.
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                let step = if ui.input(|i| i.modifiers.shift) { 0.005 } else { 0.02 };
                let n = (self.param.modulated_normalized_value() + scroll.signum() * step)
                    .clamp(0.0, 1.0);
                self.setter.begin_set_parameter(self.param);
                self.set_normalized(n);
                self.setter.end_set_parameter(self.param);
                resp.mark_changed();
            }
        }

        // Drawing
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);

            if let Some(label) = self.label {
                painter.text(
                    label_rect.center(),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(11.0),
                    palette::TEXT_DIM,
                );
            }

            let center = knob_rect.center();
            let radius = self.diameter * 0.5;
            let active = self.param.modulated_normalized_value();

            // Track arc (background).
            paint_arc(
                &painter,
                center,
                radius - 4.0,
                ARC_START_DEG.to_radians(),
                ARC_END_DEG.to_radians(),
                3.0,
                palette::TRACK,
            );

            // Active arc.
            let active_end =
                (ARC_START_DEG + (ARC_END_DEG - ARC_START_DEG) * active).to_radians();
            paint_arc(
                &painter,
                center,
                radius - 4.0,
                ARC_START_DEG.to_radians(),
                active_end,
                3.0,
                if resp.dragged() { palette::ACCENT } else { palette::ACCENT_DIM },
            );

            // Knob body.
            painter.circle_filled(center, radius - 8.0, palette::BG_PANEL_HI);
            painter.circle_stroke(
                center,
                radius - 8.0,
                Stroke::new(1.0, palette::BORDER),
            );

            // Pointer line.
            let pointer_len = radius - 10.0;
            let dir = Vec2::angled(active_end - std::f32::consts::FRAC_PI_2);
            painter.line_segment(
                [center + dir * 4.0, center + dir * pointer_len],
                Stroke::new(2.0, palette::ACCENT),
            );

            // Value text.
            painter.text(
                value_rect.center(),
                Align2::CENTER_CENTER,
                self.param.to_string(),
                FontId::monospace(10.0),
                palette::TEXT,
            );
        }

        resp.on_hover_text(format!(
            "{}: {}\nDrag to adjust  •  Shift = fine  •  Double-click to reset",
            self.param.name(),
            self.param.to_string()
        ))
    }
}

impl<P: Param> egui::Widget for Knob<'_, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui)
    }
}

fn paint_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start: f32,
    end: f32,
    width: f32,
    color: Color32,
) {
    if (end - start).abs() < 1e-3 {
        return;
    }
    let segments = 48;
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = start + (end - start) * t - std::f32::consts::FRAC_PI_2;
        points.push(center + Vec2::angled(a) * radius);
    }
    painter.add(Shape::line(points, Stroke::new(width, color)));
}

/// An LED-style toggle bound to a `BoolParam`.
pub fn led_toggle(
    ui: &mut Ui,
    param: &nih_plug::params::BoolParam,
    setter: &ParamSetter,
    label: &str,
) -> Response {
    let total = vec2(64.0, 30.0);
    let (rect, mut resp) = ui.allocate_exact_size(total, Sense::click());
    let on = param.modulated_normalized_value() >= 0.5;

    if resp.clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, !on);
        setter.end_set_parameter(param);
        resp.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        // Plate
        painter.rect_filled(rect, 3.0, palette::BG_PANEL_HI);
        painter.rect_stroke(
            rect,
            3.0,
            Stroke::new(1.0, palette::BORDER),
            egui::StrokeKind::Inside,
        );
        // LED dot
        let dot_center = Pos2::new(rect.min.x + 10.0, rect.center().y);
        let (fill, ring) = if on {
            (palette::ACCENT, palette::ACCENT_DIM)
        } else {
            (palette::TRACK, palette::BORDER)
        };
        painter.circle_filled(dot_center, 5.0, fill);
        painter.circle_stroke(dot_center, 6.0, Stroke::new(1.0, ring));
        // Label
        painter.text(
            Pos2::new(rect.min.x + 22.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(11.0),
            palette::TEXT,
        );
    }

    resp
}

/// Draw a titled, bordered panel and run `add_contents` inside it.
pub fn panel<R>(ui: &mut Ui, title: &str, accent: Color32, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let frame = egui::Frame::group(ui.style())
        .fill(palette::BG_PANEL)
        .stroke(Stroke::new(1.0, palette::BORDER))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .corner_radius(4.0);
    frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (tag_rect, _) = ui.allocate_exact_size(vec2(6.0, 14.0), Sense::hover());
                ui.painter().rect_filled(tag_rect, 1.0, accent);
                ui.label(
                    egui::RichText::new(title)
                        .color(palette::TEXT)
                        .strong()
                        .size(12.0),
                );
            });
            ui.add_space(4.0);
            add_contents(ui)
        })
        .inner
}

/// Apply the retro chiptune visual style to the egui context. Call once per frame.
pub fn apply_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = palette::BG_DEEP;
    visuals.panel_fill = palette::BG_DEEP;
    visuals.extreme_bg_color = palette::BG_DEEP;
    visuals.faint_bg_color = palette::BG_PANEL;
    visuals.widgets.noninteractive.bg_fill = palette::BG_PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette::BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette::TEXT);
    visuals.widgets.inactive.bg_fill = palette::BG_PANEL_HI;
    visuals.widgets.inactive.weak_bg_fill = palette::BG_PANEL_HI;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette::TEXT);
    visuals.widgets.hovered.bg_fill = palette::BG_PANEL_HI;
    visuals.widgets.hovered.weak_bg_fill = palette::BG_PANEL_HI;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette::ACCENT_DIM);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette::ACCENT);
    visuals.widgets.active.bg_fill = palette::ACCENT_DIM;
    visuals.widgets.active.weak_bg_fill = palette::ACCENT_DIM;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette::ACCENT);
    visuals.selection.bg_fill = palette::ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, palette::ACCENT);
    ctx.set_visuals(visuals);

    let _ = TAU; // silence unused-import if compiler is grumpy
}
