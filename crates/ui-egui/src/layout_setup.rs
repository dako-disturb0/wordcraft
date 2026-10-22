//! The layout question: at the first start a window asks whether WordCraft should use the mobile
//! layout, the desktop layout, or pick one by itself from the screen (Automatic, highlighted). The
//! answer is the File › Options › General › Interface layout setting ([`crate::mobile`]); until it
//! is given WordCraft uses the desktop layout, as before. Hosts open it with
//! [`WordApp::ask_layout`] once the settings are restored; the mobile layout's menu (Choose
//! layout…) and `ui.layoutSetup` bring it back.

use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::WordApp;
use crate::mobile::LayoutMode;
use crate::theme::{Tokens, medium, regular, semibold};

/// The open question: the answer highlighted so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setup {
    pub choice: LayoutMode,
}

impl Setup {
    pub fn new(choice: LayoutMode) -> Self {
        Setup { choice }
    }
}

/// What each answer does (English, translated where shown).
pub fn description(mode: LayoutMode) -> &'static str {
    match mode {
        LayoutMode::Auto => "Switches between the mobile and desktop layouts by itself as the window changes size.",
        LayoutMode::Mobile => "Large touch buttons, a toolbar at the bottom and pages that fit the screen.",
        LayoutMode::Desktop => "The full ribbon, rulers and status bar, for a mouse and keyboard.",
    }
}

/// What Automatic comes to on this screen, for the lines under the answers.
#[derive(Clone, Copy)]
struct Here {
    /// The screen's size in points, once a frame has measured it.
    size: Option<egui::Vec2>,
    /// Automatic shows the mobile layout on it.
    mobile: bool,
}

/// Room around an answer's contents, the width of its sketch, and the radius of its radio mark.
const PAD: f32 = 12.0;
const SKETCH: f32 = 64.0;
const MARK: f32 = 9.0;

/// Draws the question while it is open. Continue (or Enter, or Escape) keeps the highlighted
/// answer; a tap beside the window doesn't count, it is too easily made by accident.
pub fn show(app: &mut WordApp, ctx: &egui::Context) {
    let Some(mut setup) = app.mobile.setup else { return };
    let screen = ctx.content_rect();
    let width = (screen.width() - 72.0).clamp(220.0, 460.0);
    let max_h = (screen.height() - 150.0).max(120.0);
    let m = &app.mobile;
    let here = Here { size: m.screen, mobile: LayoutMode::Auto.wants_mobile(m.screen, m.touch, m.active) };
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin::same(20));
    let resp = egui::Modal::new(egui::Id::new("wordcraft.layoutSetup")).frame(frame).show(ctx, |ui| body(ui, &mut setup, width, max_h, here));
    let answer = resp.inner.or_else(|| (resp.should_close() && !resp.backdrop_response.clicked()).then_some(setup.choice));
    match answer {
        Some(mode) => {
            app.mobile.setup = None;
            app.set_layout_mode(mode);
        }
        None => app.mobile.setup = Some(setup),
    }
}

/// The question; `Some(answer)` once Continue (or Enter) gives it.
fn body(ui: &mut Ui, setup: &mut Setup, width: f32, max_h: f32, here: Here) -> Option<LayoutMode> {
    let t = Tokens::get(ui.ctx());
    ui.set_width(width);
    egui::ScrollArea::vertical().max_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
        ui.label(egui::RichText::new(tl!("Choose a layout")).font(semibold(20.0)).color(t.text));
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(tl!("WordCraft can fit its layout to your screen. You can change it later in File › Options › General."))
                .font(regular(13.0))
                .color(t.text_dim),
        );
        ui.add_space(12.0);
        for mode in LayoutMode::ALL {
            if card(ui, mode, setup.choice == mode, width).clicked() {
                setup.choice = mode;
            }
            ui.add_space(8.0);
        }
        if let Some(size) = here.size {
            let (w, h) = (size.x.round().to_string(), size.y.round().to_string());
            let text = crate::i18n::fmt(tl!("Screen size: {width} × {height}"), &[("width", &w), ("height", &h)]);
            ui.label(egui::RichText::new(text).font(regular(12.0)).color(t.text_dim));
        }
        let auto = if here.mobile {
            tl!("Automatic uses the mobile layout on this screen.")
        } else {
            tl!("Automatic uses the desktop layout on this screen.")
        };
        ui.label(egui::RichText::new(auto).font(regular(12.0)).color(t.text_dim));
    });
    // The arrow keys move the highlight; Enter answers.
    let (up, down, enter) = ui.input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::Enter)));
    if up || down {
        let at = LayoutMode::ALL.iter().position(|m| *m == setup.choice).unwrap_or(0);
        let next = if down { at.saturating_add(1) } else { at.saturating_sub(1) };
        if let Some(m) = LayoutMode::ALL.get(next) {
            setup.choice = *m;
        }
    }
    ui.add_space(14.0);
    let mut answer = enter.then_some(setup.choice);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui.add(crate::widgets::primary_button(ui, tl!("Continue")).min_size(vec2(120.0, 40.0))).clicked() {
            answer = Some(setup.choice);
        }
    });
    answer
}

