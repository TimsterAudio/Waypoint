//! The window: the rail on the left (name, sections, save status), the
//! current screen, and the item editor on the right when an item is open.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, pos2, vec2, Align2, CornerRadius, CursorIcon, Frame, Margin, Rect, Sense, Stroke,
    ThemePreference,
};
use waypoint_core::file::TripFile;
use waypoint_core::{Item, Kind, Rate, Trip};

use crate::doc::{Doc, SaveState};
use crate::settings::{Settings, ThemeChoice};
use crate::theme::{self, Palette};
use crate::ui;
use crate::views;

/// Rates found, and a message about any that weren't.
type Fetched = (std::collections::BTreeMap<String, Rate>, Option<String>);

const RAIL_W: f32 = 196.0;
const EDITOR_W: f32 = 440.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Overview,
    Itinerary,
    Bookings,
    Costs,
    Trip,
    Settings,
}

impl Route {
    const ALL: [Route; 6] = [
        Route::Overview,
        Route::Itinerary,
        Route::Bookings,
        Route::Costs,
        Route::Trip,
        Route::Settings,
    ];

    fn label(self) -> &'static str {
        match self {
            Route::Overview => "Overview",
            Route::Itinerary => "Itinerary",
            Route::Bookings => "Bookings",
            Route::Costs => "Costs",
            Route::Trip => "Trip details",
            Route::Settings => "Settings",
        }
    }

    fn needs_trip(self) -> bool {
        self != Route::Settings
    }
}

/// Something a screen asks the app to do after drawing.
pub enum Action {
    Go(Route),
    /// Open an item in the editor.
    Open(String),
    /// Add an item of this kind, on this day if given, and open it.
    Add(Kind, Option<jiff::civil::Date>),
    /// Show this day in the itinerary.
    Day(jiff::civil::Date),
    NewTrip,
    OpenTrip,
    OpenPath(PathBuf),
    Sample,
    SaveCopy,
    CloseTrip,
    FetchRates,
    ShowPhonePage,
    /// Write the itinerary to share and open it in the browser.
    ShowSharePage,
}

pub struct WaypointApp {
    pub settings: Settings,
    settings_path: Option<PathBuf>,
    doc: Option<Doc>,
    route: Route,
    /// The item open in the editor.
    open_item: Option<String>,
    editor: views::editor::Editor,
    itinerary: views::itinerary::Itinerary,
    bookings: views::bookings::Bookings,
    costs: views::costs::Costs,
    /// An exchange-rate fetch under way.
    rates_job: Option<mpsc::Receiver<Fetched>>,
    /// A message for the start screen, such as why a trip didn't open.
    message: Option<String>,
}

impl WaypointApp {
    pub fn new(
        cc: &eframe::CreationContext,
        settings: Settings,
        settings_path: Option<PathBuf>,
        open: Option<Open>,
    ) -> Self {
        theme::install(&cc.egui_ctx);
        apply_theme(&cc.egui_ctx, settings.theme);
        let mut app = WaypointApp {
            settings,
            settings_path,
            doc: None,
            route: Route::Overview,
            open_item: None,
            editor: Default::default(),
            itinerary: Default::default(),
            bookings: Default::default(),
            costs: Default::default(),
            rates_job: None,
            message: None,
        };
        match open {
            Some(Open::Sample) => app.open_sample(),
            Some(Open::Path(p)) => app.open_path(&p, &cc.egui_ctx),
            None => {}
        }
        app
    }

    fn save_settings(&mut self) {
        if let Err(e) = self.settings.save(self.settings_path.as_deref()) {
            self.message = Some(format!("Couldn't save Waypoint's settings: {e}"));
        }
    }

    fn close_doc(&mut self) {
        if let Some(mut d) = self.doc.take() {
            d.close(self.settings.phone_page);
        }
        self.open_item = None;
        self.rates_job = None;
    }

