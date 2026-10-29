//! Day by day: what's on, in time order, and where you sleep that night.

use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, CursorIcon, Rect, Sense, Stroke, Ui};
use jiff::civil::Date;
use waypoint_core::{Item, Kind, Status, Trip};

use crate::app::{today, Action};
use crate::theme;
use crate::ui;

#[derive(Default)]
pub struct Itinerary {
    /// Scroll this day into view on the next frame.
    pub scroll_to: Option<Date>,
}

/// One item as a row: time, kind, title and status, with lines of detail
/// under it (the details, a stay's address and phone, the time at home).
/// Returns true when clicked.
pub fn item_row(ui: &mut Ui, trip: &Trip, item: &Item, selected: bool) -> bool {
    let p = ui::pal(ui);
    let w = ui.available_width();
    let mut lines: Vec<(String, egui::Color32)> = Vec::new();
    let detail = detail_line(item);
    if !detail.is_empty() {
        lines.push((detail, p.ink2));
    }
    if item.kind == Kind::Stay {
        let reach: Vec<&str> = [item.address.trim(), item.phone.trim()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        if !reach.is_empty() {
            lines.push((reach.join(" · "), p.ink2));
        }
    }
    if let Some(home) = trip.timeline().home_times(item) {
        lines.push((format!("Home time: {home}"), p.trace));
    }
    let h = 30.0 + 20.0 * lines.len().max(1) as f32;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let painter = ui.painter_at(rect);
    if selected {
        painter.rect_filled(
            rect,
            CornerRadius::same(theme::RADIUS),
            theme::alpha(p.trace, 0.12),
        );
    } else if resp.hovered() {
        painter.rect_filled(rect, CornerRadius::same(theme::RADIUS), p.faint);
    }
    let cancelled = item.status == Status::Cancelled;
    let ink = if cancelled { p.ink2 } else { p.ink };
    let top = rect.top() + 16.0;
    let bottom = rect.top() + 36.0;

    let time = item
        .start
        .as_ref()
        .and_then(|s| s.time)
        .map(|t| t.strftime("%H:%M").to_string())
        .unwrap_or_default();
    painter.text(
        pos2(rect.left() + 10.0, top),
        Align2::LEFT_CENTER,
        &time,
        theme::semi(15.0),
        p.trace,
    );
    painter.rect_filled(
        Rect::from_center_size(pos2(rect.left() + 70.0, top), vec2(9.0, 9.0)),
        2,
        p.kind(item.kind),
    );

    // Status pill on the right, cost to its left.
    let status = item.status.label();
    let sfont = theme::medium(12.0);
    let sg = painter.layout_no_wrap(status.to_string(), sfont.clone(), p.status(item.status));
    let pill = Rect::from_min_size(
        pos2(rect.right() - 10.0 - sg.size().x - 16.0, top - 10.0),
        vec2(sg.size().x + 16.0, 20.0),
    );
    let fill = if item.status == Status::Paid {
        theme::alpha(p.ok, 0.14)
    } else {
        egui::Color32::TRANSPARENT
    };
    let border = if item.status == Status::Idea {
        p.grid
    } else {
        theme::alpha(p.status(item.status), 0.7)
    };
    painter.rect(
        pill,
        CornerRadius::same(10),
        fill,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    painter.text(
        pill.center(),
        Align2::CENTER_CENTER,
        status,
        sfont,
        p.status(item.status),
    );
    let mut right = pill.left() - 12.0;
    if !item.cost.is_empty() {
        let c = painter.text(
            pos2(right, top),
            Align2::RIGHT_CENTER,
            trip.cost_label(item),
            theme::body(14.0),
            p.ink2,
        );
        right = c.left() - 12.0;
    }

    let title_left = rect.left() + 84.0;
    let title = painter.layout_no_wrap(item.display_title(), theme::semi(15.0), ink);
    let max_w = (right - title_left).max(40.0);
    let clipped = painter.with_clip_rect(Rect::from_min_max(
        pos2(title_left, rect.top()),
        pos2(title_left + max_w, rect.bottom()),
    ));
    let tr = Rect::from_min_size(pos2(title_left, top - title.size().y / 2.0), title.size());
    clipped.galley(tr.min, title, ink);
    if cancelled {
        clipped.line_segment(
            [
                pos2(tr.left(), top),
                pos2(tr.right().min(title_left + max_w), top),
            ],
            Stroke::new(1.0, p.ink2),
        );
    }

    let clip = painter.with_clip_rect(Rect::from_min_max(
        pos2(title_left, rect.top()),
        pos2(rect.right() - 10.0, rect.bottom()),
    ));
    for (n, (text, color)) in lines.into_iter().enumerate() {
        clip.text(
            pos2(title_left, bottom + 20.0 * n as f32),
            Align2::LEFT_CENTER,
            text,
            theme::body(13.0),
            color,
        );
    }
    resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// The grey line under a row: route, place, reference.
fn detail_line(item: &Item) -> String {
    let mut parts: Vec<String> = Vec::new();
    match item.kind {
        Kind::Travel => {
            if let Some(t) = &item.travel {
                let mut s = t.mode.label().to_string();
                if !item.title.trim().is_empty() && (!t.from.is_empty() || !t.to.is_empty()) {
                    s.push_str(&format!(" {} to {}", t.from, t.to));
                }
                parts.push(s);
            }
            if let (Some(e), Some(et)) = (&item.end, item.end.as_ref().and_then(|e| e.time)) {
                let mut s = format!("arrives {}", et.strftime("%H:%M"));
                if item.date() != Some(e.date) {
                    s.push_str(&e.date.strftime(" %a %-d").to_string());
                }
                parts.push(s);
            }
        }
        Kind::Stay => {
            if let Some(e) = &item.end {
                let n = item.nights().len();
                parts.push(format!(
                    "{n} night{} to {}",
                    if n == 1 { "" } else { "s" },
                    e.date.strftime("%a %-d %b")
                ));
            }
        }
        _ => {
            if let Some(et) = item.end.as_ref().and_then(|e| e.time) {
                parts.push(format!("until {}", et.strftime("%H:%M")));
            }
        }
    }
    if !item.place.is_empty() && item.place != item.display_title() {
        parts.push(item.place.clone());
    }
    if !item.reference.is_empty() {
        parts.push(format!("Ref {}", item.reference));
    }
    parts.join(" · ")
}

/// The "+ Add" menu: one entry per kind.
pub fn add_menu(ui: &mut Ui, label: &str, day: Option<Date>, actions: &mut Vec<Action>) {
    let p = ui::pal(ui);
    let r = ui::link(ui, label);
    egui::Popup::menu(&r).show(|ui| {
        ui.set_min_width(140.0);
        for k in Kind::ALL {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(k.label())
                            .font(theme::body(14.0))
                            .color(p.ink),
                    )
                    .frame(false),
                )
                .clicked()
            {
                actions.push(Action::Add(k, day));
            }
        }
    });
}

impl Itinerary {
    pub fn ui(&mut self, ui: &mut Ui, trip: &Trip, open: Option<&str>, actions: &mut Vec<Action>) {
        let p = ui::pal(ui);
        let today = today();
        ui.horizontal(|ui| {
            ui::h1(ui, "Itinerary");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui::button(ui, "Phone itinerary", ui::Btn::Secondary)
                    .on_hover_text("Write the phone itinerary now and open it in your browser")
                    .clicked()
                {
                    actions.push(Action::ShowPhonePage);
                }
                if ui::button(ui, "Itinerary to share", ui::Btn::Secondary)
                    .on_hover_text(
                        "A page for family and friends: where you'll be and when, with no costs, \
                         booking references or notes. Print it from the browser to make a PDF.",
                    )
                    .clicked()
                {
                    actions.push(Action::ShowSharePage);
                }
                let pdf = ui::button(ui, "Save PDF", ui::Btn::Secondary).on_hover_text(
                    "Save the itinerary as a PDF, everything or the version to share",
                );
                egui::Popup::menu(&pdf).show(|ui| {
                    ui.set_min_width(220.0);
                    if ui.button("My itinerary, everything").clicked() {
                        actions.push(Action::SavePdf { shared: false });
                    }
                    if ui.button("The itinerary to share").clicked() {
                        actions.push(Action::SavePdf { shared: true });
                    }
                });
            });
        });
        let days = trip.days();
        if days.is_empty()
            && ui::empty(
                ui,
                "No dates yet",
                "Set the trip's first and last days, or give an item a date, and the days appear here.",
                Some("Set the dates"),
            ) {
                actions.push(Action::Go(crate::app::Route::Trip));
            }
        let start = trip.start.or(days.first().copied());
        for d in &days {
            if self.scroll_to == Some(*d) {
                ui.scroll_to_cursor(Some(egui::Align::TOP));
                self.scroll_to = None;
            }
            ui::panel(ui, |ui| {
                ui.horizontal(|ui| {
                    let is_today = *d == today;
                    ui::text(
                        ui,
                        d.strftime("%A %-d %B").to_string(),
                        theme::display(21.0),
                        if is_today { p.trace } else { p.ink },
                    );
                    if let Some(s) = start {
                        let n = s.until(*d).map(|x| x.get_days()).unwrap_or(0) + 1;
                        if n >= 1 {
                            ui::small(ui, format!("Day {n}"), p.ink2);
                        }
                    }
                    if is_today {
                        ui::small(ui, "Today", p.trace);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        add_menu(ui, "+ Add", Some(*d), actions);
                    });
                });
                ui.spacing_mut().item_spacing.y = 2.0;
                for item in trip.items_on(*d) {
                    if item_row(ui, trip, item, open == Some(item.id.as_str())) {
                        actions.push(Action::Open(item.id.clone()));
                    }
                }
                ui.add_space(6.0);
                let last_day = trip.end.is_some_and(|e| *d >= e);
                let stays = trip.stays_for_night(*d);
                ui.horizontal(|ui| {
                    if !stays.is_empty() {
                        ui::small(ui, "Tonight:", p.ink2);
                        for s in &stays {
                            if ui::link(ui, &s.display_title()).clicked() {
                                actions.push(Action::Open(s.id.clone()));
                            }
                        }
                    } else if !last_day && trip.start.is_some_and(|s| *d >= s) {
                        let on_the_move = !trip.travelling_overnight(*d).is_empty();
                        if on_the_move {
                            ui::small(ui, "Tonight: travelling overnight", p.ink2);
                        } else {
                            ui::small(ui, "Tonight: nowhere booked yet", p.warn);
                            if ui::link(ui, "Add a stay").clicked() {
                                actions.push(Action::Add(Kind::Stay, Some(*d)));
                            }
                        }
                    }
                });
            });
        }

        let undated = trip.undated();
        ui::panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui::text(ui, "No date yet", theme::display(21.0), p.ink);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    add_menu(ui, "+ Add", None, actions);
                });
            });
            if undated.is_empty() {
                ui::muted(ui, "Ideas without a day go here.");
            }
            ui.spacing_mut().item_spacing.y = 2.0;
            for item in undated {
                if item_row(ui, trip, item, open == Some(item.id.as_str())) {
                    actions.push(Action::Open(item.id.clone()));
                }
            }
        });
    }
}
