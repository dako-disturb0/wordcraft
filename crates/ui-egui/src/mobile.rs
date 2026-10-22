//! The mobile layout and the interface layout setting.
//!
//! The setting ([`LayoutMode`]: File › Options › General › Interface layout, `ui.layout`) is
//! Desktop, Mobile or Automatic. Automatic shows the mobile layout in a narrow window and on a
//! phone held either way, and follows the window as it is resized or turned ([`auto_mobile`]). The
//! web app asks at the first visit which one to use ([`crate::layout_setup`]); until it has been
//! answered WordCraft looks as it always did.
//!
//! The mobile layout keeps the page and moves the commands within a thumb's reach: a bar at the
//! top with File (the Backstage), the document's name, Undo, Redo and a menu (Save, Search
//! Commands, the layout, Options), and a toolbar at the bottom with the Home, Insert, Layout,
//! Review and View commands as large buttons in a row that scrolls sideways. The ribbon, the
//! rulers and the status bar make way for the page, which fits the screen's width; status messages
//! show above the toolbar. On a touch screen a finger drag scrolls the page, a tap places the
//! caret, a double tap selects a word and a long press opens the context menu (`canvas.rs`).

use egui::{CornerRadius, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::theme::{Tokens, medium, regular, semibold};
use crate::{WordApp, icons, widgets};

/// The interface layout setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "String", into = "String")]
pub enum LayoutMode {
    /// The mobile layout on phones and in narrow windows, the desktop layout elsewhere
    /// ([`auto_mobile`]).
    Auto,
    /// The ribbon, the rulers and the status bar, whatever the screen. The default until the
    /// layout question has been answered, so nothing changes for anyone who hasn't been asked.
    #[default]
    Desktop,
    /// The mobile layout, whatever the screen.
    Mobile,
}

impl LayoutMode {
    /// In the order the layout question and File › Options offer them.
    pub const ALL: [LayoutMode; 3] = [LayoutMode::Auto, LayoutMode::Mobile, LayoutMode::Desktop];

    /// The value used in `ui.json` and by the `ui.layout` command.
    pub fn code(self) -> &'static str {
        match self {
            LayoutMode::Auto => "auto",
            LayoutMode::Desktop => "desktop",
            LayoutMode::Mobile => "mobile",
        }
    }

    pub fn from_code(code: &str) -> Option<LayoutMode> {
        LayoutMode::ALL.into_iter().find(|m| m.code().eq_ignore_ascii_case(code.trim()))
    }

    /// The English name (translated with `tl!` where shown).
    pub fn label(self) -> &'static str {
        match self {
            LayoutMode::Auto => "Automatic",
            LayoutMode::Desktop => "Desktop",
            LayoutMode::Mobile => "Mobile",
        }
    }

    /// Whether the mobile layout shows on a screen `size` points large (`None` before the first
    /// frame), `touch` whether it is a touch screen and `was_mobile` whether it shows now.
    pub fn wants_mobile(self, size: Option<egui::Vec2>, touch: bool, was_mobile: bool) -> bool {
        match self {
            LayoutMode::Desktop => false,
            LayoutMode::Mobile => true,
            LayoutMode::Auto => size.map_or(was_mobile, |s| auto_mobile(s, touch, was_mobile)),
        }
    }
}

/// An unknown saved value reads as the default rather than discarding the whole `ui.json`.
impl From<String> for LayoutMode {
    fn from(s: String) -> Self {
        LayoutMode::from_code(&s).unwrap_or_default()
    }
}

impl From<LayoutMode> for String {
    fn from(m: LayoutMode) -> Self {
        m.code().to_string()
    }
}

/// Automatic shows the mobile layout in a window narrower than this, in points…
pub const NARROW: f32 = 600.0;
/// …and keeps it until the window is at least this wide, so a window dragged across the line
/// doesn't flip from one layout to the other and back.
pub const WIDE: f32 = 680.0;
/// A touch screen less wide than `PHONE_LONG` and less tall than `PHONE_SHORT` is a phone held
/// sideways. A tablet's on-screen keyboard can leave less height than that, but not less width.
pub const PHONE_LONG: f32 = 960.0;
pub const PHONE_SHORT: f32 = 500.0;

