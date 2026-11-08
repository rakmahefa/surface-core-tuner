//! Onglet GPU : fréquences courantes, sliders RP1/RP0, RPS et utilisation.
//!
//! Réagencement post-audit : cartes deux colonnes, sigles RP0/RP1 explicités,
//! curseurs rendus seulement si les limites matérielles sont connues.

use super::{
    card, freq_slider, level_bar, metric_grid, range_warning, section, time_chart, LOAD_CRIT,
    LOAD_WARN,
};
use crate::core::{ReadLatest, Store, View};
use crate::paths::gpu_dir;
use crate::profiles;
use crate::themes::{palette, theme};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub(crate) fn render_gpu(ui: &mut egui::Ui, view: &View, store: &Arc<Mutex<Store>>) {
    ui.label(
        egui::RichText::new("GPU")
            .monospace()
            .size(20.0)
            .strong()
            .color(palette().value),
    );
    ui.add_space(4.0);

    // ── Ligne 1 : état courant | utilisation ──
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Current Frequency");
                metric_grid(ui, "gpu_cur", view, "gpu", &["Cur Freq", "Act Freq"]);
                ui.add_space(6.0);
                section(ui, "RPS (Render Power Sparing)");
                metric_grid(
                    ui,
                    "gpu_rps",
                    view,
                    "gpu",
                    &["RPS Cur Freq", "RPS Min Freq", "RPS Max Freq", "RPS Boost Freq"],
                );
            });
        });
        columns[1].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Utilisation");
                if let Some(busy) = view.num("gpu", "Busy") {
                    level_bar(ui, busy, "GPU Busy".into(), LOAD_WARN, LOAD_CRIT, false);
                } else {
                    ui.label(
                        egui::RichText::new("no utilisation readings on this machine")
                            .small()
                            .color(palette().muted),
                    );
                }
                ui.add_space(6.0);
                egui::Grid::new("gpu_rc6")
                    .num_columns(2)
                    .spacing([24.0, 5.0])
                    .show(ui, |ui| {
                        super::metric_row(ui, "RC6 Residency (idle %)", {
                            view.get("gpu", "RC6 Residency").map(|s| s.display())
                        });
                    });
            });
        });
    });

    ui.add_space(10.0);

    // ── Contrôle de fréquence (pleine largeur si le GPU est pilotable) ──
    if gpu_dir().is_some() {
        card(ui, |ui| {
            section(ui, "Frequency Scaling");
            ui.label(
                egui::RichText::new(
                    "RP1 = minimum sustained clock, RP0 = maximum sustained clock. \
                     Drag to adjust — applies at the end of the drag.",
                )
                .small()
                .color(palette().muted),
            );
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    ui.label("RP1 (min):");
                    if let Some(val) =
                        freq_slider(ui, view, "gpu", "Min Freq", "RP1 Min", "RP0 Max", "min")
                    {
                        let _ = profiles::set_gpu_min_mhz(val);
                    }
                });
                columns[1].vertical(|ui| {
                    ui.label("RP0 (max):");
                    if let Some(val) =
                        freq_slider(ui, view, "gpu", "Max Freq", "RP1 Min", "RP0 Max", "max")
                    {
                        let _ = profiles::set_gpu_max_mhz(val);
                    }
                });
            });
            if let (Some(min), Some(max)) = (view.num("gpu", "Min Freq"), view.num("gpu", "Max Freq")) {
                range_warning(ui, min, max);
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if let Some(rp1) = view.num("gpu", "RP1 Min") {
                    ui.label(
                        egui::RichText::new(format!("HW range RP1 {rp1:.0} MHz"))
                            .small()
                            .color(palette().muted),
                    );
                }
                if let Some(rp0) = view.num("gpu", "RP0 Max") {
                    ui.label(
                        egui::RichText::new(format!("– RP0 {rp0:.0} MHz"))
                            .small()
                            .color(palette().muted),
                    );
                }
                if let Some(boost) = view.num("gpu", "Boost Freq") {
                    ui.label(
                        egui::RichText::new(format!("· boost {boost:.0} MHz"))
                            .small()
                            .color(palette().muted),
                    );
                }
            });
        });
        ui.add_space(10.0);
    }

    // ── Graphique d'utilisation (pleine largeur) ──
    card(ui, |ui| {
        time_chart(
            ui,
            store,
            "gpu",
            "Busy",
            "GPU Busy",
            theme().series(1),
            300.0,
            Some(100.0),
        );
    });
}
