//! The item editor on the right: everything about one stay, journey,
//! activity or meal. Changes apply as they're typed and save by themselves.

use eframe::egui::{self, Ui};
use jiff::civil::Date;
use waypoint_core::{money, Kind, Mode, Status, Travel, When};

use crate::app::today;
use crate::doc::Doc;
use crate::theme;
use crate::ui::{self, Btn};
use crate::views::trip::currency_select;

#[derive(Default)]
pub struct Editor {
    /// Put the cursor in the title (a new item).
    pub focus_title: bool,
}

pub enum Outcome {
    None,
    Close,
    Deleted,
    Duplicated(String),
}

/// Common time zones, for the zone menu. Any IANA name can be typed.
const ZONES: &[&str] = &[
    "Australia/Sydney",
    "Australia/Melbourne",
    "Australia/Brisbane",
    "Australia/Adelaide",
    "Australia/Perth",
    "Australia/Hobart",
    "Australia/Darwin",
    "Pacific/Auckland",
    "Pacific/Fiji",
    "Asia/Tokyo",
    "Asia/Seoul",
    "Asia/Shanghai",
    "Asia/Hong_Kong",
    "Asia/Taipei",
    "Asia/Singapore",
    "Asia/Kuala_Lumpur",
    "Asia/Bangkok",
    "Asia/Ho_Chi_Minh",
    "Asia/Jakarta",
    "Asia/Makassar",
    "Asia/Manila",
    "Asia/Kolkata",
    "Asia/Dubai",
    "Europe/London",
    "Europe/Dublin",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Rome",
    "Europe/Madrid",
    "Europe/Amsterdam",
    "Europe/Zurich",
    "Europe/Vienna",
    "Europe/Prague",
    "Europe/Athens",
    "Europe/Istanbul",
    "Atlantic/Reykjavik",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "America/Vancouver",
    "America/Toronto",
    "Pacific/Honolulu",
    "UTC",
];

fn label_pair(kind: Kind) -> (&'static str, &'static str) {
    match kind {
        Kind::Stay => ("Check in", "Check out"),
        Kind::Travel => ("Departs", "Arrives"),
        _ => ("Starts", "Ends"),
    }
}

/// A day, time and zone that may not be set yet.
fn when_input(ui: &mut Ui, id: &str, label: &str, when: &mut Option<When>, default: Date) {
    let p = ui::pal(ui);
    ui.label(
        egui::RichText::new(label)
            .font(theme::semi(14.0))
            .color(p.ink),
    );
    ui.horizontal(|ui| {
        let mut has = when.is_some();
        if ui
            .checkbox(&mut has, "")
            .on_hover_text("Has a date")
            .changed()
        {
            *when = has.then(|| When::day(default));
        }
        match when {
            Some(w) => {
                ui::date_input(ui, &format!("{id}-date"), &mut w.date);
                ui::time_input(ui, egui::Id::new((id, "time")), &mut w.time);
            }
            None => {
                ui::small(ui, "No date yet", p.ink2);
            }
        }
    });
    if let Some(w) = when {
        ui.horizontal(|ui| {
            ui.add_space(26.0);
            let r = ui::text_input(ui, &mut w.tz, "Time zone (optional)", 196.0);
            r.on_hover_text("Only needed when a journey crosses time zones, so its length comes out right. For example Asia/Tokyo.");
            let menu = ui::link(ui, "Choose…");
            egui::Popup::menu(&menu).show(|ui| {
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    if ui.selectable_label(w.tz.is_empty(), "None (local time)").clicked() {
                        w.tz.clear();
                    }
                    for z in ZONES {
                        if ui.selectable_label(w.tz == *z, *z).clicked() {
                            w.tz = z.to_string();
                        }
                    }
                });
            });
            if !w.tz.is_empty() && w.zone().is_none() {
                ui::small(ui, "Unknown zone", p.warn);
            }
        });
    }
    ui.add_space(4.0);
}

fn opt_date(ui: &mut Ui, id: &str, label: &str, value: &mut Option<Date>, default: Date) {
    let p = ui::pal(ui);
    ui.horizontal(|ui| {
        form_label(ui, label);
        let mut has = value.is_some();
        if ui.checkbox(&mut has, "").changed() {
            *value = has.then_some(value.unwrap_or(default));
        }
        match value {
            Some(d) => {
                ui::date_input(ui, id, d);
            }
            None => {
                ui::small(ui, "None", p.ink2);
            }
        }
    });
}

/// A labelled one-line field in a two-column form.
fn line(ui: &mut Ui, label: &str, value: &mut String, hint: &str) {
    ui.horizontal(|ui| {
        form_label(ui, label);
        ui::text_input(ui, value, hint, ui.available_width());
    });
}

/// The label to the left of a form field, in a fixed-width column.
fn form_label(ui: &mut Ui, text: &str) {
    let p = ui::pal(ui);
    ui.allocate_ui_with_layout(
        egui::vec2(100.0, 20.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(100.0);
            ui::small(ui, text, p.ink2);
        },
    );
}

