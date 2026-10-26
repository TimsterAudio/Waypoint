//! The trip's own details: name, dates, currency, budget, who's going, notes.

use eframe::egui::{self, Ui};
use waypoint_core::money;

use crate::app::{today, Action};
use crate::doc::Doc;
use crate::theme;
use crate::ui::{self, Btn};

/// A currency picker: the common ones, by code and name.
pub fn currency_select(ui: &mut Ui, id: &str, code: &mut String, home: &str) {
    let shown = if code.is_empty() {
        home.to_string()
    } else {
        code.clone()
    };
    egui::ComboBox::from_id_salt(id)
        .selected_text(egui::RichText::new(&shown).font(theme::medium(14.0)))
        .width(110.0)
        .height(420.0)
        .show_ui(ui, |ui| {
            for c in money::CURRENCIES {
                let label = format!("{}  {}", c.code, c.name);
                if ui
                    .selectable_label(
                        shown == c.code,
                        egui::RichText::new(label).font(theme::body(14.0)),
                    )
                    .clicked()
                {
                    *code = c.code.to_string();
                }
            }
        });
}

pub fn ui(ui: &mut Ui, doc: &mut Doc, actions: &mut Vec<Action>) {
    let p = ui::pal(ui);
    ui::h1(ui, "Trip details");
    let trip = &mut doc.trip;
    ui::panel(ui, |ui| {
        ui::field(ui, "Name", None, |ui| {
            ui::text_input(ui, &mut trip.name, "Japan 2027", 360.0)
        });
        let booked = trip.items.iter().filter(|i| i.status.committed()).count();
        ui::field(
            ui,
            "Stage",
            Some(match (trip.planning, booked) {
                (true, 0) => "Still planning: changing the first day moves the whole trip and every idea in it, keeping the trip's length.",
                (true, _) => "Still planning, but some things are booked, so changing the dates won't move anything. Move bookings one at a time.",
                (false, _) => "Booking: changing the trip's dates doesn't move anything in it.",
            }),
            |ui| {
                if let Some(k) = ui::chips(ui, &["Planning", "Booking"], Some(if trip.planning { 0 } else { 1 })) {
                    trip.planning = k == 0;
                }
            },
        );
        let can_move = trip.can_move();
        let end_before = trip.end;
        let mut moved: Option<(jiff::civil::Date, jiff::civil::Date)> = None;
        ui::field(
            ui,
            "Dates",
            Some("The first day and the day you get home. Nights between them with nowhere to stay are flagged on the Overview."),
            |ui| {
                ui.horizontal(|ui| {
                    let mut has = trip.start.is_some() && trip.end.is_some();
                    if ui.checkbox(&mut has, "").changed() {
                        if has {
                            let s = trip.start.unwrap_or_else(today);
                            trip.start = Some(s);
                            trip.end = Some(trip.end.unwrap_or_else(|| s.checked_add(jiff::Span::new().days(7)).unwrap_or(s)));
                        } else {
                            trip.start = None;
                            trip.end = None;
                        }
                    }
                    if let (Some(s), Some(e)) = (trip.start.as_mut(), trip.end.as_mut()) {
                        let first = *s;
                        ui::date_input(ui, "trip-start", s);
                        if *s != first && can_move {
                            moved = Some((first, *s));
                        }
                        ui::small(ui, "to", p.ink2);
                        ui::date_input(ui, "trip-end", e);
                        if *e < *s {
                            *e = *s;
                        }
                        let n = s.until(*e).map(|x| x.get_days()).unwrap_or(0);
                        ui::small(ui, format!("{n} night{}", if n == 1 { "" } else { "s" }), p.ink2);
                    } else {
                        ui::small(ui, "No dates yet", p.ink2);
                    }
                });
            },
        );
        // While planning with nothing booked, a new first day moves it all.
        if let Some((from, to)) = moved {
            let days = from.until(to).map(|x| x.get_days()).unwrap_or(0);
            trip.start = Some(from);
            trip.end = end_before;
            trip.move_by(days);
        }
        ui::field(
            ui,
            "Home currency",
            Some("What you finally pay in. Costs in other currencies are converted to it."),
            |ui| {
                let mut code = trip.home_currency.clone();
                currency_select(ui, "home-currency", &mut code, &trip.home_currency);
                if code != trip.home_currency && !code.is_empty() {
                    trip.home_currency = code;
                }
            },
        );
        ui::field(
            ui,
            "Home time zone",
            Some("Where home is. Bookings in other time zones also show what time it is at home."),
            |ui| {
                ui.horizontal(|ui| {
                    let system = jiff::tz::TimeZone::system()
                        .iana_name()
                        .unwrap_or("this computer's")
                        .to_string();
                    ui::text_input(ui, &mut trip.home_tz, &system, 240.0);
                    let menu = ui::link(ui, "Choose…");
                    egui::Popup::menu(&menu).show(|ui| {
                        egui::ScrollArea::vertical()
                            .max_height(320.0)
                            .show(ui, |ui| {
                                for z in waypoint_core::zones::COMMON {
                                    if ui.selectable_label(trip.home_tz == *z, *z).clicked() {
                                        trip.home_tz = z.to_string();
                                    }
                                }
                            });
                    });
                    let tz = trip.home_tz.trim();
                    if !tz.is_empty() && jiff::tz::TimeZone::get(tz).is_err() {
                        ui::small(ui, "Unknown zone", p.warn);
                    }
                });
            },
        );
        ui::field(
            ui,
            "Budget",
            Some("Optional. In the home currency."),
            |ui| {
                ui.horizontal(|ui| {
                    ui::amount_input(ui, egui::Id::new("budget"), &mut trip.budget, 0, 140.0);
                    ui::small(ui, &trip.home_currency, p.ink2);
                });
            },
        );
        ui::field(
            ui,
            "Who's going",
            Some("Names, separated by commas."),
            |ui| {
                // Keeps what's typed until the box is left, so a comma or
                // space at the end isn't tidied away before the next name.
                let shown = trip.travellers.join(", ");
                let (_, typed) =
                    ui::typed_input(ui, egui::Id::new("travellers"), shown, "Tim, Sam", 360.0);
                if let Some(s) = typed {
                    trip.travellers = s
                        .split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect();
                }
            },
        );
        ui::field(ui, "Notes", Some("Passport numbers, insurance policy, anything for the whole trip. These go on the phone itinerary too."), |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut trip.notes)
                    .font(theme::body(15.0))
                    .desired_width(f32::INFINITY)
                    .desired_rows(5)
                    .margin(egui::Margin::symmetric(10, 8)),
            );
        });
    });

    ui::panel(ui, |ui| {
        ui::h2(ui, "The file");
        match &doc.file {
            Some(f) => {
                ui::muted(ui, format!("Saved in {}", f.path.display()));
                ui::muted(ui, format!("Phone itinerary: {}", f.html_path().display()));
                ui.horizontal(|ui| {
                    if ui::button(ui, "Open the phone itinerary", Btn::Secondary).clicked() {
                        actions.push(Action::ShowPhonePage);
                    }
                    if ui::button(ui, "Itinerary to share", Btn::Secondary).clicked() {
                        actions.push(Action::ShowSharePage);
                    }
                    if ui::button(ui, "Save PDF…", Btn::Secondary).clicked() {
                        actions.push(Action::SavePdf { shared: false });
                    }
                    if ui::button(ui, "Save PDF to share…", Btn::Secondary).clicked() {
                        actions.push(Action::SavePdf { shared: true });
                    }
                    if ui::button(ui, "Save a copy as…", Btn::Secondary).clicked() {
                        actions.push(Action::SaveCopy);
                    }
                });
            }
            None => {
                ui::muted(ui, "The sample trip isn't saved anywhere. Save a copy to keep it, change it and get its phone itinerary.");
                if ui::button(ui, "Save a copy as…", Btn::Primary).clicked() {
                    actions.push(Action::SaveCopy);
                }
            }
        }
    });
}
