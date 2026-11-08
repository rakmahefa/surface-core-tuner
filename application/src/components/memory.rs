//! Onglet Mémoire : RAM, swap, pression mémoire et graphe d'utilisation.
//!
//! Réagencement post-audit : RAM et swap en cartes côte à côte, grilles de
//! métriques à colonnes stables (fini les chevauchements Total/Used), axe
//! du graphique borné sur la fenêtre réelle.

use super::{card, level_bar, metric_grid, section, time_chart, LOAD_CRIT, LOAD_WARN};
use crate::core::{ReadLatest, Store, View};
use crate::themes::{palette, theme};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub(crate) fn render_memory(ui: &mut egui::Ui, view: &View, store: &Arc<Mutex<Store>>) {
    ui.label(
        egui::RichText::new("Memory")
            .monospace()
            .size(20.0)
            .strong()
            .color(palette().value),
    );
    ui.add_space(4.0);

    // ── Barre RAM (pleine largeur) ──
    card(ui, |ui| {
        if let Some(used_pct) = view.num("memory", "Used%") {
            level_bar(ui, used_pct, "RAM used".into(), LOAD_WARN, LOAD_CRIT, false);
        } else {
            ui.label(
                egui::RichText::new("no memory readings on this machine")
                    .small()
                    .color(palette().muted),
            );
        }
    });

    ui.add_space(10.0);

    // ── Ligne : RAM | Swap ──
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "RAM");
                metric_grid(
                    ui,
                    "mem",
                    view,
                    "memory",
                    &["Total", "Used", "Available", "Free", "Buffers", "Cached"],
                );
            });
        });
        columns[1].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Swap");
                metric_grid(
                    ui,
                    "swap",
                    view,
                    "memory",
                    &["Swap Total", "Swap Used", "Swap Free", "Swap Cached"],
                );
            });
        });
    });

    ui.add_space(10.0);

    // ── Graphique d'utilisation (pleine largeur) ──
    card(ui, |ui| {
        time_chart(
            ui,
            store,
            "memory",
            "Used%",
            "RAM Used",
            theme().series(2),
            300.0,
            Some(100.0),
        );
    });

    ui.add_space(10.0);

    // ── Pression mémoire ──
    card(ui, |ui| {
        section(ui, "Pressure Stall (avg10)");
        metric_grid(ui, "mem_psi", view, "memory", &["PSI some avg10", "PSI full avg10"]);
    });
}
