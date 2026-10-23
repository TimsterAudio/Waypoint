//! The trip at a glance: how long to go, the money, what needs doing, and
//! what's next.

use eframe::egui::{self, Ui};
use waypoint_core::checks::{self, Severity};
use waypoint_core::{money, Trip};

use crate::app::{today, Action};
use crate::theme;
use crate::ui::{self, Stat};

/// "In 204 days", "Day 3 of 10", "Finished 12 days ago".
pub fn countdown(trip: &Trip, today: jiff::civil::Date) -> Option<String> {
    let (s, e) = (trip.start?, trip.end?);
    let days =
        |a: jiff::civil::Date, b: jiff::civil::Date| a.until(b).map(|x| x.get_days()).unwrap_or(0);
    Some(if today < s {
        match days(today, s) {
            1 => "Tomorrow".into(),
            n => format!("In {n} days"),
        }
    } else if today <= e {
        format!("Day {} of {}", days(s, today) + 1, days(s, e) + 1)
    } else {
        match days(e, today) {
            1 => "Finished yesterday".into(),
            n => format!("Finished {n} days ago"),
        }
    })
}

pub fn ui(ui: &mut Ui, trip: &Trip, actions: &mut Vec<Action>) {
    let p = ui::pal(ui);
    let today = today();
    ui::h1(ui, &trip.name);
    let mut sub = Vec::new();
    if let (Some(s), Some(e)) = (trip.start, trip.end) {
        sub.push(format!(
            "{} to {}",
            s.strftime("%a %-d %b %Y"),
            e.strftime("%a %-d %b %Y")
        ));
        sub.push(format!(
            "{} nights",
            s.until(e).map(|x| x.get_days()).unwrap_or(0)
        ));
    }
    if !trip.travellers.is_empty() {
        sub.push(trip.travellers.join(", "));
    }
    ui.horizontal(|ui| {
        if let Some(c) = countdown(trip, today) {
            ui::text(ui, c, theme::display(22.0), p.trace);
        }
        if sub.is_empty() {
            if ui::link(ui, "Set the trip's dates").clicked() {
                actions.push(Action::Go(crate::app::Route::Trip));
            }
        } else {
            ui::muted(ui, sub.join(" · "));
        }
    });

    let t = trip.totals();
    let home = &trip.home_currency;
    let mut booked = Stat::new("Booked", money::format(t.committed, home), p.ink);
    if let Some(b) = trip.budget {
        let left = b - t.spend();
        booked = if left >= 0.0 {
            booked.detail(
                format!(
                    "{} of the {} budget left",
                    money::format(left, home),
                    money::format(b, home)
                ),
                p.ink2,
            )
        } else {
            booked.detail(
                format!(
                    "{} over the {} budget",
                    money::format(-left, home),
                    money::format(b, home)
                ),
                p.heat,
            )
        };
    }
    let mut ideas = Stat::new("Ideas, not booked", money::format(t.ideas, home), p.ink2);
    if t.unpriced > 0 {
        let n = t.unpriced;
        ideas = ideas.detail(
            format!(
                "{n} item{} with no exchange rate left out",
                if n == 1 { "" } else { "s" }
            ),
            p.warn,
        );
    }
    let stats = [
        booked,
        Stat::new("Paid", money::format(t.paid, home), p.ok),
        Stat::new(
            "Still to pay",
            money::format(t.owing(), home),
            if t.owing() > 0.0 { p.warn } else { p.ink },
        ),
        ideas,
    ];
    ui::stats(ui, &stats);

    let warnings = checks::check(trip, today);
    ui::split_with(
        ui,
        0.55,
        24.0,
        actions,
        |ui, actions| {
            ui::panel(ui, |ui| {
                ui::h2(ui, "To look at");
                if warnings.is_empty() {
                    ui::muted(
                        ui,
                        "Nothing. Every night has somewhere to stay and nothing is falling due.",
                    );
                }
                for w in &warnings {
                    let color = match w.severity {
                        Severity::Act => p.warn,
                        Severity::Check => p.ink2,
                    };
                    let r = ui.horizontal(|ui| {
                        ui::kind_mark(ui, color);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&w.text)
                                    .font(theme::body(15.0))
                                    .color(p.ink),
                            )
                            .wrap()
                            .sense(egui::Sense::click()),
                        )
                    });
                    let r = r.inner;
                    if (w.day.is_some() || !w.items.is_empty())
                        && r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                    {
                        if let Some(id) = w.items.first() {
                            actions.push(Action::Open(id.clone()));
                        }
                        if let Some(d) = w.day {
                            actions.push(Action::Day(d));
                        }
                    }
                }
            });
        },
        |ui, actions| {
            ui::panel(ui, |ui| {
                ui::h2(ui, "Coming up");
                let mut next: Vec<&waypoint_core::Item> = trip
                    .items
                    .iter()
                    .filter(|i| {
                        i.date().is_some_and(|d| d >= today)
                            && i.status != waypoint_core::Status::Cancelled
                    })
                    .collect();
                waypoint_core::model::sort_by_time(&mut next);
                if next.is_empty() {
                    ui::muted(ui, "Nothing dated from today on.");
                }
                for i in next.into_iter().take(6) {
                    let r = ui.horizontal(|ui| {
                        ui::kind_mark(ui, p.kind(i.kind));
                        let when = i.start.as_ref().map(|w| {
                            let mut s = w.date.strftime("%a %-d %b").to_string();
                            if let Some(t) = w.time {
                                s.push_str(&t.strftime(" %H:%M").to_string());
                            }
                            s
                        });
                        ui::small(ui, when.unwrap_or_default(), p.ink2);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(i.display_title())
                                    .font(theme::medium(15.0))
                                    .color(p.ink),
                            )
                            .truncate()
                            .sense(egui::Sense::click()),
                        )
                    });
                    if r.inner
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        actions.push(Action::Open(i.id.clone()));
                    }
                }
            });
        },
    );

    if trip.items.is_empty() {
        let picked = ui::empty_with(
            ui,
            "Nothing planned yet",
            "Add where you're staying, how you're getting there and what you'd like to do. Ideas \
             are fine: mark them booked, confirmed and paid as you go.",
            &[
                ("Add a stay", ui::Btn::Primary),
                ("Add travel", ui::Btn::Secondary),
                ("Add an activity", ui::Btn::Secondary),
            ],
        );
        let kind = match picked {
            Some(0) => Some(waypoint_core::Kind::Stay),
            Some(1) => Some(waypoint_core::Kind::Travel),
            Some(2) => Some(waypoint_core::Kind::Activity),
            _ => None,
        };
        if let Some(k) = kind {
            actions.push(Action::Add(k, trip.start));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn counts_down_and_through() {
        let mut t = Trip::new("T");
        t.start = Some(date(2027, 5, 15));
        t.end = Some(date(2027, 5, 25));
        assert_eq!(countdown(&t, date(2027, 5, 1)).unwrap(), "In 14 days");
        assert_eq!(countdown(&t, date(2027, 5, 14)).unwrap(), "Tomorrow");
        assert_eq!(countdown(&t, date(2027, 5, 17)).unwrap(), "Day 3 of 11");
        assert_eq!(
            countdown(&t, date(2027, 5, 26)).unwrap(),
            "Finished yesterday"
        );
    }
}