/// One answer: a sketch of the layout, its name and what it does, and a radio mark.
fn card(ui: &mut Ui, mode: LayoutMode, chosen: bool, width: f32) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let name = tl!(mode.label());
    let text_x = PAD + SKETCH + PAD;
    let wrap = (width - text_x - PAD - 2.0 * MARK - PAD).max(60.0);
    let about = ui.painter().layout(tl!(description(mode)).to_string(), regular(12.0), t.text_dim, wrap);
    let height = (PAD + 22.0 + about.size().y + PAD).max(PAD + 48.0 + PAD);
    let (r, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, chosen, name));
    let p = ui.painter();
    let fill = if chosen {
        t.checked
    } else if resp.hovered() || resp.is_pointer_button_down_on() {
        t.hover
    } else {
        t.input
    };
    let stroke = if chosen { Stroke::new(2.0, t.accent) } else { Stroke::new(1.0, t.border_strong) };
    p.rect(r, CornerRadius::same(10), fill, stroke, egui::StrokeKind::Inside);
    sketch(p, Rect::from_min_size(pos2(r.min.x + PAD, r.center().y - 24.0), vec2(SKETCH, 48.0)), mode, &t);
    let x = r.min.x + text_x;
    let title = p.text(pos2(x, r.min.y + PAD), Align2::LEFT_TOP, name, semibold(15.0), t.text);
    if mode == LayoutMode::Auto {
        let badge = p.layout_no_wrap(tl!("Recommended").to_string(), medium(11.0), t.on_accent);
        let b = Rect::from_min_size(pos2(title.max.x + 8.0, title.center().y - badge.size().y / 2.0 - 2.0), badge.size() + vec2(12.0, 4.0));
        // Only where it fits beside the name (long translations, narrow phones).
        if b.max.x < r.max.x - PAD - 2.0 * MARK - PAD {
            p.rect_filled(b, CornerRadius::same(8), t.accent);
            p.galley(b.min + vec2(6.0, 2.0), badge, t.on_accent);
        }
    }
    p.galley(pos2(x, r.min.y + PAD + 22.0), about, t.text_dim);
    let c = pos2(r.max.x - PAD - MARK, r.center().y);
    p.circle_stroke(c, MARK, Stroke::new(1.5, if chosen { t.accent } else { t.border_strong }));
    if chosen {
        p.circle_filled(c, MARK - 4.0, t.accent);
    }
    resp
}

/// A small drawing of the layout: a window with the ribbon over the page and a status bar under
/// it (Desktop), a phone with a bar at the top and a toolbar at the bottom (Mobile), or both
/// (Automatic).
fn sketch(p: &egui::Painter, r: Rect, mode: LayoutMode, t: &Tokens) {
    match mode {
        LayoutMode::Desktop => window_sketch(p, Rect::from_center_size(r.center(), vec2(r.width(), r.height() * 0.8)), t),
        LayoutMode::Mobile => phone_sketch(p, Rect::from_center_size(r.center(), vec2(r.height() * 0.55, r.height())), t),
        LayoutMode::Auto => {
            window_sketch(p, Rect::from_min_size(r.min + vec2(0.0, 4.0), vec2(r.width() * 0.74, r.height() * 0.66)), t);
            let phone = vec2(r.width() * 0.34, r.height() * 0.8);
            phone_sketch(p, Rect::from_min_size(pos2(r.max.x - phone.x, r.max.y - phone.y), phone), t);
        }
    }
}

/// A band across `r` from `top` to `bottom` (fractions of its height), `inset` points in from
/// its sides.
fn band(r: Rect, top: f32, bottom: f32, inset: f32) -> Rect {
    Rect::from_min_max(pos2(r.min.x + inset, r.min.y + r.height() * top), pos2(r.max.x - inset, r.min.y + r.height() * bottom))
}

