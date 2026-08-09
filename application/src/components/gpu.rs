//! Onglet GPU : fréquences courantes, sliders RP1/RP0, RPS et utilisation.

use super::{
    freq_slider, level_bar, metric, metric_mhz, range_warning, readonly_note, section, time_chart,
    LOAD_CRIT, LOAD_WARN,
};
use crate::core::{ReadLatest, Store, View};
use crate::paths::gpu_dir;
use crate::profiles;
use crate::themes::theme;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub(crate) fn render_gpu(ui: &mut egui::Ui, view: &View, store: &Arc<Mutex<Store>>) {
    ui.heading("GPU");

    // Fréquences courantes (lecture seule)
    section(ui, "Current");
    ui.horizontal(|ui| {
        metric_mhz(ui, view, "gpu", "Cur Freq");
        metric_mhz(ui, view, "gpu", "Act Freq");
    });
    ui.separator();

    // Interactive sliders (écritures sysfs — nécessite le chemin DRM du GPU)
    if gpu_dir().is_some() {
        section(ui, "Frequency Scaling");
        ui.label("Drag to adjust. Applies immediately.");
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("RP1 (Min):");
                if let Some(val) = freq_slider(
                    ui, view, "gpu", "Min Freq", "RP1 Min", "RP0 Max", "Min Freq",
                ) {
                    let _ = profiles::set_gpu_min_mhz(val);
                }
            });
            ui.vertical(|ui| {
                ui.label("RP0 (Max):");
                if let Some(val) = freq_slider(
                    ui, view, "gpu", "Max Freq", "RP1 Min", "RP0 Max", "Max Freq",
                ) {
                    let _ = profiles::set_gpu_max_mhz(val);
                }
            });
        });
        if let (Some(min), Some(max)) = (view.num("gpu", "Min Freq"), view.num("gpu", "Max Freq")) {
            range_warning(ui, min, max);
        }
        // RP0/RP1 read-only reference
        ui.horizontal(|ui| {
            if let Some(rp1) = view.num("gpu", "RP1 Min") {
                ui.label(format!("HW range: RP1 {rp1:.0} MHz"));
            }
            if let Some(rp0) = view.num("gpu", "RP0 Max") {
                ui.label(format!("– RP0 {rp0:.0} MHz"));
            }
            readonly_note(ui);
        });
        if let Some(boost) = view.num("gpu", "Boost Freq") {
            ui.horizontal(|ui| {
                ui.label(format!("Boost: {boost:.0} MHz"));
                readonly_note(ui);
            });
        }
    }
    ui.separator();

    section(ui, "RPS");
    egui::Grid::new("gpu_rps").num_columns(2).show(ui, |ui| {
        metric_mhz(ui, view, "gpu", "RPS Cur Freq");
        metric_mhz(ui, view, "gpu", "RPS Min Freq");
        metric_mhz(ui, view, "gpu", "RPS Max Freq");
        metric_mhz(ui, view, "gpu", "RPS Boost Freq");
    });
    ui.separator();

    // Utilisation : même code couleur que les charges CPU
    if let Some(busy) = view.num("gpu", "Busy") {
        section(ui, "Utilisation");
        level_bar(
            ui,
            busy,
            format!("GPU Busy: {busy:.1}%"),
            LOAD_WARN,
            LOAD_CRIT,
            false,
        );
    }
    metric(ui, view, "gpu", "RC6 Residency");

    ui.separator();
    time_chart(ui, store, "gpu", "Busy", theme().series(1), 300.0);
}