/// Automatic's choice for a screen `size` points large: the mobile layout for a narrow window,
/// and for a touch screen held like a phone either way. `was_mobile` gives the way back to the
/// desktop layout some slack ([`WIDE`]). A size that makes no sense keeps the layout showing.
pub fn auto_mobile(size: egui::Vec2, touch: bool, was_mobile: bool) -> bool {
    if !size.x.is_finite() || !size.y.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
        return was_mobile;
    }
    let line = if was_mobile { WIDE } else { NARROW };
    size.x < line || (touch && size.x < PHONE_LONG && size.y < PHONE_SHORT)
}

/// The tabs of the mobile layout's toolbar: the ribbon tabs a phone needs most.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Home,
    Insert,
    Layout,
    Review,
    View,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Home, Tab::Insert, Tab::Layout, Tab::Review, Tab::View];

    /// The ribbon tab's English name (translated where shown).
    pub fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Insert => "Insert",
            Tab::Layout => "Layout",
            Tab::Review => "Review",
            Tab::View => "View",
        }
    }
}

/// What the mobile layout keeps while WordCraft runs (none of it is saved).
#[derive(Default)]
pub struct MobileState {
    /// The mobile layout shows.
    pub active: bool,
    /// The screen's size in points and whether it is a touch screen, as of the last frame.
    pub screen: Option<egui::Vec2>,
    pub touch: bool,
    /// The toolbar's tab.
    pub tab: Tab,
    /// The zoom (`view.zoom`'s value) the page had before the mobile layout fitted it to the
    /// screen's width; it comes back with the desktop layout unless the user zoomed since.
    zoom_before: Option<Value>,
    /// The document whose page the mobile layout fitted last.
    fitted: Option<u64>,
    /// The layout question ([`crate::layout_setup`]) while it is open.
    pub setup: Option<crate::layout_setup::Setup>,
}

impl MobileState {
    /// Whether a finger drag on the page scrolls it rather than selecting text, and a tap places
    /// the caret: the mobile layout on a touch screen.
    pub fn touch_scrolls(&self) -> bool {
        self.active && self.touch
    }
}

impl WordApp {
    /// Whether the mobile layout shows.
    pub fn mobile_layout(&self) -> bool {
        self.mobile.active
    }

    /// Use the interface layout `mode` from now on (File › Options, the layout question,
    /// `ui.layout`). It counts as the user's answer, so the question isn't asked again.
    pub fn set_layout_mode(&mut self, mode: LayoutMode) {
        self.ui.layout = mode;
        self.ui.layout_chosen = true;
        self.sync_layout();
    }

    /// Ask the layout question again (the mobile menu's Choose layout…, `ui.layoutSetup`), with
    /// the current setting highlighted.
    pub fn open_layout_setup(&mut self) {
        self.mobile.setup = Some(crate::layout_setup::Setup::new(self.ui.layout));
    }

    /// Note the screen's size and whether it is a touch screen, then show the layout the setting
    /// asks for there. Every frame, before anything is laid out.
    pub(crate) fn follow_screen(&mut self, ctx: &egui::Context) {
        let size = ctx.content_rect().size();
        self.mobile.screen = (size.x > 0.0 && size.y > 0.0).then_some(size);
        let host = self.services.touch_screen.as_ref().and_then(|ask| ask());
        self.mobile.touch = host.unwrap_or(false) || ctx.input(|i| i.has_touch_screen());
        self.sync_layout();
    }

    /// Show the layout the setting asks for on the screen last seen: in the mobile layout the page
    /// fits the screen's width, back in the desktop one it gets its zoom back.
    fn sync_layout(&mut self) {
        let m = &self.mobile;
        let want = self.ui.layout.wants_mobile(m.screen, m.touch, m.active);
        if want != m.active {
            self.mobile.active = want;
            if want {
                self.mobile.fitted = None;
            } else {
                self.restore_zoom();
            }
        }
        if self.mobile.active {
            self.fit_page_width();
        }
    }

    /// The mobile layout fits the page to the screen's width when it comes up, and each document
    /// opened or created while it shows, once it is in Print Layout (the other views wrap to the
    /// window already). A zoom the user picks afterwards stays.
    fn fit_page_width(&mut self) {
        let doc = self.session.document_id();
        let v = &self.session.view;
        let print = !v.read_mode && v.mode == wordcraft_layout::ViewMode::Print;
        if self.mobile.fitted == Some(doc) || !print || !self.has_document() {
            return;
        }
        self.mobile.fitted = Some(doc);
        self.mobile.zoom_before = None;
        if v.fit == "pageWidth" {
            return;
        }
        let before = zoom_value(&v.fit, v.zoom);
        if self.session.run("view.pageWidth", &json!({})).is_ok() {
            self.mobile.zoom_before = Some(before);
        }
    }