    fn set_doc(&mut self, doc: Doc, ctx: &egui::Context) {
        self.close_doc();
        self.doc = Some(doc);
        self.route = Route::Overview;
        self.itinerary = Default::default();
        self.bookings = Default::default();
        self.costs = Default::default();
        self.message = None;
        if self.settings.rates_on_open {
            self.fetch_rates(ctx);
        }
    }

    fn open_sample(&mut self) {
        let doc = Doc::new(waypoint_core::sample::trip(), None);
        self.close_doc();
        self.doc = Some(doc);
        self.route = Route::Overview;
    }

    pub fn open_path(&mut self, path: &Path, ctx: &egui::Context) {
        match TripFile::open(path) {
            Ok((file, trip)) => {
                self.settings.remember(path);
                self.save_settings();
                self.set_doc(Doc::new(trip, Some(file)), ctx);
            }
            Err(e) => {
                if !path.exists() {
                    self.settings.forget(path);
                    self.save_settings();
                }
                self.close_doc();
                self.message = Some(e.to_string());
            }
        }
    }

    fn new_trip(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Where to keep the new trip (your Google Drive folder is a good place)")
            .set_file_name("My trip.trip.json")
            .save_file()
        else {
            return;
        };
        let path = with_extension(path);
        let name = trip_name(&path);
        let trip = Trip::new(&name);
        match TripFile::create(&path, &trip) {
            Ok(file) => {
                self.settings.remember(&path);
                self.save_settings();
                let mut doc = Doc::new(trip, Some(file));
                doc.save(self.settings.phone_page);
                self.set_doc(doc, ctx);
                self.route = Route::Trip;
            }
            Err(e) => self.message = Some(e.to_string()),
        }
    }

    fn open_dialog(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Open a trip")
            .add_filter("Trips", &["json"])
            .pick_file()
        {
            self.open_path(&path, ctx);
        }
    }

    /// Save the open trip (usually the sample) as a new file.
    fn save_copy(&mut self, ctx: &egui::Context) {
        let Some(doc) = &self.doc else { return };
        let suggested = TripFile::file_name(&doc.trip.name.replace(" (sample)", ""));
        let Some(path) = rfd::FileDialog::new()
            .set_title("Save a copy of this trip")
            .set_file_name(&suggested)
            .save_file()
        else {
            return;
        };
        let path = with_extension(path);
        let mut trip = doc.trip.clone();
        trip.name = trip_name(&path);
        // A save dialog has already asked about replacing a file.
        let _ = std::fs::remove_file(&path);
        match TripFile::create(&path, &trip) {
            Ok(file) => {
                self.settings.remember(&path);
                self.save_settings();
                let mut doc = Doc::new(trip, Some(file));
                doc.save(self.settings.phone_page);
                self.set_doc(doc, ctx);
            }
            Err(e) => {
                if let Some(d) = &mut self.doc {
                    d.toast(e.to_string());
                }
            }
        }
    }

