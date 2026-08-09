//! Onglet Réseau : débits, totaux et température WiFi.

use super::{metric, section};
use crate::core::View;
use eframe::egui;

pub(crate) fn render_network(ui: &mut egui::Ui, view: &View) {
    ui.heading("Network");

    section(ui, "Throughput");
    egui::Grid::new("net_rate").num_columns(2).show(ui, |ui| {
        metric(ui, view, "network", "RX Rate");
        metric(ui, view, "network", "TX Rate");
    });
    ui.separator();

    section(ui, "Total");
    egui::Grid::new("net_total").num_columns(2).show(ui, |ui| {
        metric(ui, view, "network", "RX Bytes");
        metric(ui, view, "network", "TX Bytes");
    });
    ui.separator();

    section(ui, "Temperature");
    metric(ui, view, "network", "WiFi Temp");
}