/// The desktop layout: title bar, ribbon, a page, status bar.
fn window_sketch(p: &egui::Painter, r: Rect, t: &Tokens) {
    p.rect(r, CornerRadius::same(3), t.canvas, Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
    p.rect_filled(band(r, 0.0, 0.16, 1.0), CornerRadius::same(2), t.title_bar);
    let ribbon = band(r, 0.16, 0.34, 1.0);
    p.rect_filled(ribbon, 0.0, t.ribbon);
    for i in [0.0, 1.0, 2.0, 3.0] {
        let x = ribbon.min.x + 4.0 + i * (ribbon.width() - 8.0) / 4.0;
        p.rect_filled(Rect::from_min_size(pos2(x, ribbon.center().y - 1.5), vec2(4.0, 3.0)), 1.0, t.accent);
    }
    p.rect(band(r, 0.42, 0.86, r.width() * 0.28), 0.0, Color32::WHITE, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    p.rect_filled(band(r, 0.9, 1.0, 1.0), CornerRadius::same(2), t.status_bar);
}

/// The mobile layout: a phone with the top bar, a page, and the toolbar.
fn phone_sketch(p: &egui::Painter, r: Rect, t: &Tokens) {
    p.rect(r, CornerRadius::same(5), t.canvas, Stroke::new(1.5, t.icon), egui::StrokeKind::Inside);
    p.rect_filled(band(r, 0.07, 0.17, 2.5), CornerRadius::same(1), t.title_bar);
    p.rect(band(r, 0.24, 0.7, r.width() * 0.16), 0.0, Color32::WHITE, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    let tools = band(r, 0.76, 0.93, 2.5);
    p.rect_filled(tools, CornerRadius::same(1), t.ribbon);
    for f in [0.25, 0.5, 0.75] {
        p.circle_filled(pos2(tools.min.x + tools.width() * f, tools.center().y), 1.5, t.accent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Services;
    use egui_kittest::kittest::Queryable;
    use wordcraft_engine::Session;

    fn asked() -> egui_kittest::Harness<'static, WordApp> {
        let mut a = WordApp::new(Session::new(wordcraft_doc::Document::new()), Services::default());
        assert!(a.mobile.setup.is_none(), "`new` never asks");
        assert!(a.ask_layout());
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            a,
        );
        h.run_steps(6);
        h
    }

    /// The question shows what Automatic comes to here; picking Mobile and Continue sets the
    /// layout, and the question is answered for good.
    #[test]
    fn picking_an_answer_sets_the_layout() {
        let mut h = asked();
        assert!(h.query_by_label("Choose a layout").is_some());
        assert!(h.query_by_label("Automatic uses the desktop layout on this screen.").is_some(), "a wide window");
        h.get_by_label("Mobile").click_accesskit();
        h.run_steps(3);
        assert_eq!(h.state().mobile.setup.map(|s| s.choice), Some(LayoutMode::Mobile));
        h.get_by_label("Continue").click_accesskit();
        h.run_steps(4);
        let a = h.state_mut();
        assert!(a.mobile.setup.is_none() && a.ui.layout_chosen);
        assert_eq!(a.ui.layout, LayoutMode::Mobile);
        assert!(a.mobile_layout());
        assert!(!a.ask_layout(), "answered");
    }

    /// Escape keeps the highlighted answer: Automatic at the first start, which in a wide window
    /// is the desktop layout.
    #[test]
    fn escape_keeps_automatic() {
        let mut h = asked();
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        let a = h.state();
        assert!(a.mobile.setup.is_none());
        assert_eq!((a.ui.layout, a.ui.layout_chosen), (LayoutMode::Auto, true));
        assert!(!a.mobile_layout());
    }

    /// The arrow keys move the highlight and Enter answers.
    #[test]
    fn arrow_keys_and_enter_answer() {
        let mut h = asked();
        h.key_press(egui::Key::ArrowDown);
        h.run_steps(3);
        assert_eq!(h.state().mobile.setup.map(|s| s.choice), Some(LayoutMode::Mobile));
        h.key_press(egui::Key::Enter);
        h.run_steps(3);
        assert!(h.state().mobile.setup.is_none());
        assert_eq!(h.state().ui.layout, LayoutMode::Mobile);
    }

    /// Every answer has a name and says what it does.
    #[test]
    fn every_answer_is_described() {
        for mode in LayoutMode::ALL {
            assert!(!mode.label().is_empty() && description(mode).ends_with('.'));
        }
    }
}
