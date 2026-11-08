//! Onglet Profils : prédéfinis, personnalisés (apply/delete) et
//! sauvegarde des réglages courants.
//!
//! Post-audit : chaque profil est une carte avec nom en gras et description
//! atténuée (hiérarchie lisible), badge ACTIVE (plus de glyphe « coche »),
//! boutons Apply/Delete à vraie taille tactile, feedback en bandeau sans
//! emoji, formulaire de sauvegarde regroupé dans une carte.

use super::{badge, card, section};
use crate::core::{ReadLatest, View};
use crate::profiles::{self, Profile};
use crate::themes::palette;
use eframe::egui;

/// État de l'onglet Profils : liste chargée (prédéfinis + personnalisés),
/// nom du profil en cours de création, feedback utilisateur.
pub(crate) struct ProfileTab {
    pub(crate) profiles: Vec<Profile>,
    new_name: String,
    feedback: Option<(bool, String)>,
}

impl ProfileTab {
    fn new() -> Self {
        Self {
            profiles: profiles::load_all(),
            new_name: String::new(),
            feedback: None,
        }
    }

    /// Recharge la liste depuis le disque (après création/suppression).
    fn reload(&mut self) {
        self.profiles = profiles::load_all();
    }
}

impl Default for ProfileTab {
    fn default() -> Self {
        Self::new()
    }
}

/// Action demandée par l'utilisateur sur une ligne de profil.
enum ProfileAction {
    Apply,
    Delete,
}

/// Rendu de l'onglet Profils : prédéfinis, personnalisés (apply/delete),
/// sauvegarde des réglages courants.
pub(crate) fn render_profiles(ui: &mut egui::Ui, view: &View, tab: &mut ProfileTab) {
    ui.label(
        egui::RichText::new("Profiles")
            .monospace()
            .size(20.0)
            .strong()
            .color(palette().value),
    );
    ui.label(
        egui::RichText::new("Apply a power profile, then fine-tune with the CPU/GPU sliders.")
            .small()
            .color(palette().muted),
    );
    ui.add_space(6.0);

    // Feedback en bandeau teinté : visible, distinct du contenu courant.
    if let Some((ok, msg)) = &tab.feedback {
        let p = palette();
        let (fill, fg) = if *ok {
            (p.tint(p.ok, 18), p.ok_soft)
        } else {
            (p.tint(p.crit, 22), p.crit_soft)
        };
        egui::Frame::default()
            .fill(fill)
            .stroke(egui::Stroke::new(1.0_f32, fg))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::symmetric(10.0, 7.0))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(msg).small().strong().color(fg));
            });
        ui.add_space(4.0);
    }

    let (predefined, custom) = profiles::split(&tab.profiles);

    let mut action = None;
    for p in predefined {
        action = profile_card(ui, view, p, false).or(action);
    }

    ui.add_space(6.0);
    section(ui, "Custom Profiles");
    if custom.is_empty() {
        ui.label(
            egui::RichText::new("None yet — save your current settings below.")
                .small()
                .color(palette().muted),
        );
    }
    for p in custom {
        action = profile_card(ui, view, p, true).or(action);
    }

    // Exécute l'action demandée une fois les emprunts des lignes terminés.
    if let Some((action, name)) = action {
        match action {
            ProfileAction::Apply => {
                let feedback = match profiles::find(&tab.profiles, &name) {
                    Some(p) => match p.apply() {
                        Ok(()) => (true, format!("Profile \"{name}\" applied.")),
                        Err(e) => (false, format!("\"{name}\": {e}")),
                    },
                    None => (false, format!("Profile \"{name}\" not found.")),
                };
                tab.feedback = Some(feedback);
            }
            ProfileAction::Delete => {
                tab.feedback = Some(match profiles::remove_custom(&name) {
                    Ok(_) => {
                        tab.reload();
                        (true, format!("\"{name}\" deleted."))
                    }
                    Err(e) => (false, format!("{e}")),
                });
            }
        }
    }

    // ── Sauvegarde des réglages courants ──
    ui.add_space(6.0);
    card(ui, |ui| {
        section(ui, "Save Current Settings");
        ui.label(
            egui::RichText::new("Captures the current governor and frequency caps as a reusable profile.")
                .small()
                .color(palette().muted),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(
                egui::TextEdit::singleline(&mut tab.new_name)
                    .desired_width(220.0)
                    .hint_text("e.g. Silent evening"),
            );
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Save profile").monospace().strong(),
                    )
                    .min_size(egui::vec2(96.0, 28.0)),
                )
                .clicked()
            {
                save_profile(tab, view);
            }
        });
    });
}

