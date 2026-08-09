//! Composants d'affichage de l'interface graphique.
//!
//! `mod.rs` regroupe les helpers partagés par tous les onglets :
//! sections, métriques, barres de niveau, graphes temporels et contrôles
//! de fréquence. Chaque onglet vit dans son propre module (`profiles`,
//! `cpu`, `gpu`, `memory`, `battery`, `thermal`, `io`, `network`), avec
//! une fonction `render_*` unique qui dessine son contenu.

pub(crate) mod battery;
pub(crate) mod cpu;
pub(crate) mod gpu;
pub(crate) mod io;
pub(crate) mod memory;
pub(crate) mod network;
pub(crate) mod profiles;
pub(crate) mod thermal;

use crate::core::{ReadLatest, Store, View};
use crate::themes::palette;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Largeur des barres de progression.
const BAR_WIDTH: f32 = 400.0;
/// Hauteur uniforme des graphes temporels.
const CHART_HEIGHT: f32 = 160.0;

// Seuils de code couleur (vert → jaune → rouge), centralisés pour être
// accordables au même endroit.
pub(crate) const LOAD_WARN: f64 = 50.0; // % charge / busy / mémoire
pub(crate) const LOAD_CRIT: f64 = 80.0;
pub(crate) const CPU_TEMP_WARN: f64 = 70.0; // °C cœurs CPU
pub(crate) const CPU_TEMP_CRIT: f64 = 85.0;
pub(crate) const ZONE_TEMP_WARN: f64 = 60.0; // °C zones thermiques
pub(crate) const ZONE_TEMP_CRIT: f64 = 80.0;
// Batterie : appliqués à « 100 − capacité » (bas = critique).
pub(crate) const BATTERY_WARN: f64 = 50.0;
pub(crate) const BATTERY_CRIT: f64 = 80.0;

/// En-tête de section : espacement vertical régulier + titre en gras.
pub(crate) fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.strong(title);
}

/// Mention « (read-only) » — secondaire mais lisible.
pub(crate) fn readonly_note(ui: &mut egui::Ui) {
    ui.colored_label(palette().muted, "(read-only)");
}

/// Ligne « label | valeur » alignée à droite, « — » si valeur absente.
pub(crate) fn metric_row(ui: &mut egui::Ui, label: &str, display: Option<String>) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            match display {
                Some(d) => {
                    ui.colored_label(palette().value, d);
                }
                None => {
                    ui.colored_label(palette().muted, "—");
                }
            };
        });
    });
}

/// Dernière valeur d'une métrique, formatée avec son unité native.
pub(crate) fn metric(ui: &mut egui::Ui, view: &View, kind: &str, label: &str) {
    metric_row(ui, label, view.get(kind, label).map(|s| s.display()));
}

/// Métrique de fréquence — toujours en MHz (échelle homogène dans l'UI).
pub(crate) fn metric_mhz(ui: &mut egui::Ui, view: &View, kind: &str, label: &str) {
    metric_row(
        ui,
        label,
        view.num(kind, label).map(|v| format!("{v:.0} MHz")),
    );
}

/// Barre de niveau avec code couleur de seuils.
///
/// `invert` : vrai quand une valeur basse est critique (ex. batterie) —
/// les seuils sont alors appliqués à `100 − value`.
pub(crate) fn level_bar(
    ui: &mut egui::Ui,
    value: f64,
    text: String,
    warn: f64,
    crit: f64,
    invert: bool,
) {
    let badness = if invert { 100.0 - value } else { value };
    let color = palette().threshold(badness, warn, crit);
    let frac = (value.clamp(0.0, 100.0) / 100.0) as f32;
    ui.add(
        egui::ProgressBar::new(frac)
            .text(text)
            .desired_width(BAR_WIDTH)
            .fill(color),
    );
}

/// Avertit quand min > max (le kernel clampe silencieusement ces réglages).
pub(crate) fn range_warning(ui: &mut egui::Ui, min: f64, max: f64) {
    if min > max {
        ui.colored_label(palette().warn, "⚠ min > max — clampé par le matériel");
    }
}

// ── Time chart helper ───────────────────────

