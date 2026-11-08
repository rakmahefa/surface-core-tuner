//! Composants d'affichage de l'interface graphique.
//!
//! `mod.rs` regroupe les briques partagées par tous les onglets : cartes,
//! en-têtes de section, tableaux de métriques (deux colonnes stables, sans
//! chevauchement), barres de niveau, graphiques temporels bornés, états
//! vides et contrôles de fréquence sécurisés. Chaque onglet vit dans son
//! propre module avec une fonction `render_*` unique.
//!
//! Règles issues de l'audit UI/UX :
//! - aucune valeur ne s'affiche en rouge (rouge = seuil critique uniquement) ;
//! - aucune donnée manquante ne se confond avec une valeur (tiret atténué) ;
//! - aucun emoji dans les libellés (polices egui non couvrantes) ;
//! - les graphiques affichent exactement la fenêtre annoncée.

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

/// Hauteur uniforme des graphiques temporels.
const CHART_HEIGHT: f32 = 150.0;

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

// ── Cartes et sections ──────────────────────

/// Carte : cadre crème à liseré encre, le conteneur de base des sections.
/// Retourne le rendu du contenu passé en closure.
pub(crate) fn card<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::default()
        .fill(palette().card)
        .stroke(egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(23, 23, 23)))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui)
        })
        .inner
}

/// En-tête de section : petites capitales atténuées + espacement régulier.
/// Se distingue du corps par la taille, la couleur et la graisse.
pub(crate) fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(title.to_uppercase())
            .small()
            .strong()
            .color(palette().muted),
    );
    ui.add_space(2.0);
}

/// Badge d'état compact (ex. « ACTIVE », « AC ») : texte coloré sur fond teinté.
pub(crate) fn badge(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::default()
        .fill(palette().tint(color, 26))
        .stroke(egui::Stroke::new(1.0_f32, color))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(7.0, 2.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().strong().color(color));
        });
}

/// État vide explicite : titre, explication de la cause, pas de silence.
/// Occupe toute la largeur et centre son contenu pour éviter les pages
/// « désertes ».
pub(crate) fn empty_state(ui: &mut egui::Ui, title: &str, explanation: &str) {
    egui::Frame::default()
        .fill(palette().card)
        .stroke(egui::Stroke::new(1.0_f32, palette().muted))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(28.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(title)
                        .monospace()
                        .size(16.0)
                        .strong()
                        .color(palette().muted),
                );
                ui.add_space(4.0);
                ui.label(egui::RichText::new(explanation).small().color(palette().muted));
                ui.add_space(8.0);
            });
        });
}

// ── Tableaux de métriques ───────────────────

/// Ligne « libellé | valeur » dans une grille à deux colonnes dédiées.
///
/// Contraisent à l'ancien helper (valeur en layout right-to-left *dans* la
/// cellule du libellé), chaque donnée dispose de sa propre colonne : aucun
/// chevauchement possible, et les valeurs s'alignent verticalement.
pub(crate) fn metric_row(ui: &mut egui::Ui, label: &str, display: Option<String>) {
    ui.label(label);
    match display {
        Some(d) => {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(d)
                        .monospace()
                        .strong()
                        .color(palette().value),
                );
            });
        }
        None => {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(palette().muted, "— no data");
            });
        }
    }
    ui.end_row();
}

/// Dernière valeur d'une métrique, formatée avec son unité native.
pub(crate) fn metric(ui: &mut egui::Ui, view: &View, kind: &str, label: &str) {
    metric_row(ui, label, view.get(kind, label).map(|s| s.display()));
}

/// Grille de métriques « libellé / valeur » : deux colonnes stables.
/// `pairs` liste les métriques à afficher, dans l'ordre.
pub(crate) fn metric_grid(ui: &mut egui::Ui, id: &str, view: &View, kind: &str, labels: &[&str]) {
    egui::Grid::new(format!("metrics_{id}"))
        .num_columns(2)
        .spacing([24.0, 5.0])
        .show(ui, |ui| {
            for label in labels {
                metric(ui, view, kind, label);
            }
        });
}

/// Barre de niveau avec code couleur de seuils.
///
/// Le texte (valeur) est sorti de la barre : plus de libellé écrasé par le
/// remplissage. La barre occupe l'espace restant, la valeur en monospace
/// se place à droite.
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
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text).color(palette().value));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{value:.1}%"))
                    .monospace()
                    .strong()
                    .color(color),
            );
            ui.add(
                egui::ProgressBar::new(frac)
                    .desired_width(ui.available_width().max(120.0))
                    .fill(color),
            );
        });
    });
}

