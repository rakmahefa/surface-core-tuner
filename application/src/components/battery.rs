//! Onglet Batterie : capacité, puissance, énergie, santé et statut.

use super::{level_bar, metric, section, BATTERY_CRIT, BATTERY_WARN};
use crate::core::{ReadLatest, View};
use eframe::egui;

pub(crate) fn render_battery(ui: &mut egui::Ui, view: &View) {
    ui.heading("Battery");

    // Barre de capacité — bas = critique (seuils inversés)
    if let Some(cap) = view.num("battery", "Capacity") {
        level_bar(
            ui,
            cap,
            format!("🔋 {cap:.0}%"),
            BATTERY_WARN,
            BATTERY_CRIT,
            true,
        );
    }
    ui.separator();

    section(ui, "Power");
    egui::Grid::new("bat_power").num_columns(2).show(ui, |ui| {
        metric(ui, view, "battery", "Power");
        metric(ui, view, "battery", "Voltage");
    });
    ui.separator();

    section(ui, "Energy");
    egui::Grid::new("bat_energy").num_columns(2).show(ui, |ui| {
        metric(ui, view, "battery", "Energy Now");
        metric(ui, view, "battery", "Energy Full");
        metric(ui, view, "battery", "Energy Design");
    });
    ui.separator();

    section(ui, "Health");
    egui::Grid::new("bat_health").num_columns(2).show(ui, |ui| {
        metric(ui, view, "battery", "Wear");
        metric(ui, view, "battery", "Cycles");
    });
    ui.separator();

    section(ui, "Status");
    egui::Grid::new("bat_status").num_columns(2).show(ui, |ui| {
        metric(ui, view, "battery", "Status");
        metric(ui, view, "battery", "Manufacturer");
        metric(ui, view, "battery", "Technology");
        metric(ui, view, "battery", "AC Online");
    });
}
