//! Every item in one table, filtered by kind and status or searched: the
//! part that replaces the spreadsheet.

use eframe::egui::Ui;
use waypoint_core::{money, Item, Kind, Status, Trip};

use crate::app::Action;
use crate::ui::{self, cell, col, Cell};
use crate::views::itinerary::add_menu;

#[derive(Default)]
pub struct Bookings {
    kind: Option<Kind>,
    status: Option<Status>,
    search: String,
}

fn matches(item: &Item, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let q = q.to_lowercase();
    let mut hay = vec![
        item.display_title(),
        item.place.clone(),
        item.address.clone(),
        item.provider.clone(),
        item.reference.clone(),
        item.contact.clone(),
        item.notes.clone(),
    ];
    if let Some(t) = &item.travel {
        hay.extend([
            t.carrier.clone(),
            t.number.clone(),
            t.from.clone(),
            t.to.clone(),
        ]);
    }
    hay.iter().any(|h| h.to_lowercase().contains(&q))
}

impl Bookings {
    pub fn ui(&mut self, ui: &mut Ui, trip: &Trip, open: Option<&str>, actions: &mut Vec<Action>) {
        let p = ui::pal(ui);
        ui.horizontal(|ui| {
            ui::h1(ui, "Bookings");
            ui.with_layout(
                eframe::egui::Layout::right_to_left(eframe::egui::Align::Center),
                |ui| {
                    add_menu(ui, "+ Add", None, actions);
                },
            );
        });
        ui.horizontal_wrapped(|ui| {
            let kinds: Vec<&str> = std::iter::once("All kinds")
                .chain(Kind::ALL.iter().map(|k| k.label()))
                .collect();
            let sel = self
                .kind
                .map(|k| Kind::ALL.iter().position(|x| *x == k).unwrap() + 1)
                .unwrap_or(0);
            if let Some(i) = ui::chips(ui, &kinds, Some(sel)) {
                self.kind = (i > 0).then(|| Kind::ALL[i - 1]);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let statuses: Vec<&str> = std::iter::once("Any status")
                .chain(Status::ALL.iter().map(|s| s.label()))
                .collect();
            let sel = self
                .status
                .map(|s| Status::ALL.iter().position(|x| *x == s).unwrap() + 1)
                .unwrap_or(0);
            if let Some(i) = ui::chips(ui, &statuses, Some(sel)) {
                self.status = (i > 0).then(|| Status::ALL[i - 1]);
            }
            ui.add_space(12.0);
            ui::text_input(
                ui,
                &mut self.search,
                "Search names, places, references…",
                280.0,
            );
        });

        let mut items: Vec<&Item> = trip
            .items
            .iter()
            .filter(|i| self.kind.is_none_or(|k| i.kind == k))
            .filter(|i| self.status.is_none_or(|s| i.status == s))
            .filter(|i| matches(i, self.search.trim()))
            .collect();
        // Dated items in order, then the undated.
        items.sort_by_key(|i| {
            (
                i.start.is_none(),
                i.start.as_ref().map(|w| (w.date, w.time)),
            )
        });

        let home = &trip.home_currency;
        let cols = [
            col("Date", 110.0, false),
            col("Time", 60.0, false),
            col("Kind", 80.0, false),
            col("What", 0.0, false),
            col("Status", 92.0, false),
            col("Reference", 140.0, false),
            col("Cost", 110.0, true),
            col(format!("In {home}"), 110.0, true),
        ];
        let rows: Vec<Vec<Cell>> = items
            .iter()
            .map(|i| {
                let code = trip.currency_of(i);
                let date = i
                    .start
                    .as_ref()
                    .map(|w| w.date.strftime("%a %-d %b").to_string())
                    .unwrap_or_else(|| "No date".into());
                let time = i
                    .start
                    .as_ref()
                    .and_then(|w| w.time)
                    .map(|t| t.strftime("%H:%M").to_string())
                    .unwrap_or_default();
                let (cost, home_cost) = if i.cost.is_empty() {
                    (String::new(), String::new())
                } else {
                    (
                        money::format(i.cost.amount, &code),
                        trip.home_amount(i)
                            .map(|v| money::format(v, home))
                            .unwrap_or_else(|| "no rate".into()),
                    )
                };
                let mut title = cell(i.display_title()).strong();
                if i.status == Status::Cancelled {
                    title = title.color(p.ink2);
                }
                vec![
                    cell(date).color(if i.start.is_none() { p.ink2 } else { p.ink }),
                    cell(time),
                    cell(i.kind.label()).color(p.kind(i.kind)),
                    title,
                    cell(i.status.label()).color(p.status(i.status)),
                    cell(i.reference.clone()),
                    cell(cost),
                    cell(home_cost)
                        .color(if i.cost.owing() > 0.0 && i.status.committed() {
                            p.warn
                        } else {
                            p.ink
                        })
                        .tooltip(if i.cost.owing() > 0.0 && i.status.committed() {
                            format!("{} still to pay", money::format(i.cost.owing(), &code))
                        } else {
                            String::new()
                        }),
                ]
            })
            .collect();
        ui::panel(ui, |ui| {
            if rows.is_empty() {
                ui::muted(
                    ui,
                    if trip.items.is_empty() {
                        "Nothing added yet."
                    } else {
                        "Nothing matches."
                    },
                );
                return;
            }
            let selected = open.and_then(|id| items.iter().position(|i| i.id == id));
            if let Some(k) = ui::table(ui, &cols, &rows, 14.0, selected) {
                actions.push(Action::Open(items[k].id.clone()));
            }
            ui::small(
                ui,
                format!("{} of {} items", items.len(), trip.items.len()),
                p.ink2,
            );
        });
    }
}
