//! Onglet CPU : charge par cœur, gouverneur, échelle de fréquences
//! (globale et par cœur), températures et pression mémoire.

use super::{
    freq_slider, governor_selector, level_bar, metric, range_warning, readonly_note, section,
    time_chart, CPU_TEMP_CRIT, CPU_TEMP_WARN, LOAD_CRIT, LOAD_WARN,
};
use crate::core::{ReadLatest, Store, View};
use crate::paths::CPU_THREADS;
use crate::profiles;
use crate::themes::{palette, theme};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub(crate) fn render_cpu(
    ui: &mut egui::Ui,
    view: &View,
    store: &Arc<Mutex<Store>>,
    governors: &Option<Vec<String>>,
) {
    ui.heading("CPU");

    // Governor (global — appliqué à tous les cœurs)
    ui.horizontal(|ui| {
        ui.label("Governor:");
        if let Some(gov) = governor_selector(ui, view, "CPU0 Governor", governors) {
            let _ = profiles::set_cpu_governor(&gov);
        }
    });
    ui.separator();

    // Per-core load
    section(ui, "Per-Core Load");
    for cpu in 0..CPU_THREADS {
        if let Some(load) = view.num("cpu", &format!("CPU{cpu} Load")) {
            level_bar(
                ui,
                load,
                format!("CPU{cpu}: {load:.1}%"),
                LOAD_WARN,
                LOAD_CRIT,
                false,
            );
        }
    }
    ui.separator();

    // Time chart
    time_chart(ui, store, "cpu", "CPU0 Load", theme().series(0), 300.0);
    ui.separator();

    // Per-core frequency
    section(ui, "Frequency");
    egui::Grid::new("cpu_freq")
        .num_columns(4)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            for cpu in 0..CPU_THREADS {
                let label = format!("CPU{cpu} Freq");
                match view.num("cpu", &label) {
                    Some(f) => {
                        ui.label(format!("CPU{cpu}: {f:.0} MHz"));
                    }
                    None => {
                        ui.colored_label(palette().muted, format!("CPU{cpu}: —"));
                    }
                }
                if (cpu + 1) % 4 == 0 {
                    ui.end_row();
                }
            }
        });
    ui.separator();

    // Frequency scaling controls
    section(ui, "Frequency Scaling");
    ui.label("Drag to adjust. Applies immediately.");

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            if let Some(val) = freq_slider(
                ui,
                view,
                "cpu",
                "CPU0 Min",
                "HW Min Freq",
                "HW Max Freq",
                "CPU0 Min",
            ) {
                let _ = profiles::set_cpu_min_mhz(val);
            }
        });
        ui.vertical(|ui| {
            if let Some(val) = freq_slider(
                ui,
                view,
                "cpu",
                "CPU0 Max",
                "HW Min Freq",
                "HW Max Freq",
                "CPU0 Max",
            ) {
                let _ = profiles::set_cpu_max_mhz(val);
            }
        });
    });
    if let (Some(min), Some(max)) = (view.num("cpu", "CPU0 Min"), view.num("cpu", "CPU0 Max")) {
        range_warning(ui, min, max);
    }

    // HW limits info
    ui.horizontal(|ui| {
        if let Some(hw_min) = view.num("cpu", "HW Min Freq") {
            ui.label(format!("HW limits: {hw_min:.0} MHz"));
        }
        if let Some(hw_max) = view.num("cpu", "HW Max Freq") {
            ui.label(format!("– {hw_max:.0} MHz"));
        }
        readonly_note(ui);
    });

    // Per-core frequency scaling (collapsible, grille alignée)
    egui::CollapsingHeader::new("Per-Core Control")
        .default_open(false)
        .show(ui, |ui| {
            ui.label("Set min/max individually per core (may be overridden by governor).");
            ui.spacing_mut().slider_width = 100.0;
            egui::Grid::new("per_core")
                .num_columns(5)
                .spacing([14.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Core");
                    ui.strong("Governor");
                    ui.strong("Min");
                    ui.strong("Max");
                    ui.strong("Now");
                    ui.end_row();
                    for cpu in 0..CPU_THREADS {
                        ui.label(format!("CPU{cpu}:"));

                        // Gouverneur individuel
                        if let Some(gov) =
                            governor_selector(ui, view, &format!("CPU{cpu} Governor"), governors)
                        {
                            let _ = profiles::set_cpu_governor_core(cpu, &gov);
                        }

                        if let Some(val) = freq_slider(
                            ui,
                            view,
                            "cpu",
                            &format!("CPU{cpu} Min"),
                            "HW Min Freq",
                            "HW Max Freq",
                            "",
                        ) {
                            let _ = profiles::set_cpu_min_mhz_core(cpu, val);
                        }

                        if let Some(val) = freq_slider(
                            ui,
                            view,
                            "cpu",
                            &format!("CPU{cpu} Max"),
                            "HW Min Freq",
                            "HW Max Freq",
                            "",
                        ) {
                            let _ = profiles::set_cpu_max_mhz_core(cpu, val);
                        }

                        // Fréquence courante (ou placeholder)
                        let now = view
                            .num("cpu", &format!("CPU{cpu} Freq"))
                            .map(|f| format!("{f:.0} MHz"))
                            .unwrap_or_else(|| "—".to_string());
                        ui.colored_label(palette().muted, now);
                        ui.end_row();
                    }
                });
        });
    ui.separator();

    // Temperatures
    section(ui, "Temperatures");
    egui::Grid::new("cpu_temp")
        .num_columns(4)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            let temps = [
                "Package Temp",
                "Core0 Temp",
                "Core1 Temp",
                "Core2 Temp",
                "Core3 Temp",
            ];
            for t in &temps {
                match view.num("cpu", t) {
                    Some(temp) => {
                        let color = palette().threshold(temp, CPU_TEMP_WARN, CPU_TEMP_CRIT);
                        ui.colored_label(color, format!("{t}: {temp:.1}°C"));
                    }
                    None => {
                        ui.colored_label(palette().muted, format!("{t}: —"));
                    }
                }
                ui.end_row();
            }
        });
    ui.separator();

    // Temperature chart
    time_chart(ui, store, "cpu", "Package Temp", theme().series(3), 300.0);
    ui.separator();

    // Pressure stall
    section(ui, "Pressure Stall (avg10)");
    egui::Grid::new("cpu_psi").num_columns(2).show(ui, |ui| {
        metric(ui, view, "cpu", "PSI some avg10");
        metric(ui, view, "cpu", "PSI full avg10");
    });
}