    /// Back in the desktop layout the page gets the zoom it had before the mobile layout fitted
    /// it, unless the user zoomed since.
    fn restore_zoom(&mut self) {
        let doc = self.session.document_id();
        if let Some(before) = self.mobile.zoom_before.take()
            && self.mobile.fitted == Some(doc)
            && self.session.view.fit == "pageWidth"
            && !self.session.view.read_mode
        {
            let _ = self.session.run("view.zoom", &json!({"value": before}));
        }
        self.mobile.fitted = None;
    }
}

/// `view.zoom`'s value for a page shown in the fit mode `fit`, else at `zoom` (1 = 100 %).
fn zoom_value(fit: &str, zoom: f32) -> Value {
    if !fit.is_empty() {
        return json!(fit);
    }
    let percent = if zoom.is_finite() { (zoom * 100.0).round().clamp(10.0, 500.0) } else { 100.0 };
    json!(percent)
}

/// Height of the top bar, of a toolbar tab and of a touch target (the platforms' guidelines ask
/// for 44–48 points).
const BAR_H: f32 = 48.0;
const TAB_H: f32 = 34.0;
const TOUCH: f32 = 44.0;
/// The toolbar: the tabs over a row of buttons, with a little air around the buttons.
const TOOLBAR_H: f32 = TAB_H + TOUCH + 8.0;
/// How long a status message shows, in milliseconds (as long as in the status bar).
const TOAST_MS: f64 = 6000.0;

/// The top bar: File (the Backstage), the document's name, Undo, Redo and the menu.
pub fn top_bar(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    // macOS: the bar is the window's title bar, with room for the traffic lights.
    let mac = app.integrated_titlebar;
    let left = if mac { app.mac_titlebar.content_left.clamp(0.0, 127.0) as i8 } else { 4 };
    let height = if mac { app.mac_titlebar.height.max(BAR_H) } else { BAR_H };
    egui::Panel::top("mobile_top_bar")
        .exact_size(height)
        .frame(egui::Frame::NONE.fill(t.title_bar).inner_margin(egui::Margin { left, right: 4, top: 0, bottom: 0 }))
        .show(ui, |ui| {
            let drag = ui.interact(ui.max_rect(), ui.id().with("mobile_title_drag"), Sense::click_and_drag());
            if mac && drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                if touch_button(ui, "menu", tl!("File"), true, false).clicked() {
                    let _ = app.run("ui.backstage", json!({"value": true}));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    let more = touch_button(ui, "more", tl!("More options"), true, false);
                    more_menu(app, &more);
                    tool(ui, app, "redo", "Redo", "edit.redo", json!({}), false);
                    tool(ui, app, "undo", "Undo", "edit.undo", json!({}), false);
                    let rest = ui.available_rect_before_wrap();
                    paint_title(app, ui, rest, &t);
                });
            });
        });
}

/// The document's name, cut short with … when it doesn't fit, with a dot after it while there are
/// unsaved changes.
fn paint_title(app: &WordApp, ui: &Ui, rect: Rect, t: &Tokens) {
    if !app.has_document() || rect.width() < 24.0 {
        return;
    }
    let name = if app.session.dirty { format!("{} •", app.title_stem()) } else { app.title_stem() };
    let mut job = egui::text::LayoutJob::single_section(name, egui::text::TextFormat::simple(semibold(15.0), t.text));
    job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - 8.0);
    let galley = ui.painter().layout_job(job);
    let at = pos2(rect.min.x + 4.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(rect).galley(at, galley, t.text);
}

/// The top bar's menu: Save, Search Commands, the layout, and Options.
fn more_menu(app: &mut WordApp, button: &egui::Response) {
    egui::Popup::menu(button).align(egui::RectAlign::BOTTOM_END).show(|ui| {
        let save = widgets::enabled(app, "file.save");
        if menu_item(ui, tl!("Save"), save) {
            let _ = app.run("file.save", json!({}));
        }
        if menu_item(ui, tl!("Search Commands"), app.has_document()) {
            let _ = app.run("ui.dialog", json!({"name": "commands"}));
        }
        ui.separator();
        if menu_item(ui, tl!("Use desktop layout"), true) {
            app.set_layout_mode(LayoutMode::Desktop);
        }
        if menu_item(ui, tl!("Choose layout…"), true) {
            app.open_layout_setup();
        }
        if menu_item(ui, tl!("Options"), true) {
            let _ = app.run("ui.backstage", json!({"value": true, "page": "options"}));
        }
    });
}

