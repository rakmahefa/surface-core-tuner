//! Onglet Batterie : capacité, puissance, énergie, santé et statut.
//!
//! Réagencement post-audit : quatre groupes en cartes 2 × 2 (fini la pile
//! mono-colonne à 70 % de vide), état vide explicite quand aucune batterie
//! n'est détectée, plus aucun libellé collé au tiret.

use super::{badge, card, level_bar, metric_grid, section, BATTERY_CRIT, BATTERY_WARN};
use crate::core::{ReadLatest, View};
use crate::themes::palette;
use eframe::egui;

pub(crate) fn render_battery(ui: &mut egui::Ui, view: &View) {
    ui.label(
        egui::RichText::new("Battery")
            .monospace()
            .size(20.0)
            .strong()
            .color(palette().value),
    );
    ui.add_space(4.0);

    let has_battery = view
        .by_kind("battery")
        .iter()
        .any(|s| s.value.as_num().is_some());

    // ── Barre de capacité (pleine largeur) ou état vide ──
    if !has_battery {
        super::empty_state(
            ui,
            "No battery detected",
            "This machine exposes no battery in sysfs (virtual machine, \
             desktop, or removed battery). Plug detection may still work.",
        );
        ui.add_space(10.0);
    } else if let Some(cap) = view.num("battery", "Capacity") {
        card(ui, |ui| {
            level_bar(ui, cap, "Charge".into(), BATTERY_WARN, BATTERY_CRIT, true);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if let Some(ac) = view.num("battery", "AC Online") {
                    if ac > 0.0 {
                        badge(ui, "AC POWER", palette().ok_soft);
                    } else {
                        badge(ui, "DISCHARGING", palette().warn);
                    }
                }
            });
        });
        ui.add_space(10.0);
    }

    // ── Quatre groupes en 2 × 2 ──
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Power");
                metric_grid(ui, "bat_power", view, "battery", &["Power", "Voltage"]);
            });
            ui.add_space(10.0);
            card(ui, |ui| {
                section(ui, "Health");
                metric_grid(ui, "bat_health", view, "battery", &["Wear", "Cycles"]);
            });
        });
        columns[1].vertical(|ui| {
            card(ui, |ui| {
                section(ui, "Energy");
                metric_grid(
                    ui,
                    "bat_energy",
                    view,
                    "battery",
                    &["Energy Now", "Energy Full", "Energy Design"],
                );
            });
            ui.add_space(10.0);
            card(ui, |ui| {
                section(ui, "Status");
                metric_grid(
                    ui,
                    "bat_status",
                    view,
                    "battery",
                    &["Status", "Manufacturer", "Technology", "AC Online"],
                );
            });
        });
    });
}