fn section(ui: &mut Ui, title: &str) {
    let p = ui::pal(ui);
    ui.add_space(8.0);
    ui::rule(ui, p.grid);
    ui.label(
        egui::RichText::new(title)
            .font(theme::display(18.0))
            .color(p.ink),
    );
}

impl Editor {
    pub fn ui(&mut self, ui: &mut Ui, doc: &mut Doc, id: &str) -> Outcome {
        let p = ui::pal(ui);
        let Some(original) = doc.trip.item(id).cloned() else {
            return Outcome::Close;
        };
        let mut item = original.clone();
        let mut outcome = Outcome::None;
        let mut new_status = None;
        let default_day = item.date().or(doc.trip.start).unwrap_or_else(today);
        ui.spacing_mut().item_spacing.y = 8.0;

        ui.horizontal(|ui| {
            ui::kind_mark(ui, p.kind(item.kind));
            ui::text(ui, item.kind.label(), theme::display(22.0), p.ink);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui::link(ui, "Close").on_hover_text("Esc").clicked() {
                    outcome = Outcome::Close;
                }
            });
        });
        let hint = match item.kind {
            Kind::Stay => "Hotel name",
            Kind::Travel => "Optional: worked out from the details below",
            Kind::Meal => "Restaurant or meal",
            _ => "What it is",
        };
        let r = ui.add(
            egui::TextEdit::singleline(&mut item.title)
                .hint_text(hint)
                .font(theme::semi(17.0))
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(10, 8)),
        );
        if std::mem::take(&mut self.focus_title) {
            r.request_focus();
        }

        let kinds: Vec<&str> = Kind::ALL.iter().map(|k| k.label()).collect();
        if let Some(i) = ui::chips(ui, &kinds, Kind::ALL.iter().position(|k| *k == item.kind)) {
            item.kind = Kind::ALL[i];
            if item.kind == Kind::Travel && item.travel.is_none() {
                item.travel = Some(Travel::default());
            }
        }
        let statuses: Vec<&str> = Status::ALL.iter().map(|s| s.label()).collect();
        if let Some(i) = ui::chips(
            ui,
            &statuses,
            Status::ALL.iter().position(|s| *s == item.status),
        ) {
            new_status = Some(Status::ALL[i]);
        }

        section(ui, "When");
        let (a, b) = label_pair(item.kind);
        when_input(ui, &format!("{id}-start"), a, &mut item.start, default_day);
        let end_default = item
            .start
            .as_ref()
            .map(|s| {
                if item.kind == Kind::Stay {
                    s.date.tomorrow().unwrap_or(s.date)
                } else {
                    s.date
                }
            })
            .unwrap_or(default_day);
        when_input(ui, &format!("{id}-end"), b, &mut item.end, end_default);
        if let Some(m) = item.duration_minutes() {
            if item.kind != Kind::Stay {
                ui::small(
                    ui,
                    format!("{}h {:02}m", m / 60, m % 60),
                    if m < 0 { p.heat } else { p.ink2 },
                );
            }
        }
        if item.kind == Kind::Stay {
            let n = item.nights().len();
            if n > 0 {
                ui::small(
                    ui,
                    format!("{n} night{}", if n == 1 { "" } else { "s" }),
                    p.ink2,
                );
            }
        }

        if item.kind == Kind::Travel {
            section(ui, "Journey");
            let t = item.travel.get_or_insert_with(Travel::default);
            ui.horizontal(|ui| {
                form_label(ui, "By");
                let opts: Vec<(Mode, String)> = Mode::ALL
                    .iter()
                    .map(|m| (*m, m.label().to_string()))
                    .collect();
                ui::select(ui, &format!("{id}-mode"), &mut t.mode, &opts, 140.0);
            });
            let (carrier, number) = match t.mode {
                Mode::Flight => ("Airline", "Flight number"),
                Mode::Train => ("Train", "Train number"),
                Mode::Car => ("Hire firm", "Car / class"),
                _ => ("Company", "Number"),
            };
            line(ui, carrier, &mut t.carrier, "");
            line(ui, number, &mut t.number, "");
            line(ui, "From", &mut t.from, "");
            line(ui, "To", &mut t.to, "");
            line(
                ui,
                if t.mode == Mode::Flight {
                    "Terminal"
                } else {
                    "Platform"
                },
                &mut t.terminal,
                "",
            );
            line(ui, "Seats", &mut t.seat, "");
        }

        section(ui, "Where");
        line(ui, "Place", &mut item.place, "Name of the place");
        line(ui, "Address", &mut item.address, "Street, town");
        if item.kind == Kind::Stay {
            line(ui, "Room", &mut item.room, "");
        }

        section(ui, "Booking");
        line(
            ui,
            "Booked with",
            &mut item.provider,
            "Hotel, agent, website",
        );
        line(
            ui,
            "Reference",
            &mut item.reference,
            "Booking or confirmation number",
        );
        line(ui, "Contact", &mut item.contact, "Person or desk");
        line(ui, "Phone", &mut item.phone, "+81 3 …");
        line(ui, "Email", &mut item.email, "");
        line(ui, "Website", &mut item.website, "");

        section(ui, "Cost");
        let home = doc.trip.home_currency.clone();
        let code = if item.cost.currency.is_empty() {
            home.clone()
        } else {
            item.cost.currency.to_uppercase()
        };
        let decimals = money::currency(&code).map(|c| c.decimals).unwrap_or(2);
        ui.horizontal(|ui| {
            form_label(ui, "Total");
            let mut amount = (item.cost.amount != 0.0).then_some(item.cost.amount);
            ui::amount_input(
                ui,
                egui::Id::new((id, "amount")),
                &mut amount,
                decimals,
                130.0,
            );
            item.cost.amount = amount.unwrap_or(0.0);
            let mut c = item.cost.currency.clone();
            currency_select(ui, &format!("{id}-currency"), &mut c, &home);
            if c != item.cost.currency {
                item.cost.currency = if c == home { String::new() } else { c };
                item.cost.locked_rate = None;
            }
        });
        ui.horizontal(|ui| {
            form_label(ui, "Paid so far");
            let mut paid = (item.cost.paid != 0.0).then_some(item.cost.paid);
            ui::amount_input(ui, egui::Id::new((id, "paid")), &mut paid, decimals, 130.0);
            item.cost.paid = paid.unwrap_or(0.0);
            if item.cost.amount > 0.0
                && item.cost.paid < item.cost.amount
                && ui::link(ui, "All of it").clicked()
            {
                item.cost.paid = item.cost.amount;
            }
        });
        opt_date(
            ui,
            &format!("{id}-due"),
            "Rest due",
            &mut item.cost.due,
            default_day,
        );
        opt_date(
            ui,
            &format!("{id}-cancel"),
            "Free cancel by",
            &mut item.cost.cancel_by,
            default_day,
        );

        // What it comes to at home.
        if code != home && item.cost.amount > 0.0 {
            let trip = &doc.trip;
            match trip.rate_for(&item) {
                money::RateUsed::Locked(r) => {
                    ui.horizontal(|ui| {
                        ui::small(
                            ui,
                            format!(
                                "{} at the rate when paid (1 {code} = {r:.6} {home})",
                                money::format(item.cost.amount * r, &home)
                            ),
                            p.ink2,
                        );
                        if ui::link(ui, "Use today's rate").clicked() {
                            item.cost.locked_rate = None;
                        }
                    });
                }
                money::RateUsed::Current(r) => {
                    ui::small(
                        ui,
                        format!(
                            "About {} at today's rate",
                            money::format(item.cost.amount * r, &home)
                        ),
                        p.ink2,
                    );
                }
                money::RateUsed::Missing => {
                    ui::small(
                        ui,
                        format!("No {code} rate yet: add one on the Costs screen."),
                        p.warn,
                    );
                }
                money::RateUsed::Home => {}
            }
        }

        section(ui, "Notes");
        ui.add(
            egui::TextEdit::multiline(&mut item.notes)
                .hint_text("Anything to remember: what's included, how to get there, what to bring")
                .font(theme::body(15.0))
                .desired_width(f32::INFINITY)
                .desired_rows(4)
                .margin(egui::Margin::symmetric(10, 8)),
        );

        ui.add_space(8.0);
        ui::rule(ui, p.grid);
        if let Some(when) = original.updated {
            let at = when
                .to_zoned(jiff::tz::TimeZone::system())
                .strftime("%-d %b %Y at %H:%M")
                .to_string();
            let who = if original.updated_by.is_empty() {
                String::new()
            } else {
                format!(" by {}", original.updated_by)
            };
            ui::small(ui, format!("Last changed {at}{who}"), p.ink2);
        }
        let mut duplicate = false;
        let mut delete = false;
        ui.horizontal(|ui| {
            if ui::button(ui, "Duplicate", Btn::Secondary).clicked() {
                duplicate = true;
            }
            if ui::button(ui, "Delete", Btn::Danger).clicked() {
                delete = true;
            }
        });

        // Apply.
        if item != original {
            if let Some(slot) = doc.trip.item_mut(id) {
                *slot = item.clone();
            }
        }
        if let Some(s) = new_status {
            doc.trip.set_status(id, s);
        }
        if duplicate {
            let mut copy = item.clone();
            copy.id = waypoint_core::model::new_id();
            copy.title = format!("{} (copy)", item.display_title());
            copy.reference.clear();
            copy.cost.paid = 0.0;
            copy.cost.locked_rate = None;
            if copy.status == Status::Paid
                || copy.status == Status::Confirmed
                || copy.status == Status::Booked
            {
                copy.status = Status::Idea;
            }
            let new_id = copy.id.clone();
            let at = doc
                .trip
                .items
                .iter()
                .position(|i| i.id == id)
                .map(|k| k + 1)
                .unwrap_or(doc.trip.items.len());
            doc.trip.items.insert(at, copy);
            outcome = Outcome::Duplicated(new_id);
        }
        if delete {
            doc.delete(id);
            outcome = Outcome::Deleted;
        }
        outcome
    }
}