/// A menu entry tall enough for a finger; true when it was picked, and the menu closes.
fn menu_item(ui: &mut Ui, text: &str, on: bool) -> bool {
    let picked = ui.add_enabled(on, egui::Button::new(text).min_size(vec2(220.0, 36.0))).clicked();
    if picked {
        ui.close();
    }
    picked
}

/// The toolbar at the bottom: the tabs, then the chosen tab's commands as large buttons in a row
/// that scrolls sideways.
pub fn bottom_bar(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("mobile_toolbar").exact_size(TOOLBAR_H).frame(egui::Frame::NONE.fill(t.ribbon)).show(ui, |ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        strip("mobile_tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                ui.add_space(6.0);
                for tab in Tab::ALL {
                    if tab_chip(ui, tl!(tab.label()), app.mobile.tab == tab).clicked() {
                        app.mobile.tab = tab;
                    }
                }
            });
        });
        ui.add_space(4.0);
        // Each tab keeps where its row was scrolled to.
        let tab = app.mobile.tab;
        strip(("mobile_tools", tab.label())).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                ui.add_space(4.0);
                match tab {
                    Tab::Home => home_tools(app, ui),
                    Tab::Insert => insert_tools(app, ui),
                    Tab::Layout => layout_tools(app, ui),
                    Tab::Review => review_tools(app, ui),
                    Tab::View => view_tools(app, ui),
                }
                ui.add_space(4.0);
            });
        });
    });
}

/// A row that scrolls sideways, without a scroll bar: a finger swipes it.
fn strip(id: impl egui::AsIdSalt) -> egui::ScrollArea {
    egui::ScrollArea::horizontal().id_salt(id).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
}

/// Home: character and paragraph formatting, then the clipboard.
fn home_tools(app: &mut WordApp, ui: &mut Ui) {
    let st = app.session.run("format.state", &json!({})).unwrap_or_default();
    let flag = |k: &str| st.get(k).and_then(Value::as_bool).unwrap_or(false);
    tool(ui, app, "bold", "Bold", "format.bold", json!({}), flag("bold"));
    tool(ui, app, "italic", "Italic", "format.italic", json!({}), flag("italic"));
    tool(ui, app, "underline", "Underline", "format.underline", json!({}), flag("underline"));
    tool(ui, app, "strike", "Strikethrough", "format.strikethrough", json!({}), flag("strike"));
    let highlight = app.canvas.last_highlight.clone();
    tool(ui, app, "highlight", "Text Highlight Color", "format.highlight", json!({"color": highlight}), false);
    let color = app.canvas.last_font_color.clone();
    tool(ui, app, "fontcolor", "Font Color", "format.color", json!({"color": color}), false);
    tool(ui, app, "grow", "Increase Font Size", "format.growFont", json!({}), false);
    tool(ui, app, "shrink", "Decrease Font Size", "format.shrinkFont", json!({}), false);
    tool(ui, app, "clear", "Clear All Formatting", "format.clear", json!({}), false);
    divider(ui);
    // The buttons show the alignment as seen on the page, as the ribbon's do.
    let para = app.session.doc.para_at(&app.session.sel.focus).map(|p| app.session.doc.styles.resolve_para(&p.props));
    let align = para.as_ref().map(|r| r.align.visual(r.bidi)).unwrap_or_default();
    let lists = wordcraft_engine::cmd::para::list_buttons(&app.session.doc, para.as_ref().and_then(|r| r.numbering));
    tool(ui, app, "bullets", "Bullets", "para.bullets", json!({}), lists.bullets);
    tool(ui, app, "numbering", "Numbering", "para.numbering", json!({}), lists.numbering);
    tool(ui, app, "outdent", "Decrease Indent", "para.outdent", json!({}), false);
    tool(ui, app, "indent", "Increase Indent", "para.indent", json!({}), false);
    {
        use wordcraft_doc::Align as A;
        tool(ui, app, "alignLeft", "Align Left", "para.alignLeft", json!({}), align == A::Left);
        tool(ui, app, "alignCenter", "Center", "para.alignCenter", json!({}), align == A::Center);
        tool(ui, app, "alignRight", "Align Right", "para.alignRight", json!({}), align == A::Right);
        tool(ui, app, "justify", "Justify", "para.justify", json!({}), align == A::Justify);
    }
    divider(ui);
    tool(ui, app, "cut", "Cut", "edit.cut", json!({}), false);
    tool(ui, app, "copy", "Copy", "edit.copy", json!({}), false);
    tool(ui, app, "paste", "Paste", "edit.paste", json!({}), false);
    dialog_tool(ui, app, "find", "Find", "find");
}