/// Carte de profil : nom, description, badge actif, boutons.
/// Retourne l'action demandée (exécutée par l'appelant après la boucle).
fn profile_card(
    ui: &mut egui::Ui,
    view: &View,
    p: &Profile,
    custom: bool,
) -> Option<(ProfileAction, String)> {
    let mut action = None;
    let active = p.is_active(view);
    let body = |ui: &mut egui::Ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(&p.name)
                        .monospace()
                        .size(15.0)
                        .strong()
                        .color(palette().value),
                );
                ui.label(
                    egui::RichText::new(describe_profile(p, view))
                        .small()
                        .color(palette().muted),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if active {
                    badge(ui, "ACTIVE", palette().ok_soft);
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Apply").monospace().strong(),
                        )
                        .min_size(egui::vec2(80.0, 28.0)),
                    )
                    .clicked()
                {
                    action = Some((ProfileAction::Apply, p.name.clone()));
                }
                if custom
                    && ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Delete")
                                    .monospace()
                                    .color(palette().crit),
                            )
                            .min_size(egui::vec2(80.0, 28.0)),
                        )
                        .clicked()
                {
                    action = Some((ProfileAction::Delete, p.name.clone()));
                }
            });
        });
    };
    // Le profil actif est surligné pour un repérage immédiat dans la liste.
    if active {
        egui::Frame::default()
            .fill(palette().tint(palette().ok, 14))
            .stroke(egui::Stroke::new(1.5_f32, palette().ok_soft))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, body);
    } else {
        card(ui, body);
    }
    ui.add_space(6.0);
    action
}

/// Capture et sauvegarde les réglages courants sous le nom saisi.
fn save_profile(tab: &mut ProfileTab, view: &View) {
    let name = tab.new_name.trim().to_string();
    if name.is_empty() {
        tab.feedback = Some((false, "A name is required.".into()));
        return;
    }
    let reserved = profiles::predefined()
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case(&name));
    if reserved {
        tab.feedback = Some((false, "Name already used by a predefined profile.".into()));
        return;
    }
    match Profile::from_store(view, &name) {
        Some(p) => match profiles::add_custom(&p) {
            Ok(_) => {
                tab.new_name.clear();
                tab.reload();
                tab.feedback = Some((true, format!("Profile \"{name}\" saved.")));
            }
            Err(e) => tab.feedback = Some((false, format!("{e}"))),
        },
        None => {
            tab.feedback = Some((
                false,
                "Nothing to capture: no governor/frequency readings available.".into(),
            ))
        }
    }
}

/// Décrit un profil avec les valeurs cibles réelles (depuis les limites HW).
fn describe_profile(p: &Profile, view: &View) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(g) = &p.governor {
        parts.push(g.clone());
    } else if let Some(govs) = &p.core_governor {
        if govs.iter().all(|g| g == &govs[0]) {
            parts.push(govs[0].clone());
        }
    }
    if let Some(r) = p.cpu_max {
        match view.num("cpu", "HW Max Freq") {
            Some(hw) => parts.push(format!("CPU max {:.0} MHz", hw * r)),
            None => parts.push(format!("CPU max {:.0}%", r * 100.0)),
        }
    }
    if let Some(v) = &p.core_min {
        parts.push(range_text("CPU min", v));
    }
    if let Some(v) = &p.core_max {
        parts.push(range_text("CPU max", v));
    }
    if let Some(m) = p.gpu_min {
        parts.push(format!("GPU min {:.0} MHz", m));
    }
    if let Some(r) = p.gpu_max {
        match view.num("gpu", "RP0 Max") {
            Some(hw) => parts.push(format!("GPU max {:.0} MHz", hw * r)),
            None => parts.push(format!("GPU max {:.0}%", r * 100.0)),
        }
    }
    parts.join(" · ")
}

/// Formate une plage de fréquences par cœur (valeur unique si uniforme).
fn range_text(prefix: &str, values: &[f64]) -> String {
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if lo == hi {
        format!("{prefix} {lo:.0} MHz")
    } else {
        format!("{prefix} {lo:.0}-{hi:.0} MHz")
    }
}