    fn fetch_rates(&mut self, ctx: &egui::Context) {
        let Some(doc) = &self.doc else { return };
        let wanted = doc.trip.foreign_currencies();
        if wanted.is_empty() || self.rates_job.is_some() {
            return;
        }
        let home = doc.trip.home_currency.clone();
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(waypoint_core::rates::fetch(&home, &wanted));
            ctx.request_repaint();
        });
        self.rates_job = Some(rx);
    }

    fn poll_rates(&mut self) {
        let Some(rx) = &self.rates_job else { return };
        let Ok((rates, message)) = rx.try_recv() else {
            return;
        };
        self.rates_job = None;
        let Some(doc) = &mut self.doc else { return };
        let before = doc.trip.clone();
        let n = rates.len();
        doc.trip.rates.extend(rates);
        if doc.trip != before {
            doc.edited(&before);
        }
        match message {
            Some(m) => doc.toast(m),
            None => doc.toast(format!(
                "Updated {n} exchange rate{}",
                if n == 1 { "" } else { "s" }
            )),
        }
    }

    fn do_action(&mut self, a: Action, ctx: &egui::Context) {
        match a {
            Action::Go(r) => self.route = r,
            Action::Open(id) => self.open_item = Some(id),
            Action::Add(kind, day) => {
                if let Some(doc) = &mut self.doc {
                    let before = doc.trip.clone();
                    let mut item = Item::new(kind);
                    item.start = day.map(waypoint_core::When::day);
                    if kind == Kind::Stay {
                        item.end = day
                            .and_then(|d| d.tomorrow().ok())
                            .map(waypoint_core::When::day);
                    }
                    self.open_item = Some(item.id.clone());
                    self.editor.focus_title = true;
                    doc.trip.items.push(item);
                    doc.edited(&before);
                }
            }
            Action::Day(d) => {
                self.route = Route::Itinerary;
                self.itinerary.scroll_to = Some(d);
            }
            Action::NewTrip => self.new_trip(ctx),
            Action::OpenTrip => self.open_dialog(ctx),
            Action::OpenPath(p) => self.open_path(&p, ctx),
            Action::Sample => self.open_sample(),
            Action::SaveCopy => self.save_copy(ctx),
            Action::CloseTrip => self.close_doc(),
            Action::FetchRates => self.fetch_rates(ctx),
            Action::ShowPhonePage => {
                let Some(doc) = &mut self.doc else { return };
                match doc.write_phone_page() {
                    Some(p) => {
                        ctx.open_url(egui::OpenUrl::new_tab(file_url(&p)));
                    }
                    None if doc.is_sample() => doc.toast("Save a copy of the sample first; the phone itinerary goes beside the trip file."),
                    None => {}
                }
            }
            Action::ShowSharePage => {
                let Some(doc) = &mut self.doc else { return };
                if let Some(p) = doc.write_share_page() {
                    ctx.open_url(egui::OpenUrl::new_tab(file_url(&p)));
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("the page");
                    doc.toast(format!(
                        "Opened \"{name}\" in your browser. To make a PDF, print it and choose Save as PDF."
                    ));
                }
            }
        }
    }

    // ------------------------------------------------------------ the rail

    fn rail(&mut self, ui: &mut egui::Ui, height: f32) -> Option<Action> {
        let p = Palette::of(ui.ctx());
        let top = ui.max_rect().top();
        let mut action = None;
        ui.add_space(26.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let (r, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
            mark(ui.painter(), r.center(), &p);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Waypoint")
                    .font(theme::display(26.0))
                    .color(p.ink),
            );
        });
        ui.add_space(10.0);
        if let Some(doc) = &self.doc {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&doc.trip.name)
                            .font(theme::semi(14.0))
                            .color(p.ink2),
                    )
                    .truncate(),
                );
            });
        }
        ui.add_space(12.0);
        let warnings = self
            .doc
            .as_ref()
            .map(|d| {
                waypoint_core::checks::check(&d.trip, today())
                    .iter()
                    .filter(|w| w.severity == waypoint_core::checks::Severity::Act)
                    .count()
            })
            .unwrap_or(0);
        for route in Route::ALL {
            let enabled = !route.needs_trip() || self.doc.is_some();
            let on =
                self.route == route && (enabled || self.doc.is_none() && route == Route::Settings);
            let (rect, resp) = ui.allocate_exact_size(
                vec2(ui.available_width(), 38.0),
                if enabled {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            let hovered = resp.hovered() && enabled;
            if on || hovered {
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(theme::RADIUS),
                    if on { p.sheet } else { p.faint },
                );
            }
            if on {
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        pos2(rect.left() + 4.0, rect.top() + 10.0),
                        vec2(3.0, rect.height() - 20.0),
                    ),
                    2,
                    p.trace,
                );
            }
            let color = if !enabled {
                theme::alpha(p.ink2, 0.45)
            } else if on || hovered {
                p.ink
            } else {
                p.ink2
            };
            let label = ui.painter().text(
                pos2(rect.left() + 14.0, rect.center().y),
                Align2::LEFT_CENTER,
                route.label(),
                theme::medium(15.0),
                color,
            );
            if route == Route::Overview && warnings > 0 {
                ui.painter().circle_filled(
                    pos2(label.right() + 8.0, label.center().y),
                    3.5,
                    p.warn,
                );
            }
            if enabled && resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                action = Some(Action::Go(route));
            }
            ui.add_space(2.0);
        }
        if self.doc.is_some() {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                if ui::link(ui, "Close trip").clicked() {
                    action = Some(Action::CloseTrip);
                }
            });
        }

        // Save status, at the bottom.
        let (dot, label, tip) = self.status_line(&p);
        let bottom = top + height - 24.0;
        let at = pos2(ui.max_rect().left() + 10.0, bottom);
        let resp = ui.interact(
            Rect::from_min_size(pos2(at.x - 4.0, bottom - 10.0), vec2(RAIL_W - 24.0, 20.0)),
            ui.id().with("rail-status"),
            Sense::hover(),
        );
        ui.painter().circle_filled(at, 4.0, dot);
        ui.painter().text(
            at + vec2(12.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            theme::body(13.0),
            p.ink2,
        );
        if let Some(t) = tip {
            resp.on_hover_text(t);
        }
        action
    }

    fn status_line(&self, p: &Palette) -> (egui::Color32, String, Option<String>) {
        let Some(doc) = &self.doc else {
            return (p.ink2, "No trip open".into(), None);
        };
        let Some(file) = &doc.file else {
            return (p.ink2, "Sample: not saved".into(), None);
        };
        let where_ = Some(file.path.display().to_string());
        if doc.conflict {
            return (p.heat, "Changed elsewhere".into(), where_);
        }
        match &doc.state {
            SaveState::Saved(at) => (p.ok, format!("Saved {}", ago(*at)), where_),
            SaveState::Unsaved => (p.warn, "Saving…".into(), where_),
            SaveState::Failed(e, _) => (p.heat, "Not saved".into(), Some(e.clone())),
        }
    }
}

