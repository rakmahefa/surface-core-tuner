//! Onglet CPU : charge par cœur, gouverneur, échelle de fréquences
//! (globale et par cœur), températures et pression mémoire.
//!
//! Réagencement post-audit : sections en cartes crème réparties sur deux
//! colonnes (fini la colonne unique et les 60 % de vide), barres de charge
//! avec valeur hors barre, grilles de métriques à colonnes stables.

use super::{
    card, freq_slider, governor_selector, level_bar, metric_grid, range_warning, section,
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
    ui.label(
        egui::RichText::new("CPU")
            .monospace()
            .size(20.0)
            .strong()
            .color(palette().value),
    );
    ui.add_space(4.0);

    // ── Ligne 1 : contrôles | charge par cœur ──
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Governor");
                ui.horizontal(|ui| {
                    ui.label("Scaling policy:");
                    if let Some(gov) = governor_selector(ui, view, "CPU0 Governor", governors) {
                        let _ = profiles::set_cpu_governor(&gov);
                    }
                });
                ui.add_space(6.0);
                section(ui, "Frequency Scaling");
                ui.label(
                    egui::RichText::new("Drag to adjust — applies at the end of the drag.")
                        .small()
                        .color(palette().muted),
                );
                if let Some(val) =
                    freq_slider(ui, view, "cpu", "CPU0 Min", "HW Min Freq", "HW Max Freq", "Min")
                {
                    let _ = profiles::set_cpu_min_mhz(val);
                }
                if let Some(val) =
                    freq_slider(ui, view, "cpu", "CPU0 Max", "HW Min Freq", "HW Max Freq", "Max")
                {
                    let _ = profiles::set_cpu_max_mhz(val);
                }
                if let (Some(min), Some(max)) = (view.num("cpu", "CPU0 Min"), view.num("cpu", "CPU0 Max")) {
                    range_warning(ui, min, max);
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    match (view.num("cpu", "HW Min Freq"), view.num("cpu", "HW Max Freq")) {
                        (Some(lo), Some(hi)) => {
                            ui.label(
                                egui::RichText::new(format!("HW range {lo:.0} – {hi:.0} MHz"))
                                    .small()
                                    .color(palette().muted),
                            );
                        }
                        _ => {
                            ui.label(
                                egui::RichText::new("HW range unavailable")
                                    .small()
                                    .color(palette().muted),
                            );
                        }
                    }
                });
            });
        });
        columns[1].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Per-Core Load");
                let mut any = false;
                for cpu in 0..CPU_THREADS {
                    if let Some(load) = view.num("cpu", &format!("CPU{cpu} Load")) {
                        any = true;
                        level_bar(
                            ui,
                            load,
                            format!("CPU{cpu}"),
                            LOAD_WARN,
                            LOAD_CRIT,
                            false,
                        );
                    }
                }
                if !any {
                    ui.label(
                        egui::RichText::new("no load readings on this machine")
                            .small()
                            .color(palette().muted),
                    );
                }
            });
        });
    });

    ui.add_space(10.0);

    // ── Graphique de charge (pleine largeur) ──
    card(ui, |ui| {
        time_chart(
            ui,
            store,
            "cpu",
            "CPU0 Load",
            "CPU0 Load",
            theme().series(0),
            300.0,
            Some(100.0),
        );
    });

    ui.add_space(10.0);

    // ── Ligne 2 : fréquences | températures + pression ──
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Per-Core Frequency");
                let freqs: Vec<Option<f64>> = (0..CPU_THREADS)
                    .map(|cpu| view.num("cpu", &format!("CPU{cpu} Freq")))
                    .collect();
                if freqs.iter().all(|f| f.is_none()) {
                    ui.label(
                        egui::RichText::new("no frequency readings on this machine")
                            .small()
                            .color(palette().muted),
                    );
                } else {
                    egui::Grid::new("cpu_freq")
                        .num_columns(4)
                        .spacing([24.0, 4.0])
                        .show(ui, |ui| {
                            for (cpu, f) in freqs.iter().enumerate() {
                                match f {
                                    Some(f) => {
                                        ui.label(
                                            egui::RichText::new(format!("CPU{cpu}: {f:.0} MHz"))
                                                .monospace()
                                                .color(palette().value),
                                        );
                                    }
                                    None => {
                                        ui.label(
                                            egui::RichText::new(format!("CPU{cpu}: —"))
                                                .monospace()
                                                .color(palette().muted),
                                        );
                                    }
                                }
                                if (cpu + 1) % 4 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                }
            });
        });
        columns[1].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Temperatures");
                let temps = ["Package Temp", "Core0 Temp", "Core1 Temp", "Core2 Temp", "Core3 Temp"];
                let mut any = false;
                egui::Grid::new("cpu_temp")
                    .num_columns(2)
                    .spacing([24.0, 4.0])
                    .show(ui, |ui| {
                        for t in &temps {
                            match view.num("cpu", t) {
                                Some(temp) => {
                                    any = true;
                                    let color = palette().threshold(temp, CPU_TEMP_WARN, CPU_TEMP_CRIT);
                                    ui.label(*t);
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
                                None => {
                                    ui.label(egui::RichText::new(*t).color(palette().value));
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.colored_label(palette().muted, "— no data");
                                        },
                                    );
                                    ui.end_row();
                                }
                            }
                        }
                    });
                if !any {
                    ui.label(
                        egui::RichText::new("no thermal readings on this machine")
                            .small()
                            .color(palette().muted),
                    );
                }
                ui.add_space(6.0);
                section(ui, "Pressure Stall (avg10)");
                metric_grid(ui, "cpu_psi", view, "cpu", &["PSI some avg10", "PSI full avg10"]);
            });
        });
    });

    ui.add_space(10.0);

    // ── Graphique de température (pleine largeur) ──
    card(ui, |ui| {
        time_chart(
            ui,
            store,
            "cpu",
            "Package Temp",
            "Package Temperature",
            theme().series(3),
            300.0,
            None,
        );
    });

    ui.add_space(10.0);

    // ── Contrôle par cœur (avancé, replié par défaut) ──
    egui::CollapsingHeader::new(
        egui::RichText::new("Per-Core Control (advanced)").strong().color(palette().value),
    )
    .default_open(false)
    .show(ui, |ui| {
        ui.label(
            egui::RichText::new("Set min/max individually per core (may be overridden by governor).")
                .small()
                .color(palette().muted),
        );
        ui.spacing_mut().slider_width = 110.0;
        egui::Grid::new("per_core")
            .num_columns(5)
            .spacing([14.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Core").strong().color(palette().value));
                ui.label(egui::RichText::new("Governor").strong().color(palette().value));
                ui.label(egui::RichText::new("Min").strong().color(palette().value));
                ui.label(egui::RichText::new("Max").strong().color(palette().value));
                ui.label(egui::RichText::new("Now").strong().color(palette().value));
                ui.end_row();
                for cpu in 0..CPU_THREADS {
                    ui.label(egui::RichText::new(format!("CPU{cpu}")).monospace());

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

                    // Fréquence courante (ou état indisponible)
                    let now = view
                        .num("cpu", &format!("CPU{cpu} Freq"))
                        .map(|f| format!("{f:.0} MHz"))
                        .unwrap_or_else(|| "—".to_string());
                    ui.label(egui::RichText::new(now).monospace().color(palette().muted));
                    ui.end_row();
                }
            });
    });
}