/// Insert: pictures, tables, links and comments, then breaks, boxes, symbols and the header and
/// footer.
fn insert_tools(app: &mut WordApp, ui: &mut Ui) {
    tool(ui, app, "picture", "Pictures", "insert.picture", json!({}), false);
    dialog_tool(ui, app, "table", "Insert Table…", "insertTable");
    dialog_tool(ui, app, "link", "Link", "link");
    tool(ui, app, "newComment", "New Comment", "review.newComment", json!({}), false);
    divider(ui);
    tool(ui, app, "pageBreak", "Page Break", "insert.pageBreak", json!({}), false);
    tool(ui, app, "textBox", "Text Box", "insert.textBox", json!({}), false);
    tool(ui, app, "symbol", "Symbol", "insert.symbol", json!({}), false);
    tool(ui, app, "dateTime", "Date & Time", "insert.dateTime", json!({}), false);
    divider(ui);
    tool(ui, app, "header", "Edit Header", "insert.editHeader", json!({}), false);
    tool(ui, app, "footer", "Edit Footer", "insert.editFooter", json!({}), false);
}

/// Layout: Page Setup, the orientation, the paragraph's spacing and indents, and page breaks.
fn layout_tools(app: &mut WordApp, ui: &mut Ui) {
    dialog_tool(ui, app, "margins", "Page Setup", "pageSetup");
    let landscape = wordcraft_engine::cmd::page::sect(&app.session).landscape;
    let turned = if landscape { "portrait" } else { "landscape" };
    tool(ui, app, "orientation", "Orientation", "layout.orientation", json!({"value": turned}), landscape);
    dialog_tool(ui, app, "paraSpacing", "Paragraph", "paragraph");
    tool(ui, app, "pageBreak", "Page Break", "insert.pageBreak", json!({}), false);
}

/// Review: proofing, comments and tracked changes.
fn review_tools(app: &mut WordApp, ui: &mut Ui) {
    tool(ui, app, "spelling", "Spelling & Grammar", "review.spelling", json!({}), false);
    tool(ui, app, "wordCount", "Word Count", "review.wordCount", json!({}), false);
    divider(ui);
    tool(ui, app, "newComment", "New Comment", "review.newComment", json!({}), false);
    tool(ui, app, "prevComment", "Previous Comment", "review.previousComment", json!({}), false);
    tool(ui, app, "nextComment", "Next Comment", "review.nextComment", json!({}), false);
    let shown = app.session.view.comments_pane;
    tool(ui, app, "showComments", "Comments Pane", "view.commentsPane", json!({"value": !shown}), shown);
    divider(ui);
    let tracking = app.session.doc.settings.track_changes;
    tool(ui, app, "trackChanges", "Track Changes", "review.trackChanges", json!({}), tracking);
    tool(ui, app, "accept", "Accept", "review.accept", json!({}), false);
    tool(ui, app, "reject", "Reject", "review.reject", json!({}), false);
}

/// View: the views, the zoom, and what shows on and beside the page.
fn view_tools(app: &mut WordApp, ui: &mut Ui) {
    let v = &app.session.view;
    let print = v.mode == wordcraft_layout::ViewMode::Print && !v.read_mode;
    let web = v.mode == wordcraft_layout::ViewMode::Web && !v.read_mode;
    let (read, fitted, nav, marks, dark) = (v.read_mode, v.fit == "pageWidth", v.nav_pane, v.marks, v.dark_mode);
    tool(ui, app, "printLayout", "Print Layout", "view.printLayout", json!({}), print);
    tool(ui, app, "webLayout", "Web Layout", "view.webLayout", json!({}), web);
    tool(ui, app, "readMode", "Read Mode", "view.readMode", json!({}), read);
    divider(ui);
    tool(ui, app, "pageWidth", "Page Width", "view.pageWidth", json!({}), fitted);
    tool(ui, app, "zoomIn", "Zoom In", "view.zoomIn", json!({}), false);
    tool(ui, app, "zoomOut", "Zoom Out", "view.zoomOut", json!({}), false);
    divider(ui);
    tool(ui, app, "navPane", "Navigation Pane", "view.navigationPane", json!({"value": !nav}), nav);
    tool(ui, app, "pilcrow", "Show/Hide ¶", "view.marks", json!({}), marks);
    tool(ui, app, "darkMode", "Switch Modes", "view.darkMode", json!({}), dark);
    tool(ui, app, "focus", "Focus", "view.focus", json!({}), false);
}