fn ago(at: Instant) -> String {
    let s = at.elapsed().as_secs();
    match s {
        0..=9 => "just now".into(),
        10..=59 => format!("{s} s ago"),
        60..=3599 => format!("{} min ago", s / 60),
        _ => format!("{} h ago", s / 3600),
    }
}

/// Waypoint's mark: a waypoint diamond in ink round a violet fix.
pub fn mark(painter: &egui::Painter, c: egui::Pos2, p: &Palette) {
    let r = 10.5;
    let pts = vec![
        c + vec2(0.0, -r),
        c + vec2(r, 0.0),
        c + vec2(0.0, r),
        c + vec2(-r, 0.0),
    ];
    painter.add(egui::Shape::closed_line(pts, Stroke::new(2.0, p.ink)));
    painter.circle_filled(c, 3.5, p.trace);
}

pub fn today() -> jiff::civil::Date {
    jiff::Zoned::now().date()
}

fn with_extension(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if s.ends_with(waypoint_core::file::EXTENSION) {
        path
    } else {
        let s = s.strip_suffix(".json").unwrap_or(&s).to_string();
        PathBuf::from(format!("{s}{}", waypoint_core::file::EXTENSION))
    }
}

fn trip_name(path: &Path) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Trip");
    name.strip_suffix(waypoint_core::file::EXTENSION)
        .unwrap_or(name)
        .to_string()
}