/// Avertit quand min > max (le kernel clampe silencieusement ces réglages).
pub(crate) fn range_warning(ui: &mut egui::Ui, min: f64, max: f64) {
    if min > max {
        ui.label(
            egui::RichText::new("! min > max — the kernel will clamp these values")
                .small()
                .strong()
                .color(palette().warn),
        );
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

/// Affiche un graphe temporel pour une métrique numérique.
///
/// Bornes X fixes sur la fenêtre demandée (l'axe ne « ment » plus : la
/// fenêtre annoncée est la fenêtre affichée, 0 = maintenant à droite).
/// L'axe Y est borné par `y_max` quand il est fourni (pourcentages).
pub(crate) fn time_chart(
    ui: &mut egui::Ui,
    store: &Arc<Mutex<Store>>,
    kind: &str,
    label: &str,
    title: &str,
    color: egui::Color32,
    window_secs: f64,
    y_max: Option<f64>,
) {
    let points = chart_points(store, kind, label, window_secs);

    // En-tête : titre + valeur courante (unité native du Sample).
    let current = store.lock().unwrap().get(kind, label).map(|s| s.display());
    let mins = (window_secs / 60.0).round() as u64;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).strong().color(palette().value));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(cur) = current {
                ui.label(
                    egui::RichText::new(format!("{cur} — last {mins} min"))
                        .small()
                        .color(palette().muted),
                );
            }
        });
    });

    if points.len() < 2 {
        ui.label(
            egui::RichText::new("collecting data…")
                .small()
                .color(palette().muted),
        );
        return;
    }

    // Bornes Y : fournies (pourcentages) ou dérivées des données.
    let data_max = points
        .iter()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let y_top = y_max.unwrap_or((data_max * 1.25).max(1.0));

    let line = Line::new(PlotPoints::from(points))
        .color(color)
        .width(2.0_f32)
        .name(label);

    Plot::new(format!("chart_{kind}_{label}"))
        .height(CHART_HEIGHT)
        .show_axes([true, true])
        .show_grid([true, true])
        .show(ui, |plot_ui| {
            // Bornes fixes : la fenêtre annoncée est la fenêtre affichée.
            // Petite marge négative en bas de l'axe Y pour que le label
            // « 0 » ne collisionne pas avec le premier label de l'axe X.
            let y_bottom = -y_top * 0.06;
            let bounds = egui_plot::PlotBounds::from_min_max(
                [-window_secs, y_bottom],
                [0.0, y_top],
            );
            plot_ui.set_plot_bounds(bounds);
            plot_ui.line(line);
        });
}

// ── Frequency slider helper ─────────────────

/// Slider de fréquence, rendu uniquement si les limites matérielles sont
/// connues et cohérentes. Sinon affiche un état « indisponible » explicite
/// (fini les sliders muets à « 0 MHz »).
///
/// Retourne `Some(valeur_en_mhz)` si l'utilisateur a terminé un
/// changement (fin de drag, clic ou clavier), `None` sinon.
pub(crate) fn freq_slider(
    ui: &mut egui::Ui,
    view: &View,
    kind: &str,
    label: &str,
    hw_min_label: &str,
    hw_max_label: &str,
    display: &str,
) -> Option<f64> {
    let hw_min = view.num(kind, hw_min_label);
    let hw_max = view.num(kind, hw_max_label);
    let current = view.num(kind, label);

    // Sans limites matérielles fiables, pas de contrôle : état explicite.
    let bounds_ok = matches!((hw_min, hw_max), (Some(lo), Some(hi)) if hi > lo);
    if !bounds_ok {
        ui.label(
            egui::RichText::new("hardware limits unavailable")
                .small()
                .color(palette().muted),
        );
        return None;
    }
    let (lo, hi) = (hw_min.unwrap(), hw_max.unwrap());

    // Le curseur vit sur la plage matérielle ; la valeur courante sert
    // d'état initial (clampée), ou le minimum si aucune lecture.
    let mut val = current.unwrap_or(lo).clamp(lo, hi);
    let response = ui.add(
        egui::Slider::new(&mut val, lo..=hi)
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
            ui.label(
                egui::RichText::new(current.clone())
                    .monospace()
                    .color(palette().ok_soft),
            );
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