/// A 44 pt square button with a drawn icon. `label` is its accessible name and tooltip, `on`
/// whether it can be used now, and `checked` draws it pressed (a toggle that is on).
fn touch_button(ui: &mut Ui, icon: &str, label: &str, on: bool, checked: bool) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(TOUCH, TOUCH), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, on, label));
    let fill = if checked {
        t.checked
    } else if on {
        widgets::drawn_state(ui, &resp, false).fill
    } else {
        egui::Color32::TRANSPARENT
    };
    let face = r.shrink(2.0);
    ui.painter().rect_filled(face, CornerRadius::same(8), fill);
    if checked {
        ui.painter().rect_stroke(face, CornerRadius::same(8), Stroke::new(1.0, t.accent.linear_multiply(0.5)), egui::StrokeKind::Inside);
    }
    let (c, a) = if on { (t.icon, t.accent) } else { (t.text_disabled, t.text_disabled) };
    icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(22.0, 22.0)), icon, c, a);
    resp.on_hover_text(label)
}

/// A toolbar button that runs `id`, greyed out while it can't run. `label` is the English name
/// the ribbon gives it, translated here.
fn tool(ui: &mut Ui, app: &mut WordApp, icon: &str, label: &str, id: &str, params: Value, checked: bool) {
    let on = widgets::enabled(app, id);
    if touch_button(ui, icon, tl!(label), on, checked).clicked() && on {
        widgets::run_from_ribbon(app, id, params);
    }
}

/// A toolbar button that opens the dialog `name` (`ui.dialog`).
fn dialog_tool(ui: &mut Ui, app: &mut WordApp, icon: &str, label: &str, name: &str) {
    let on = app.has_document();
    if touch_button(ui, icon, tl!(label), on, false).clicked() && on {
        let _ = app.run("ui.dialog", json!({"name": name}));
    }
}

/// A thin line between groups of buttons.
fn divider(ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let (r, _) = ui.allocate_exact_size(vec2(9.0, TOUCH), Sense::hover());
    ui.painter().vline(r.center().x, r.shrink2(vec2(0.0, 10.0)).y_range(), Stroke::new(1.0, t.border));
}

/// One of the toolbar's tabs: its name, underlined while it is the one showing.
fn tab_chip(ui: &mut Ui, label: &str, active: bool) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let font = if active { semibold(14.0) } else { medium(14.0) };
    let color = if active { t.accent_text } else { t.text_dim };
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, color);
    let (r, resp) = ui.allocate_exact_size(vec2(galley.size().x + 24.0, TAB_H), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, label));
    ui.painter().rect_filled(r.shrink2(vec2(2.0, 4.0)), CornerRadius::same(6), widgets::drawn_state(ui, &resp, false).fill);
    ui.painter().galley(r.center() - galley.size() / 2.0, galley, color);
    if active {
        let line = Rect::from_min_max(pos2(r.min.x + 10.0, r.max.y - 3.0), pos2(r.max.x - 10.0, r.max.y - 1.0));
        ui.painter().rect_filled(line, CornerRadius::same(1), t.accent);
    }
    resp
}