fn file_url(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let s = s.replace(' ', "%20").replace('#', "%23");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

pub fn apply_theme(ctx: &egui::Context, t: ThemeChoice) {
    ctx.set_theme(match t {
        ThemeChoice::System => ThemePreference::System,
        ThemeChoice::Light => ThemePreference::Light,
        ThemeChoice::Dark => ThemePreference::Dark,
    });
}

/// What to open at startup.
pub enum Open {
    Sample,
    Path(PathBuf),
}

impl eframe::App for WaypointApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let p = Palette::of(&ctx);
        self.poll_rates();
        let wake = self
            .doc
            .as_mut()
            .and_then(|d| d.tick(self.settings.phone_page));
        // Keep "saved 5 s ago" and the file check ticking.
        ctx.request_repaint_after(
            wake.unwrap_or(Duration::from_secs(1))
                .min(Duration::from_secs(1)),
        );
        if self.route.needs_trip() && self.doc.is_none() {
            self.route = Route::Overview;
        }
        // An item that's gone (deleted, or removed elsewhere) closes.
        if let (Some(id), Some(doc)) = (&self.open_item, &self.doc) {
            if doc.trip.item(id).is_none() {
                self.open_item = None;
            }
        }
        if self.doc.is_none() {
            self.open_item = None;
        }

        let screen = ctx.content_rect();
        let mut actions: Vec<Action> = Vec::new();
        egui::Panel::left("rail")
            .exact_size(RAIL_W)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(p.paper)
                    .inner_margin(Margin::symmetric(12, 0)),
            )
            .show(ui, |ui| {
                if let Some(a) = self.rail(ui, screen.height()) {
                    actions.push(a);
                }
            });
        ui.painter().line_segment(
            [pos2(RAIL_W - 0.5, 0.0), pos2(RAIL_W - 0.5, screen.bottom())],
            Stroke::new(1.0, p.grid),
        );

        let before = self.doc.as_ref().map(|d| d.trip.clone());

        if let (Some(id), Some(doc)) = (self.open_item.clone(), self.doc.as_mut()) {
            let mut close = false;
            egui::Panel::right("editor")
                .exact_size(EDITOR_W)
                .resizable(false)
                .show_separator_line(true)
                .frame(Frame::new().fill(p.sheet).inner_margin(Margin {
                    left: 22,
                    right: 22,
                    top: 20,
                    bottom: 20,
                }))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("editor-scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| match self.editor.ui(ui, doc, &id) {
                            views::editor::Outcome::None => {}
                            views::editor::Outcome::Close => close = true,
                            views::editor::Outcome::Deleted => {
                                close = true;
                            }
                            views::editor::Outcome::Duplicated(new_id) => {
                                actions.push(Action::Open(new_id));
                            }
                        });
                });
            if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.open_item = None;
            }
        }

        egui::CentralPanel::no_frame()
            .frame(Frame::new().fill(p.paper))
            .show(ui, |ui| {
                if let Some(doc) = &mut self.doc {
                    banners(ui, doc, self.settings.phone_page);
                }
                egui::ScrollArea::vertical()
                    .id_salt(self.route.label())
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let avail = ui.available_width();
                        let width = (avail - 64.0).clamp(300.0, 1180.0);
                        let side = ((avail - width) / 2.0).max(0.0);
                        ui.add_space(24.0);
                        ui.horizontal_top(|ui| {
                            ui.add_space(side);
                            ui.vertical(|ui| {
                                ui.set_width(width);
                                ui.spacing_mut().item_spacing.y = 20.0;
                                let open = self.open_item.as_deref();
                                match (&mut self.doc, self.route) {
                                    (_, Route::Settings) => {
                                        if views::settings::ui(ui, &mut self.settings) {
                                            apply_theme(&ctx, self.settings.theme);
                                            self.save_settings();
                                        }
                                    }
                                    (None, _) => views::start::ui(
                                        ui,
                                        &self.settings,
                                        self.message.as_deref(),
                                        &mut actions,
                                    ),
                                    (Some(doc), Route::Overview) => {
                                        views::overview::ui(ui, &doc.trip, &mut actions)
                                    }
                                    (Some(doc), Route::Itinerary) => {
                                        self.itinerary.ui(ui, &doc.trip, open, &mut actions)
                                    }
                                    (Some(doc), Route::Bookings) => {
                                        self.bookings.ui(ui, &doc.trip, open, &mut actions)
                                    }
                                    (Some(doc), Route::Costs) => self.costs.ui(
                                        ui,
                                        &mut doc.trip,
                                        self.rates_job.is_some(),
                                        open,
                                        &mut actions,
                                    ),
                                    (Some(doc), Route::Trip) => {
                                        views::trip::ui(ui, doc, &mut actions)
                                    }
                                }
                            });
                        });
                        ui.add_space(48.0);
                    });
                if let Some(doc) = &mut self.doc {
                    toasts(ui, doc, &mut self.open_item);
                }
            });

        if let (Some(doc), Some(before)) = (&mut self.doc, before) {
            if std::mem::take(&mut doc.reloaded) {
                // Replaced from disk: not an edit.
            } else if doc.trip != before {
                doc.edited(&before);
            }
        }
        for a in actions {
            self.do_action(a, &ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.close_doc();
    }
}

