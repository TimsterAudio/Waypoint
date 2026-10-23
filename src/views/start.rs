//! With no trip open: start a new one, open one, or look at the sample.

use eframe::egui::{self, Ui};

use crate::app::Action;
use crate::settings::Settings;
use crate::theme;
use crate::ui::{self, Btn};

pub fn ui(ui: &mut Ui, settings: &Settings, message: Option<&str>, actions: &mut Vec<Action>) {
    let p = ui::pal(ui);
    ui.add_space(24.0);
    ui::h1(ui, "Plan a trip");
    ui::para(
        ui,
        "Each trip is one file. Keep it in your Google Drive folder and it's backed up, and on \
         your phone too: Waypoint writes a phone itinerary beside it every time it saves.",
        theme::body(16.0),
        p.ink2,
        620.0,
    );
    if let Some(m) = message {
        ui::notice(ui, m);
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        if ui::button(ui, "New trip…", Btn::Primary).clicked() {
            actions.push(Action::NewTrip);
        }
        if ui::button(ui, "Open a trip…", Btn::Secondary).clicked() {
            actions.push(Action::OpenTrip);
        }
        if ui::button(ui, "Look at the sample trip", Btn::Secondary).clicked() {
            actions.push(Action::Sample);
        }
    });
    if !settings.recent.is_empty() {
        ui.add_space(8.0);
        ui::panel(ui, |ui| {
            ui::h2(ui, "Recent trips");
            for path in &settings.recent {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.strip_suffix(waypoint_core::file::EXTENSION).unwrap_or(n))
                    .unwrap_or("Trip");
                ui.horizontal(|ui| {
                    if ui::link(ui, name).clicked() {
                        actions.push(Action::OpenPath(path.clone()));
                    }
                    let folder = path
                        .parent()
                        .map(|d| d.display().to_string())
                        .unwrap_or_default();
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(folder)
                                .font(theme::body(13.0))
                                .color(p.ink2),
                        )
                        .truncate(),
                    );
                });
            }
        });
    }
}