/// Status messages, which the desktop layout shows in its status bar: above the toolbar (or at the
/// bottom of the window without it) for six seconds.
pub fn toast(app: &WordApp, ctx: &egui::Context, toolbar: bool) {
    let Some((msg, at)) = &app.status_msg else { return };
    let age = crate::now_ms() - at;
    if !(0.0..TOAST_MS).contains(&age) {
        return;
    }
    ctx.request_repaint_after(std::time::Duration::from_millis((TOAST_MS - age) as u64 + 1));
    let t = Tokens::get(ctx);
    let screen = ctx.content_rect();
    let below = if toolbar { TOOLBAR_H } else { 0.0 };
    let bottom = screen.max.y - below - 16.0;
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("mobile_toast")));
    let wrap = (screen.width() - 64.0).clamp(80.0, 480.0);
    let galley = painter.layout(msg.clone(), regular(13.0), t.ribbon, wrap);
    let size = galley.size() + vec2(28.0, 16.0);
    let rect = Rect::from_min_size(pos2(screen.center().x - size.x / 2.0, bottom - size.y), size);
    painter.rect_filled(rect, CornerRadius::same(12), t.text.gamma_multiply(0.92));
    painter.galley(rect.min + vec2(14.0, 8.0), galley, t.ribbon);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Services, UiState};
    use egui_kittest::kittest::Queryable;
    use wordcraft_engine::Session;

    fn app() -> WordApp {
        WordApp::new(Session::new(wordcraft_doc::Document::new()), Services::default())
    }

    fn harness(app: WordApp, w: f32, h: f32) -> egui_kittest::Harness<'static, WordApp> {
        egui_kittest::Harness::builder().with_size(egui::vec2(w, h)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        )
    }

    /// Automatic: the mobile layout in a narrow window or on a phone held either way, the desktop
    /// layout elsewhere, with some slack before it switches back.
    #[test]
    fn automatic_picks_the_layout_from_the_screen() {
        let size = egui::vec2;
        assert!(auto_mobile(size(390.0, 844.0), true, false), "a phone held upright");
        assert!(auto_mobile(size(844.0, 390.0), true, false), "and sideways");
        assert!(!auto_mobile(size(844.0, 390.0), false, false), "a short window on a computer is no phone");
        assert!(auto_mobile(size(500.0, 900.0), false, false), "a narrow window");
        assert!(!auto_mobile(size(1440.0, 900.0), false, false));
        assert!(!auto_mobile(size(1024.0, 400.0), true, false), "a tablet held sideways with its keyboard up");
        assert!(!auto_mobile(size(768.0, 1024.0), true, false), "a tablet held upright");
        // Between NARROW and WIDE the layout showing stays.
        assert!(!auto_mobile(size(640.0, 900.0), false, false));
        assert!(auto_mobile(size(640.0, 900.0), false, true));
        assert!(!auto_mobile(size(700.0, 900.0), false, true));
        for nonsense in [size(f32::NAN, 800.0), size(0.0, 0.0), size(800.0, f32::INFINITY)] {
            assert!(auto_mobile(nonsense, false, true) && !auto_mobile(nonsense, false, false), "{nonsense:?} keeps the layout");
        }
        assert!(LayoutMode::Mobile.wants_mobile(None, false, false));
        assert!(!LayoutMode::Desktop.wants_mobile(Some(size(390.0, 844.0)), true, true));
        assert!(!LayoutMode::Auto.wants_mobile(None, false, false), "no screen measured yet: as it was");
    }

    /// The setting is `auto`, `mobile` or `desktop` in `ui.json` and for `ui.layout`; anything else
    /// reads as Desktop without losing the other settings.
    #[test]
    fn the_setting_reads_leniently() {
        assert_eq!(LayoutMode::from_code(" Mobile "), Some(LayoutMode::Mobile));
        assert_eq!(LayoutMode::from_code("tablet"), None);
        assert_eq!(serde_json::to_string(&LayoutMode::Auto).unwrap(), "\"auto\"");
        assert_eq!(serde_json::from_str::<LayoutMode>("\"desktop\"").unwrap(), LayoutMode::Desktop);
        assert_eq!(serde_json::from_str::<LayoutMode>("\"tablet\"").unwrap(), LayoutMode::Desktop);
        let ui = UiState::from_json(br#"{"layout": "mobile", "layoutChosen": true}"#).unwrap();
        assert_eq!((ui.layout, ui.layout_chosen), (LayoutMode::Mobile, true));
        let ui = UiState::from_json(br#"{"layout": 7, "startScreen": false}"#).unwrap();
        assert_eq!(ui.layout, LayoutMode::Desktop);
        assert!(!ui.start_screen, "the other settings stay");
        let old = UiState::from_json(br#"{"tab": "Insert"}"#).unwrap();
        assert_eq!((old.layout, old.layout_chosen), (LayoutMode::Desktop, false), "settings from before the layouts");
    }

    /// The zoom the page gets back with the desktop layout: its fit mode, else its percentage.
    #[test]
    fn zoom_values_for_view_zoom() {
        assert_eq!(zoom_value("", 1.0), json!(100.0));
        assert_eq!(zoom_value("onePage", 0.4), json!("onePage"));
        assert_eq!(zoom_value("", f32::NAN), json!(100.0));
        assert_eq!(zoom_value("", 9.0), json!(500.0));
    }

    /// `ui.layout` reads and sets the layout and counts as the user's choice; it is kept between
    /// runs, and a chosen layout isn't asked about again.
    #[test]
    fn ui_layout_sets_the_layout_and_survives_a_restart() {
        let mut a = app();
        assert_eq!(a.run("ui.layout", json!({})).unwrap(), json!({"layout": "desktop", "chosen": false, "mobile": false}));
        assert_eq!(a.run("ui.layout", json!({"value": "mobile"})).unwrap(), json!({"layout": "mobile", "chosen": true, "mobile": true}));
        assert!(a.mobile_layout());
        for junk in [json!({"value": "tablet"}), json!({"value": 1}), json!({"value": null}), json!({"value": ["auto"]})] {
            assert!(a.run("ui.layout", junk).is_err());
        }
        assert_eq!(a.ui.layout, LayoutMode::Mobile, "a rejected value keeps the setting");
        let saved = serde_json::to_string(&a.prefs()).unwrap();
        assert!(saved.contains(r#""layout":"mobile""#) && saved.contains(r#""layoutChosen":true"#), "{saved}");
        let mut b = app();
        b.apply_prefs(serde_json::from_str(&saved).unwrap());
        assert_eq!(b.ui.layout, LayoutMode::Mobile);
        assert!(!b.ask_layout(), "answered: not asked again");
        let mut c = app();
        assert!(c.ask_layout(), "a first start asks");
        assert_eq!(c.mobile.setup.map(|s| s.choice), Some(LayoutMode::Auto), "with Automatic highlighted");
        assert!(crate::works_without_document("ui.layout") && crate::works_without_document("ui.layoutSetup"));
    }

    /// In a phone-sized window Automatic shows the mobile layout: the toolbar instead of the
    /// ribbon and the page fitted to the width. Its tabs switch the buttons, and back in the
    /// desktop layout the page gets its zoom back.
    #[test]
    fn the_mobile_layout_shows_on_a_phone_and_gives_way_to_the_desktop_one() {
        let mut a = app();
        a.run("ui.layout", json!({"value": "auto"})).unwrap();
        assert!(!a.mobile_layout(), "no screen measured yet");
        let mut h = harness(a, 390.0, 844.0);
        h.run_steps(5);
        assert!(h.state().mobile_layout());
        assert_eq!(h.state().session.view.fit, "pageWidth", "the page fits the screen's width");
        assert!(h.query_by_label("Bold").is_some(), "the Home tab's buttons");
        assert!(h.query_by_label("Pictures").is_none());
        h.get_by_label("Insert").click_accesskit();
        h.run_steps(3);
        assert_eq!(h.state().mobile.tab, Tab::Insert);
        assert!(h.query_by_label("Pictures").is_some() && h.query_by_label("Bold").is_none());
        h.state_mut().run("ui.layout", json!({"value": "desktop"})).unwrap();
        h.run_steps(4);
        let a = h.state();
        assert!(!a.mobile_layout());
        assert!(a.session.view.fit.is_empty() && (a.session.view.zoom - 1.0).abs() < 1e-6, "back to 100%");
    }

    /// The Backstage on a phone: the pages in a row along the top, and File › Options with its
    /// panes above the settings.
    #[test]
    fn the_backstage_fits_a_phone() {
        let mut a = app();
        a.run("ui.layout", json!({"value": "mobile"})).unwrap();
        a.run("ui.backstage", json!({"value": true, "page": "open"})).unwrap();
        let mut h = harness(a, 390.0, 844.0);
        h.run_steps(5);
        h.get_by_label("Options").click_accesskit();
        h.run_steps(4);
        assert_eq!(h.state().ui.backstage_page, "options");
        assert!(h.query_by_label("Interface layout:").is_some(), "File › Options › General");
    }

    /// A touch screen (the host says so, or egui has seen a touch) makes the mobile layout's page
    /// scroll under a finger; with a mouse a drag still selects.
    #[test]
    fn touch_scrolling_needs_the_mobile_layout_and_a_touch_screen() {
        let services = Services { touch_screen: Some(Box::new(|| Some(true))), ..Default::default() };
        let mut a = WordApp::new(Session::new(wordcraft_doc::Document::new()), services);
        a.run("ui.layout", json!({"value": "auto"})).unwrap();
        let mut h = harness(a, 844.0, 390.0);
        h.run_steps(5);
        assert!(h.state().mobile_layout(), "a phone held sideways");
        assert!(h.state().mobile.touch_scrolls());
        let mut m = app();
        m.run("ui.layout", json!({"value": "mobile"})).unwrap();
        let mut h = harness(m, 390.0, 844.0);
        h.run_steps(5);
        assert!(h.state().mobile_layout() && !h.state().mobile.touch_scrolls(), "no touch screen: a drag still selects");
    }
}
