//! Onglet I/O : débits et totaux des disques + pression d'attente.

use super::{metric, section};
use crate::core::{ReadLatest, Sample, View};
use crate::themes::palette;
use eframe::egui;

pub(crate) fn render_io(ui: &mut egui::Ui, view: &View) {
    ui.heading("I/O");

    let io_samples = view.by_kind("io");
    for dev in &["nvme0n1", "sda"] {
        section(ui, &format!("Disk: {dev}"));
        let dev_samples: Vec<&Sample> = io_samples
            .iter()
            .filter(|s| s.label.starts_with(dev))
            .copied()
            .collect();

        if dev_samples.is_empty() {
            ui.label("No data.");
            continue;
        }

        egui::Grid::new(format!("io_{dev}"))
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for s in &dev_samples {
                    ui.label(&s.label);
                    ui.colored_label(palette().value, s.display());
                    ui.end_row();
                }
            });
        ui.separator();
    }

    // Pressure stall
    section(ui, "Pressure Stall (avg10)");
    egui::Grid::new("io_psi").num_columns(2).show(ui, |ui| {
        metric(ui, view, "io", "PSI some avg10");
        metric(ui, view, "io", "PSI full avg10");
    });
}
