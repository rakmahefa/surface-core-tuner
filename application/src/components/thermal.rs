//! Onglet Zones thermiques : températures des zones du SoC.
//!
//! Post-audit : état vide explicite et centré quand aucune sonde n'est
//! exposée (plus de page déserte), valeurs colorées par seuil en colonnes
//! stables.

use super::{card, section, ZONE_TEMP_CRIT, ZONE_TEMP_WARN};
use crate::core::{ReadLatest, View};
use crate::themes::palette;
use eframe::egui;

pub(crate) fn render_thermal(ui: &mut egui::Ui, view: &View) {
    ui.label(
        egui::RichText::new("Thermal")
            .monospace()
            .size(20.0)
            .strong()
            .color(palette().value),
    );
    ui.add_space(4.0);

    let samples = view.by_kind("thermal");
    if samples.is_empty() {
        super::empty_state(
            ui,
            "No thermal data",
            "No thermal zone is exposed in sysfs on this machine. \
             On a Surface, zones appear as soc_thermal, battery, WiFi, …",
        );
        return;
    }

    card(ui, |ui| {
        section(ui, "Thermal Zones");
        egui::Grid::new("thermal")
            .num_columns(2)
            .spacing([24.0, 5.0])
            .show(ui, |ui| {
                for s in &samples {
                    if let Some(temp) = s.value.as_num() {
                        let color = palette().threshold(temp, ZONE_TEMP_WARN, ZONE_TEMP_CRIT);
                        ui.label(&s.label);
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(format!("{temp:.1} °C"))
                                        .monospace()
                                        .strong()
                                        .color(color),
                                );
                            },
                        );
                        ui.end_row();
                    }
                }
            });
    });
}
