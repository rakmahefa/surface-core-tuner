//! Onglet Mémoire : RAM, swap, pression mémoire et graphe d'utilisation.

use super::{level_bar, metric, section, time_chart, LOAD_CRIT, LOAD_WARN};
use crate::core::{ReadLatest, Store, View};
use crate::themes::theme;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub(crate) fn render_memory(ui: &mut egui::Ui, view: &View, store: &Arc<Mutex<Store>>) {
    ui.heading("Memory");

    if let Some(used_pct) = view.num("memory", "Used%") {
        level_bar(
            ui,
            used_pct,
            format!("RAM Used: {used_pct:.1}%"),
            LOAD_WARN,
            LOAD_CRIT,
            false,
        );
    }
    ui.separator();

    section(ui, "RAM");
    egui::Grid::new("mem").num_columns(2).show(ui, |ui| {
        metric(ui, view, "memory", "Total");
        metric(ui, view, "memory", "Used");
        metric(ui, view, "memory", "Available");
        metric(ui, view, "memory", "Free");
        metric(ui, view, "memory", "Buffers");
        metric(ui, view, "memory", "Cached");
    });
    ui.separator();

    time_chart(ui, store, "memory", "Used%", theme().series(2), 300.0);
    ui.separator();

    section(ui, "Swap");
    egui::Grid::new("swap").num_columns(2).show(ui, |ui| {
        metric(ui, view, "memory", "Swap Total");
        metric(ui, view, "memory", "Swap Used");
        metric(ui, view, "memory", "Swap Free");
        metric(ui, view, "memory", "Swap Cached");
    });
    ui.separator();

    section(ui, "Pressure Stall (avg10)");
    egui::Grid::new("mem_psi").num_columns(2).show(ui, |ui| {
        metric(ui, view, "memory", "PSI some avg10");
        metric(ui, view, "memory", "PSI full avg10");
    });
}
