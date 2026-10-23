//! The money: totals against the budget, by kind, by currency with the
//! exchange rates, and what's still to pay.

use std::collections::HashMap;

use eframe::egui::{self, Ui};
use waypoint_core::{money, Rate, Trip};

use crate::app::{today, Action};
use crate::theme;
use crate::ui::{self, cell, col, Btn, Stat};

#[derive(Default)]
pub struct Costs {
    /// Rates being typed, by currency.
    typing: HashMap<String, String>,
}

impl Costs {
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        trip: &mut Trip,
        fetching: bool,
        open: Option<&str>,
        actions: &mut Vec<Action>,
    ) {
        let p = ui::pal(ui);
        let home = trip.home_currency.clone();
        ui::h1(ui, "Costs");
        let t = trip.totals();
        let mut spent = Stat::new("Booked", money::format(t.committed, &home), p.ink);
        if t.lost > 0.0 {
            spent = spent.detail(
                format!(
                    "plus {} lost to cancellations",
                    money::format(t.lost, &home)
                ),
                p.heat,
            );
        }
        ui::stats(
            ui,
            &[
                spent,
                Stat::new("Paid", money::format(t.paid, &home), p.ok),
                Stat::new(
                    "Still to pay",
                    money::format(t.owing(), &home),
                    if t.owing() > 0.0 { p.warn } else { p.ink },
                ),
                Stat::new("Ideas, not booked", money::format(t.ideas, &home), p.ink2),
            ],
        );

        if let Some(b) = trip.budget {
            ui::panel(ui, |ui| {
                ui::h2(ui, "Budget");
                let spend = t.spend();
                let w = ui.available_width();
                ui::meter(
                    ui,
                    w,
                    (spend / b.max(1.0)) as f32,
                    if spend > b { p.heat } else { p.trace },
                );
                let with_ideas = spend + t.ideas;
                ui::muted(
                    ui,
                    format!(
                        "{} of {} booked ({:.0}%). With every idea booked too: {}.",
                        money::format(spend, &home),
                        money::format(b, &home),
                        spend / b.max(1.0) * 100.0,
                        money::format(with_ideas, &home)
                    ),
                );
            });
        }

        ui::split(
            ui,
            0.45,
            24.0,
            |ui| {
                ui::panel(ui, |ui| {
                    ui::h2(ui, "By kind");
                    let rows: Vec<Vec<ui::Cell>> = t
                        .by_kind
                        .iter()
                        .map(|(k, v)| {
                            vec![
                                cell(k.label()).color(p.kind(*k)),
                                cell(money::format(*v, &home)),
                                cell(if t.committed > 0.0 {
                                    format!("{:.0}%", v / t.committed * 100.0)
                                } else {
                                    String::new()
                                }),
                            ]
                        })
                        .collect();
                    if rows.is_empty() {
                        ui::muted(ui, "Nothing booked yet.");
                    } else {
                        ui::table(
                            ui,
                            &[
                                col("Kind", 0.0, false),
                                col("Booked", 120.0, true),
                                col("Share", 70.0, true),
                            ],
                            &rows,
                            14.0,
                            None,
                        );
                    }
                });
            },
            |ui| {
                ui::panel(ui, |ui| {
                    ui::h2(ui, "Still to pay");
                    let mut owing: Vec<&waypoint_core::Item> = trip
                        .items
                        .iter()
                        .filter(|i| i.status.committed() && i.cost.owing() > 0.0)
                        .collect();
                    owing.sort_by_key(|i| (i.cost.due.is_none(), i.cost.due, i.date()));
                    if owing.is_empty() {
                        ui::muted(ui, "Nothing owing on what's booked.");
                        return;
                    }
                    let today = today();
                    let rows: Vec<Vec<ui::Cell>> = owing
                        .iter()
                        .map(|i| {
                            let code = trip.currency_of(i);
                            let due = i
                                .cost
                                .due
                                .map(|d| d.strftime("%a %-d %b").to_string())
                                .unwrap_or_else(|| "No date".into());
                            let late = i.cost.due.is_some_and(|d| d <= today);
                            vec![
                                cell(i.display_title()),
                                cell(due).color(if late { p.heat } else { p.ink }),
                                cell(money::format(i.cost.owing(), &code)),
                            ]
                        })
                        .collect();
                    let sel = open.and_then(|id| owing.iter().position(|i| i.id == id));
                    if let Some(k) = ui::table(
                        ui,
                        &[
                            col("What", 0.0, false),
                            col("Due", 110.0, false),
                            col("Owing", 110.0, true),
                        ],
                        &rows,
                        14.0,
                        sel,
                    ) {
                        actions.push(Action::Open(owing[k].id.clone()));
                    }
                });
            },
        );

        ui::panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui::h2(ui, "Exchange rates");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if fetching {
                        "Updating…"
                    } else {
                        "Update rates"
                    };
                    if ui::button(ui, label, Btn::Secondary)
                        .on_hover_text("Get today's rates from Frankfurter and ExchangeRate-API")
                        .clicked()
                        && !fetching
                    {
                        actions.push(Action::FetchRates);
                    }
                });
            });
            let codes = trip.foreign_currencies();
            if codes.is_empty() {
                ui::muted(ui, format!("Everything is in {home} so far. Give an item a cost in another currency and its rate appears here."));
                return;
            }
            ui::para(
                ui,
                format!(
                    "Costs not yet paid use these rates, so their {home} amounts are estimates. \
                     Marking an item paid keeps the rate on that day."
                ),
                theme::body(14.0),
                p.ink2,
                640.0,
            );
            egui::Grid::new("rates")
                .num_columns(5)
                .spacing([18.0, 10.0])
                .show(ui, |ui| {
                    for h in [
                        "Currency",
                        "Total",
                        &format!("{home} for 1"),
                        "As of",
                        "From",
                    ] {
                        ui::small(ui, h, p.ink2);
                    }
                    ui.end_row();
                    for code in codes {
                        let total: f64 = trip
                            .items
                            .iter()
                            .filter(|i| {
                                trip.currency_of(i) == code
                                    && i.status != waypoint_core::Status::Cancelled
                            })
                            .map(|i| i.cost.amount)
                            .sum();
                        let name = money::currency(&code).map(|c| c.name).unwrap_or("");
                        ui.horizontal(|ui| {
                            ui::text(ui, &code, theme::semi(15.0), p.ink);
                            ui::small(ui, name, p.ink2);
                        });
                        ui::text(ui, money::format(total, &code), theme::body(15.0), p.ink);
                        let current = trip
                            .rates
                            .get(&code)
                            .map(|r| r.home_per_unit)
                            .filter(|v| *v > 0.0);
                        let buf = self
                            .typing
                            .entry(code.clone())
                            .or_insert_with(|| current.map(fmt_rate).unwrap_or_default());
                        let r = ui.add_sized(
                            [120.0, 34.0],
                            egui::TextEdit::singleline(buf)
                                .hint_text("type a rate")
                                .font(theme::medium(14.0))
                                .margin(egui::Margin::symmetric(10, 8)),
                        );
                        if r.changed() {
                            if let Some(v) = money::parse_amount(buf).filter(|v| *v > 0.0) {
                                trip.rates.insert(
                                    code.clone(),
                                    Rate {
                                        home_per_unit: v,
                                        as_of: Some(today()),
                                        source: "Entered by hand".into(),
                                    },
                                );
                            }
                        } else if !r.has_focus() {
                            // Keep the box in step with fetched rates.
                            if let Some(c) = current {
                                if money::parse_amount(buf) != Some(c) {
                                    *buf = fmt_rate(c);
                                }
                            }
                        }
                        let rate = trip.rates.get(&code);
                        ui::small(
                            ui,
                            rate.and_then(|r| r.as_of)
                                .map(|d| d.strftime("%-d %b %Y").to_string())
                                .unwrap_or_default(),
                            p.ink2,
                        );
                        ui::small(
                            ui,
                            rate.map(|r| r.source.clone())
                                .unwrap_or_else(|| "No rate yet".into()),
                            if rate.is_some() { p.ink2 } else { p.warn },
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

/// A rate with enough figures to matter: 0.0102, 1.0834, 16650.
fn fmt_rate(v: f64) -> String {
    let s = if v >= 100.0 {
        format!("{v:.2}")
    } else if v >= 1.0 {
        format!("{v:.4}")
    } else {
        format!("{v:.6}")
    };
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