/// Notices across the top: the sample, someone else has it open, a clash.
fn banners(ui: &mut egui::Ui, doc: &mut Doc, phone_page: bool) {
    let p = Palette::of(ui.ctx());
    let bar = |ui: &mut egui::Ui,
               color: egui::Color32,
               text: String,
               buttons: &[&str]|
     -> Option<usize> {
        let mut clicked = None;
        Frame::new()
            .fill(theme::mix(p.paper, color, 0.12))
            .inner_margin(Margin::symmetric(24, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(text)
                            .font(theme::body(14.0))
                            .color(p.ink),
                    );
                    for (i, b) in buttons.iter().enumerate() {
                        if ui::button(
                            ui,
                            b,
                            if i == 0 {
                                ui::Btn::Primary
                            } else {
                                ui::Btn::Secondary
                            },
                        )
                        .clicked()
                        {
                            clicked = Some(i);
                        }
                    }
                });
            });
        clicked
    };
    if doc.is_sample() {
        bar(
            ui,
            p.trace,
            "This is the sample trip. Changes to it aren't saved.".into(),
            &[],
        );
    }
    if doc.conflict {
        let r = bar(
            ui,
            p.heat,
            "Someone else saved this trip while you had changes that weren't saved yet.".into(),
            &["Load theirs, keep mine as a copy", "Save mine over theirs"],
        );
        match r {
            Some(0) => doc.take_theirs(),
            Some(1) => doc.keep_mine(phone_page),
            _ => {}
        }
    } else if let Some(lock) = doc.elsewhere.clone() {
        let since = jiff::Timestamp::from_second(lock.since)
            .map(|t| {
                t.to_zoned(jiff::tz::TimeZone::system())
                    .strftime("%-d %b at %H:%M")
                    .to_string()
            })
            .unwrap_or_default();
        let r = bar(
            ui,
            p.warn,
            format!(
                "{} has had this trip open since {since}. If you both change it, Waypoint will ask whose changes to keep.",
                lock.who
            ),
            &["OK"],
        );
        if r == Some(0) {
            doc.elsewhere = None;
        }
    }
}

/// The message at the bottom: "Deleted … Undo", "Updated with changes…".
fn toasts(ui: &mut egui::Ui, doc: &mut Doc, open_item: &mut Option<String>) {
    let p = Palette::of(ui.ctx());
    let rect = ui.max_rect();
    let mut text = None;
    let mut undo = false;
    if let Some((item, _, at)) = &doc.deleted {
        if at.elapsed() < Duration::from_secs(12) {
            text = Some(format!("Deleted {}", item.display_title()));
            undo = true;
        } else {
            doc.deleted = None;
        }
    }
    if text.is_none() {
        if let Some((t, at)) = &doc.toast {
            if at.elapsed() < Duration::from_secs(6) {
                text = Some(t.clone());
            } else {
                doc.toast = None;
            }
        }
    }
    let Some(text) = text else { return };
    egui::Area::new(egui::Id::new("toast"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -24.0))
        .constrain_to(rect)
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(p.ink)
                .corner_radius(CornerRadius::same(theme::RADIUS))
                .inner_margin(Margin::symmetric(16, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(text)
                                .font(theme::body(14.0))
                                .color(p.paper),
                        );
                        if undo {
                            let r = ui.add(
                                egui::Label::new(
                                    egui::RichText::new("Undo").font(theme::semi(14.0)).color(
                                        if p.dark {
                                            theme::LIGHT.trace
                                        } else {
                                            theme::DARK.trace
                                        },
                                    ),
                                )
                                .sense(Sense::click()),
                            );
                            if r.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                *open_item = doc.undo_delete();
                            }
                        }
                    });
                });
        });
}