/// Extrait les points d'une série temporelle sous un lock bref.
///
/// Le lock est relâché avant le rendu du graphe : il n'est jamais tenu
/// pendant la construction/tessellation de l'UI.
fn chart_points(
    store: &Arc<Mutex<Store>>,
    kind: &str,
    label: &str,
    window_secs: f64,
) -> Vec<[f64; 2]> {
    let store = store.lock().unwrap();
    let Some(history) = store.history(kind, label) else {
        return Vec::new();
    };
    let now = Instant::now();
    history
        .iter()
        .filter_map(|(t, v)| {
            let secs_ago = now.duration_since(*t).as_secs_f64();
            // négatif → passé à gauche, 0 = maintenant à droite
            (secs_ago < window_secs).then_some([-secs_ago, *v])
        })
        .collect()
}

/// Affiche un graphe temporel pour une métrique numérique : en-tête
/// (titre dérivé de la fenêtre + valeur courante) puis courbe.
/// Utilise le ring buffer `Store.history` déjà alimenté par le polling.
pub(crate) fn time_chart(
    ui: &mut egui::Ui,
    store: &Arc<Mutex<Store>>,
    kind: &str,
    label: &str,
    color: egui::Color32,
    window_secs: f64,
) {
    let points = chart_points(store, kind, label, window_secs);

    // En-tête : titre + valeur courante (unité native du Sample).
    let current = store.lock().unwrap().get(kind, label).map(|s| s.display());
    let mins = (window_secs / 60.0).round() as u64;
    ui.horizontal(|ui| {
        ui.strong(format!("{label} History ({mins} min)"));
        if let Some(cur) = current {
            ui.colored_label(color, cur);
        }
    });

    if points.len() < 2 {
        ui.label("⏳ collecting…");
        return;
    }

    let line = Line::new(PlotPoints::from(points))
        .color(color)
        .width(1.5_f32)
        .name(label);

    Plot::new(format!("chart_{kind}_{label}"))
        .height(CHART_HEIGHT)
        .x_axis_label("seconds")
        .show_grid(true)
        .show(ui, |plot_ui| {
            plot_ui.line(line);
        });
}

// ── Frequency slider helper ────────────────

/// Slider de fréquence.
///
/// Retourne `Some(valeur_en_mhz)` si l'utilisateur a terminé un
/// changement (fin de drag, clic ou clavier), `None` sinon. L'appelant
/// décide du chemin d'écriture.
///
/// - Lit la valeur extracte depuis le View
/// - Bornes depuis les labels `hw_min_label` / `hw_max_label`
/// - L'écriture n'est déclenchée qu'à la fin du drag (jamais en rafale)
/// - `display` : texte affiché à côté du slider (vide dans les grilles
///   denses où l'en-tête de colonne suffit)
pub(crate) fn freq_slider(
    ui: &mut egui::Ui,
    view: &View,
    kind: &str,
    label: &str,
    hw_min_label: &str,
    hw_max_label: &str,
    display: &str,
) -> Option<f64> {
    let hw_min = view.num(kind, hw_min_label).unwrap_or(0.0);
    let hw_max = view.num(kind, hw_max_label).unwrap_or(4000.0);
    let current = view.num(kind, label).unwrap_or(hw_min);

    let mut val = current;
    let response = ui.add(
        egui::Slider::new(&mut val, hw_min..=hw_max)
            .text(display)
            .suffix(" MHz")
            .clamping(egui::SliderClamping::Always),
    );

    // Applique à la fin du drag (ou sur changement clavier/clic) — jamais
    // en rafale pendant un drag : chaque changement écrirait 8 fichiers
    // sysfs par slider (16 pour la paire min/max).
    if response.drag_stopped() || (response.changed() && !response.dragged()) {
        return Some(val);
    }
    None
}

/// Sélecteur de gouverneur.
///
/// Affiche le gouverneur courant avec la liste des disponibles
/// (cachée — constante noyau, parsée une seule fois par le Dashboard).
/// Retourne `Some(nouveau)` si l'utilisateur a choisi un autre
/// gouverneur, `None` sinon.
pub(crate) fn governor_selector(
    ui: &mut egui::Ui,
    view: &View,
    current_label: &str,
    available: &Option<Vec<String>>,
) -> Option<String> {
    let current = view.text("cpu", current_label)?;
    let available = match available {
        Some(list) if !list.is_empty() => list,
        _ => {
            // Liste indisponible → simple lecture seule
            ui.colored_label(palette().ok_soft, &current);
            return None;
        }
    };

    let mut selected = current.clone();
    egui::ComboBox::from_id_salt(format!("gov_{current_label}"))
        .selected_text(selected.clone())
        .show_ui(ui, |ui| {
            for g in available {
                ui.selectable_value(&mut selected, g.clone(), g);
            }
        });
    (selected != current).then_some(selected)
}
