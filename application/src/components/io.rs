//! Onglet I/O : débits et totaux des disques + pression d'attente.
//!
//! Post-audit : un état vide explicite par disque (plus de « No data. »
//! ambigu), grilles à colonnes stables (plus de chevauchement PSI), valeurs
//! en encre — le rouge reste réservé aux seuils critiques.

use super::{card, metric_grid, section, time_chart};
use crate::core::{ReadLatest, Sample, Store, View};
use crate::themes::{palette, theme};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub(crate) fn render_io(ui: &mut egui::Ui, view: &View, store: &Arc<Mutex<Store>>) {
    ui.label(
        egui::RichText::new("I/O")
            .monospace()
            .size(20.0)
            .strong()
            .color(palette().value),
    );
    ui.add_space(4.0);

    let io_samples = view.by_kind("io");

    // ── Une carte par disque, côte à côte ──
    let mut cards: Vec<Vec<&Sample>> = Vec::new();
    for dev in &["nvme0n1", "sda"] {
        let dev_samples: Vec<&crate::core::Sample> = io_samples
            .iter()
            .filter(|s| s.label.starts_with(dev))
            .copied()
            .collect();
        cards.push(dev_samples);
    }

    ui.columns(2, |columns| {
        for (i, dev_samples) in cards.iter().enumerate() {
            let dev = if i == 0 { "nvme0n1" } else { "sda" };
            columns[i].vertical(|ui| {
                card(ui, |ui| {
                    section(ui, &format!("Disk {dev}"));
                    if dev_samples.is_empty() {
                        ui.label(
                            egui::RichText::new("device not present on this machine")
                                .small()
                                .color(palette().muted),
                        );
                    } else {
                        egui::Grid::new(format!("io_{dev}"))
                            .num_columns(2)
                            .spacing([24.0, 5.0])
                            .show(ui, |ui| {
                                for s in dev_samples {
                                    ui.label(&s.label);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(s.display())
                                                    .monospace()
                                                    .strong()
                                                    .color(palette().value),
                                            );
                                        },
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
            });
        }
    });

    ui.add_space(10.0);

    // ── Pression d'attente + débits du disque principal ──
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Pressure Stall (avg10)");
                metric_grid(ui, "io_psi", view, "io", &["PSI some avg10", "PSI full avg10"]);
            });
        });
        columns[1].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Primary Disk Throughput");
                time_chart(
                    ui,
                    store,
                    "io",
                    "nvme0n1 Read B/s",
                    "nvme0n1 read",
                    theme().series(0),
                    300.0,
                    None,
                );
            });
        });
    });
}
