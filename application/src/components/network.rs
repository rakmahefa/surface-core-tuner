//! Onglet Réseau : débits, totaux et température WiFi.
//!
//! Post-audit : cartes deux colonnes + graphiques de débit RX/TX (la donnée
//! est continue et déjà historisée dans le Store — autant la montrer).

use super::{card, metric_grid, section, time_chart};
use crate::core::{Store, View};
use crate::themes::{palette, theme};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub(crate) fn render_network(ui: &mut egui::Ui, view: &View, store: &Arc<Mutex<Store>>) {
    ui.label(
        egui::RichText::new("Network")
            .monospace()
            .size(20.0)
            .strong()
            .color(palette().value),
    );
    ui.add_space(4.0);

    // ── Ligne 1 : débits | totaux ──
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Throughput");
                metric_grid(ui, "net_rate", view, "network", &["RX Rate", "TX Rate"]);
            });
        });
        columns[1].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Total Transferred");
                metric_grid(
                    ui,
                    "net_total",
                    view,
                    "network",
                    &["RX Bytes", "TX Bytes", "WiFi Temp"],
                );
            });
        });
    });

    ui.add_space(10.0);

    // ── Graphiques RX / TX (pleine largeur) ──
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            card(ui, |ui| {
                time_chart(
                    ui,
                    store,
                    "network",
                    "RX Rate",
                    "Download Rate",
                    theme().series(0),
                    300.0,
                    None,
                );
            });
        });
        columns[1].vertical(|ui| {
            card(ui, |ui| {
                time_chart(
                    ui,
                    store,
                    "network",
                    "TX Rate",
                    "Upload Rate",
                    theme().series(1),
                    300.0,
                    None,
                );
            });
        });
    });
}
