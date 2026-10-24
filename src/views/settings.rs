//! Waypoint's own settings. Returns true when one changed.

use eframe::egui::Ui;

use crate::settings::{Settings, ThemeChoice};
use crate::theme;
use crate::ui;

pub fn ui(ui: &mut Ui, s: &mut Settings) -> bool {
    let p = ui::pal(ui);
    let mut changed = false;
    ui::h1(ui, "Settings");
    ui::panel(ui, |ui| {
        ui::h2(ui, "Appearance");
        let opts: Vec<(ThemeChoice, String)> = ThemeChoice::ALL
            .iter()
            .map(|t| (*t, t.label().to_string()))
            .collect();
        if let Some(i) = ui::chips(
            ui,
            &opts.iter().map(|o| o.1.as_str()).collect::<Vec<_>>(),
            opts.iter().position(|o| o.0 == s.theme),
        ) {
            s.theme = opts[i].0;
            changed = true;
        }
    });
    ui::panel(ui, |ui| {
        ui::h2(ui, "Phone itinerary");
        changed |= ui
            .checkbox(
                &mut s.phone_page,
                "Write a phone itinerary beside the trip each time it saves",
            )
            .changed();
        ui::para(
            ui,
            "It's a single page named after the trip (\"Japan 2027.html\"), with no scripts and \
             nothing fetched from the internet. On an iPhone, open it from the Files app under \
             Google Drive; mark it \"Available offline\" in Drive before you fly.",
            theme::body(14.0),
            p.ink2,
            620.0,
        );
    });
    ui::panel(ui, |ui| {
        ui::h2(ui, "Exchange rates");
        changed |= ui
            .checkbox(
                &mut s.rates_on_open,
                "Update exchange rates when a trip opens",
            )
            .changed();
        ui::para(
            ui,
            "Rates come from Frankfurter (the European Central Bank's daily rates) and, for \
             currencies it doesn't cover, ExchangeRate-API's free rates. Neither needs an \
             account. Waypoint only goes online for rates when this is on or you choose \
             Update rates on the Costs screen.",
            theme::body(14.0),
            p.ink2,
            620.0,
        );
    });
    ui::panel(ui, |ui| {
        ui::h2(ui, "About");
        ui::muted(ui, format!("Waypoint {}", env!("CARGO_PKG_VERSION")));
        ui::muted(
            ui,
            "Airport and airline lists from OpenFlights (Open Database License); towns and their time zones from Natural Earth.",
        );
        if let Some(path) = Settings::path() {
            ui::muted(ui, format!("Settings are kept in {}", path.display()));
        }
    });
    changed
}
