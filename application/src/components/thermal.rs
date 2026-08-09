//! Onglet Zones thermiques : températures des zones du SoC.

use super::{ZONE_TEMP_CRIT, ZONE_TEMP_WARN};
use crate::core::{ReadLatest, View};
use crate::themes::palette;
use eframe::egui;

pub(crate) fn render_thermal(ui: &mut egui::Ui, view: &View) {
    ui.heading("Thermal Zones");

    let samples = view.by_kind("thermal");
    if samples.is_empty() {
        ui.label("No thermal data available.");
        return;
    }

    egui::Grid::new("thermal")
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            for s in &samples {
                if let Some(temp) = s.value.as_num() {
                    let color = palette().threshold(temp, ZONE_TEMP_WARN, ZONE_TEMP_CRIT);
                    ui.label(&s.label);
                    ui.colored_label(color, format!("{temp:.1}°C"));
                    ui.end_row();
                }
            }
        });
}
