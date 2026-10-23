//! Waypoint: a holiday planner. Day-by-day itinerary, bookings, statuses and
//! costs, with each trip one file (best kept in a Google Drive folder) and a
//! phone itinerary written beside it.
//!
//! `waypoint` opens to the start screen, `waypoint TRIP.trip.json` opens a
//! trip, and `waypoint --sample` opens the made-up sample trip.

// No console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod doc;
mod settings;
mod theme;
mod ui;
mod views;

use eframe::egui;

use app::Open;
use settings::Settings;

fn main() -> eframe::Result {
    let mut open = None;
    for a in std::env::args().skip(1) {
        if a == "--sample" {
            open = Some(Open::Sample);
        } else if !a.starts_with("--") {
            open = Some(Open::Path(a.into()));
        }
    }
    let settings_path = Settings::path();
    let settings = Settings::load(settings_path.as_deref());

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Waypoint")
        .with_app_id("waypoint")
        .with_inner_size([1440.0, 920.0])
        .with_min_inner_size([960.0, 600.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/icon.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    eframe::run_native(
        "Waypoint",
        eframe::NativeOptions {
            viewport,
            persist_window: true,
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(app::WaypointApp::new(
                cc,
                settings,
                settings_path,
                open,
            )))
        }),
    )
}
