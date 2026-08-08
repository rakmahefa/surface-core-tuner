//! Interface graphique — egui dashboard.
//!
//! Architecture :
//! - Thread d'arrière-plan qui échantillonne les sources à `FPS` (5 Hz)
//! - Store partagé via Arc<Mutex<>>
//! - egui rafraîchi à chaque nouvel échantillon via request_repaint()
//! - Le rendu lit un `View` (snapshot pris sous lock bref) — le Mutex
//!   n'est jamais tenu pendant le rendu, et les graphes n'extraient
//!   leurs points que sous des locks brefs.

use crate::core::*;
use crate::sources;
use eframe::{egui, Frame};
use egui_plot::{Line, Plot, PlotPoints};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Fréquence d'échantillonnage et de rafraîchissement de la GUI (Hz).
const FPS: u64 = 5;
/// Intervalle de polling correspondant : 1000 / FPS.
const POLL_MS: u64 = 1000 / FPS;
/// Profondeur d'historique conservée (30 min), indépendante du FPS :
/// la capacité du ring buffer vaut `HISTORY_SECS * FPS`.
const HISTORY_SECS: u64 = 30 * 60;

/// Lance le mode graphique.
pub fn run() {
    let store = Arc::new(Mutex::new(Store::new((HISTORY_SECS * FPS) as usize)));

    let app = Dashboard {
        store: store.clone(),
        tab: Tab::Profiles,
        profile_tab: ProfileTab::new(),
        governors: None,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 680.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("NyVoara — Power & Performance"),
        // Backend de rendu : wgpu (Vulkan préféré), pas l'OpenGL par défaut.
        renderer: eframe::Renderer::Wgpu,
        // Synchro verticale si le compositeur la supporte (chemin glow ;
        // pour wgpu c'est `present_mode` ci-dessous qui contrôle le swap).
        vsync: true,
        wgpu_options: wgpu_options(),
        ..Default::default()
    };

    eframe::run_native(
        "NyVoara",
        options,
        Box::new(|cc| {
            // Style global : espacement et cibles tactiles élargis (Surface tactile).
            let mut style = (*cc.egui_ctx.style()).clone();
            style.spacing.item_spacing = egui::vec2(10.0, 6.0);
            style.spacing.button_padding = egui::vec2(6.0, 3.0);
            style.spacing.interact_size = egui::vec2(40.0, 24.0);
            style.spacing.slider_width = 140.0;
            cc.egui_ctx.set_style(style);

            // Spawn le thread de polling, avec ctx pour request_repaint
            let ctx = cc.egui_ctx.clone();
            let poller_store = store.clone();
            std::thread::spawn(move || {
                let mut sampler = sources::build_sampler();
                loop {
                    let samples = sampler.poll_all();
                    poller_store.lock().unwrap().push_batch(samples);
                    ctx.request_repaint();
                    std::thread::sleep(Duration::from_millis(POLL_MS));
                }
            });

            Ok(Box::new(app))
        }),
    )
    .expect("eframe failed");
}

/// Configuration wgpu : Vulkan prioritaire (repli GL), vsync si disponible.
fn wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    let mut opts = eframe::egui_wgpu::WgpuConfiguration {
        // vsync : Fifo si supporté, sinon repli automatique.
        present_mode: eframe::wgpu::PresentMode::AutoVsync,
        ..Default::default()
    };
    if let eframe::egui_wgpu::WgpuSetup::CreateNew {
        supported_backends, ..
    } = &mut opts.wgpu_setup
    {
        // Vulkan choisi quand un pilote existe (Mesa ANV ici) ; GL en secours.
        *supported_backends = eframe::wgpu::Backends::VULKAN | eframe::wgpu::Backends::GL;
    }
    opts
}

// ── Tabs ────────────────────────────────────

#[derive(PartialEq)]
enum Tab {
    Profiles,
    Cpu,
    Gpu,
    Memory,
    Battery,
    Thermal,
    Io,
    Network,
}

// ── Dashboard ───────────────────────────────

struct Dashboard {
    store: Arc<Mutex<Store>>,
    tab: Tab,
    profile_tab: ProfileTab,
    /// Gouverneurs disponibles — constante noyau, parsée une seule fois.
    governors: Option<Vec<String>>,
}

impl eframe::App for Dashboard {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Snapshot des dernières valeurs sous un lock bref : le rendu ne
        // tient jamais le Mutex (anti-pattern « gros Mutex pendant le
        // rendu »), le thread de polling n'est jamais bloqué par une frame.
        let view = {
            let store = self.store.lock().unwrap();
            View::snapshot(&store)
        };

        // Gouverneurs disponibles : constante noyau — parsée une seule fois.
        if self.governors.is_none() {
            self.governors = view
                .text("cpu", "Available Governors")
                .map(|s| s.split_whitespace().map(String::from).collect());
        }

        // ── Header + Tabs ──
        egui::TopBottomPanel::top("header")
            .show_separator_line(true)
            .frame(egui::Frame {
                fill: ctx.style().visuals.panel_fill,
                inner_margin: egui::Margin::symmetric(10.0, 8.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("⚡ NyVoara");
                    ui.label("Power & Performance Dashboard");
                });
                ui.add_space(4.0);
                // Bande d'onglets : fond discret pour la distinguer du contenu.
                egui::Frame::none()
                    .fill(ctx.style().visuals.faint_bg_color)
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(4.0, 3.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            tab_button(ui, &mut self.tab, Tab::Profiles, "Profiles");
                            tab_button(ui, &mut self.tab, Tab::Cpu, "CPU");
                            tab_button(ui, &mut self.tab, Tab::Gpu, "GPU");
                            tab_button(ui, &mut self.tab, Tab::Memory, "Memory");
                            tab_button(ui, &mut self.tab, Tab::Battery, "Battery");
                            tab_button(ui, &mut self.tab, Tab::Thermal, "Thermal");
                            tab_button(ui, &mut self.tab, Tab::Io, "I/O");
                            tab_button(ui, &mut self.tab, Tab::Network, "Network");
                        });
                    });
            });

        // ── Status bar (essentiel uniquement : batterie, AC, CPU0, profil) ──
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Batterie : capacité colorée selon les seuils (bas = critique)
                if let Some(cap) = view.num("battery", "Capacity") {
                    let color = threshold_color(100.0 - cap, BATTERY_WARN, BATTERY_CRIT);
                    ui.colored_label(color, format!("🔋 {cap:.0}%"));
                    ui.separator();
                }
                if let Some(ac) = view.num("battery", "AC Online") {
                    if ac > 0.0 {
                        ui.colored_label(egui::Color32::GREEN, "AC connected");
                    } else {
                        ui.colored_label(egui::Color32::YELLOW, "On battery");
                    }
                    ui.separator();
                }
                if let Some(freq) = view.num("cpu", "CPU0 Freq") {
                    ui.label(format!("CPU0: {freq:.0} MHz"));
                    ui.separator();
                }
                if let Some(p) = self
                    .profile_tab
                    .profiles
                    .iter()
                    .find(|p| p.is_active(&view))
                {
                    ui.label(format!("Profile: {}", p.name));
                }
            });
        });

        // ── Content ──
        let mut profile_tab = std::mem::take(&mut self.profile_tab);
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                Tab::Profiles => render_profiles(ui, &view, &mut profile_tab),
                Tab::Cpu => render_cpu(ui, &view, &self.store, &self.governors),
                Tab::Gpu => render_gpu(ui, &view, &self.store),
                Tab::Memory => render_memory(ui, &view, &self.store),
                Tab::Battery => render_battery(ui, &view),
                Tab::Thermal => render_thermal(ui, &view),
                Tab::Io => render_io(ui, &view),
                Tab::Network => render_network(ui, &view),
            });
        });
        self.profile_tab = profile_tab;
    }
}

fn tab_button(ui: &mut egui::Ui, active: &mut Tab, tab: Tab, label: &str) {
    let selected = *active == tab;
    if ui.selectable_label(selected, label).clicked() {
        *active = tab;
    }
}

// ── Helpers d'affichage ─────────────────────

/// Couleur de texte secondaire — gris clair lisible sur fond sombre.
const MUTED_COLOR: egui::Color32 = egui::Color32::from_gray(150);
/// Couleur des valeurs numériques.
const VALUE_COLOR: egui::Color32 = egui::Color32::LIGHT_BLUE;
/// Largeur des barres de progression.
const BAR_WIDTH: f32 = 400.0;
/// Hauteur uniforme des graphes temporels.
const CHART_HEIGHT: f32 = 160.0;

// Seuils de code couleur (vert → jaune → rouge), centralisés pour être
// accordables au même endroit.
const LOAD_WARN: f64 = 50.0; // % charge / busy / mémoire
const LOAD_CRIT: f64 = 80.0;
const CPU_TEMP_WARN: f64 = 70.0; // °C cœurs CPU
const CPU_TEMP_CRIT: f64 = 85.0;
const ZONE_TEMP_WARN: f64 = 60.0; // °C zones thermiques
const ZONE_TEMP_CRIT: f64 = 80.0;
// Batterie : appliqués à « 100 − capacité » (bas = critique).
const BATTERY_WARN: f64 = 50.0;
const BATTERY_CRIT: f64 = 80.0;

/// Couleur de seuil : vert sous `warn`, jaune sous `crit`, rouge sinon.
fn threshold_color(value: f64, warn: f64, crit: f64) -> egui::Color32 {
    if value >= crit {
        egui::Color32::RED
    } else if value >= warn {
        egui::Color32::YELLOW
    } else {
        egui::Color32::GREEN
    }
}

/// En-tête de section : espacement vertical régulier + titre en gras.
fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.strong(title);
}

/// Mention « (read-only) » — secondaire mais lisible.
fn readonly_note(ui: &mut egui::Ui) {
    ui.colored_label(MUTED_COLOR, "(read-only)");
}

/// Ligne « label | valeur » alignée à droite, « — » si valeur absente.
fn metric_row(ui: &mut egui::Ui, label: &str, display: Option<String>) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            match display {
                Some(d) => {
                    ui.colored_label(VALUE_COLOR, d);
                }
                None => {
                    ui.colored_label(MUTED_COLOR, "—");
                }
            };
        });
    });
}

/// Dernière valeur d'une métrique, formatée avec son unité native.
fn metric(ui: &mut egui::Ui, view: &View, kind: &str, label: &str) {
    metric_row(ui, label, view.get(kind, label).map(|s| s.display()));
}

/// Métrique de fréquence — toujours en MHz (échelle homogène dans l'UI).
fn metric_mhz(ui: &mut egui::Ui, view: &View, kind: &str, label: &str) {
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
fn level_bar(ui: &mut egui::Ui, value: f64, text: String, warn: f64, crit: f64, invert: bool) {
    let badness = if invert { 100.0 - value } else { value };
    let color = threshold_color(badness, warn, crit);
    let frac = (value.clamp(0.0, 100.0) / 100.0) as f32;
    ui.add(
        egui::ProgressBar::new(frac)
            .text(text)
            .desired_width(BAR_WIDTH)
            .fill(color),
    );
}

/// Avertit quand min > max (le kernel clampe silencieusement ces réglages).
fn range_warning(ui: &mut egui::Ui, min: f64, max: f64) {
    if min > max {
        ui.colored_label(
            egui::Color32::YELLOW,
            "⚠ min > max — clampé par le matériel",
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

/// Affiche un graphe temporel pour une métrique numérique : en-tête
/// (titre dérivé de la fenêtre + valeur courante) puis courbe.
/// Utilise le ring buffer `Store.history` déjà alimenté par le polling.
fn time_chart(
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
        .width(1.5)
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
/// - Lit la valeur actuelle depuis le View
/// - Bornes depuis les labels `hw_min_label` / `hw_max_label`
/// - L'écriture n'est déclenchée qu'à la fin du drag (jamais en rafale)
/// - `display` : texte affiché à côté du slider (vide dans les grilles
///   denses où l'en-tête de colonne suffit)
fn freq_slider(
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
fn governor_selector(
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
            ui.colored_label(egui::Color32::LIGHT_GREEN, &current);
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

// ── Profiles Tab ────────────────────────────

/// État de l'onglet Profils : liste chargée (prédéfinis + personnalisés),
/// nom du profil en cours de création, feedback utilisateur.
struct ProfileTab {
    profiles: Vec<crate::profiles::Profile>,
    new_name: String,
    feedback: Option<(bool, String)>,
}

impl ProfileTab {
    fn new() -> Self {
        Self {
            profiles: crate::profiles::load_all(),
            new_name: String::new(),
            feedback: None,
        }
    }

    /// Recharge la liste depuis le disque (après création/suppression).
    fn reload(&mut self) {
        self.profiles = crate::profiles::load_all();
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
fn render_profiles(ui: &mut egui::Ui, view: &View, tab: &mut ProfileTab) {
    ui.heading("Profiles");
    ui.label("Apply a predefined power profile, then fine-tune with the CPU/GPU sliders.");

    // Feedback teinté : visible immédiatement, distinct du texte courant.
    if let Some((ok, msg)) = &tab.feedback {
        let (fill, fg) = if *ok {
            (
                egui::Color32::from_rgba_unmultiplied(0, 200, 0, 18),
                egui::Color32::LIGHT_GREEN,
            )
        } else {
            (
                egui::Color32::from_rgba_unmultiplied(255, 60, 60, 22),
                egui::Color32::LIGHT_RED,
            )
        };
        egui::Frame::none()
            .fill(fill)
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(8.0, 6.0))
            .show(ui, |ui| {
                ui.colored_label(fg, msg);
            });
        ui.add_space(4.0);
    }

    let (predefined, custom) = crate::profiles::split(&tab.profiles);

    let mut action = None;
    for p in predefined {
        action = profile_row(ui, view, p, false).or(action);
    }

    ui.separator();
    section(ui, "Custom Profiles");
    if custom.is_empty() {
        ui.label("None yet — save your current settings below.");
    }
    for p in custom {
        action = profile_row(ui, view, p, true).or(action);
    }

    // Exécute l'action demandée une fois les emprunts des lignes terminés.
    if let Some((action, name)) = action {
        match action {
            ProfileAction::Apply => {
                let feedback = match crate::profiles::find(&tab.profiles, &name) {
                    Some(p) => match p.apply() {
                        Ok(()) => (true, format!("✓ Profile «{name}» applied.")),
                        Err(e) => (false, format!("⚠ «{name}»: {e}")),
                    },
                    None => (false, format!("⚠ Profile «{name}» not found.")),
                };
                tab.feedback = Some(feedback);
            }
            ProfileAction::Delete => {
                tab.feedback = Some(match crate::profiles::remove_custom(&name) {
                    Ok(_) => {
                        tab.reload();
                        (true, format!("✓ «{name}» deleted."))
                    }
                    Err(e) => (false, format!("⚠ {e}")),
                });
            }
        }
    }

    // ── Sauvegarde des réglages courants ──
    ui.separator();
    section(ui, "Save Current Settings");
    ui.label("Captures the current governor and frequency caps as a reusable profile.");
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut tab.new_name);
        if ui.button("Save").clicked() {
            save_profile(tab, view);
        }
    });
}

/// Affiche une ligne de profil : nom, description, badge actif, boutons.
/// Retourne l'action demandée (exécutée par l'appelant après la boucle).
fn profile_row(
    ui: &mut egui::Ui,
    view: &View,
    p: &crate::profiles::Profile,
    custom: bool,
) -> Option<(ProfileAction, String)> {
    let mut action = None;
    let active = p.is_active(view);
    // Ligne complète : description à gauche, badge/boutons à droite.
    let mut row = |ui: &mut egui::Ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.strong(&p.name);
                ui.label(describe_profile(p, view));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if active {
                    ui.colored_label(egui::Color32::GREEN, "✓ active");
                }
                if ui.button("Apply").clicked() {
                    action = Some((ProfileAction::Apply, p.name.clone()));
                }
                if custom && ui.button("Delete").clicked() {
                    action = Some((ProfileAction::Delete, p.name.clone()));
                }
            });
        });
    };
    // Le profil actif est surligné pour un repérage immédiat dans la liste.
    if active {
        egui::Frame::none()
            .fill(egui::Color32::from_rgba_unmultiplied(0, 200, 0, 14))
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, row);
    } else {
        row(ui);
    }
    ui.separator();
    action
}

/// Capture et sauvegarde les réglages courants sous le nom saisi.
fn save_profile(tab: &mut ProfileTab, view: &View) {
    let name = tab.new_name.trim().to_string();
    if name.is_empty() {
        tab.feedback = Some((false, "⚠ Name required.".into()));
        return;
    }
    let reserved = crate::profiles::predefined()
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case(&name));
    if reserved {
        tab.feedback = Some((false, "⚠ Name already used by a predefined profile.".into()));
        return;
    }
    match crate::profiles::Profile::from_store(view, &name) {
        Some(p) => match crate::profiles::add_custom(&p) {
            Ok(_) => {
                tab.new_name.clear();
                tab.reload();
                tab.feedback = Some((true, format!("✓ Profile «{name}» saved.")));
            }
            Err(e) => tab.feedback = Some((false, format!("⚠ {e}"))),
        },
        None => {
            tab.feedback = Some((
                false,
                "⚠ Nothing to capture: no governor/frequency readings available.".into(),
            ))
        }
    }
}

/// Décrit un profil avec les valeurs cibles réelles (depuis les limites HW).
fn describe_profile(p: &crate::profiles::Profile, view: &View) -> String {
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
        format!("{prefix} {lo:.0}–{hi:.0} MHz")
    }
}

// ── CPU Tab ─────────────────────────────────

fn render_cpu(
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
            let _ = crate::profiles::set_cpu_governor(&gov);
        }
    });
    ui.separator();

    // Per-core load
    section(ui, "Per-Core Load");
    for cpu in 0..crate::paths::CPU_THREADS {
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
    time_chart(
        ui,
        store,
        "cpu",
        "CPU0 Load",
        egui::Color32::LIGHT_BLUE,
        300.0,
    );
    ui.separator();

    // Per-core frequency
    section(ui, "Frequency");
    egui::Grid::new("cpu_freq")
        .num_columns(4)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            for cpu in 0..crate::paths::CPU_THREADS {
                let label = format!("CPU{cpu} Freq");
                match view.num("cpu", &label) {
                    Some(f) => {
                        ui.label(format!("CPU{cpu}: {f:.0} MHz"));
                    }
                    None => {
                        ui.colored_label(MUTED_COLOR, format!("CPU{cpu}: —"));
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
                let _ = crate::profiles::set_cpu_min_mhz(val);
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
                let _ = crate::profiles::set_cpu_max_mhz(val);
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
                    for cpu in 0..crate::paths::CPU_THREADS {
                        ui.label(format!("CPU{cpu}:"));

                        // Gouverneur individuel
                        if let Some(gov) =
                            governor_selector(ui, view, &format!("CPU{cpu} Governor"), governors)
                        {
                            let _ = crate::profiles::set_cpu_governor_core(cpu, &gov);
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
                            let _ = crate::profiles::set_cpu_min_mhz_core(cpu, val);
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
                            let _ = crate::profiles::set_cpu_max_mhz_core(cpu, val);
                        }

                        // Fréquence courante (ou placeholder)
                        let now = view
                            .num("cpu", &format!("CPU{cpu} Freq"))
                            .map(|f| format!("{f:.0} MHz"))
                            .unwrap_or_else(|| "—".to_string());
                        ui.colored_label(MUTED_COLOR, now);
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
                        let color = threshold_color(temp, CPU_TEMP_WARN, CPU_TEMP_CRIT);
                        ui.colored_label(color, format!("{t}: {temp:.1}°C"));
                    }
                    None => {
                        ui.colored_label(MUTED_COLOR, format!("{t}: —"));
                    }
                }
                ui.end_row();
            }
        });
    ui.separator();

    // Temperature chart
    time_chart(ui, store, "cpu", "Package Temp", egui::Color32::RED, 300.0);
    ui.separator();

    // Pressure stall
    section(ui, "Pressure Stall (avg10)");
    egui::Grid::new("cpu_psi").num_columns(2).show(ui, |ui| {
        metric(ui, view, "cpu", "PSI some avg10");
        metric(ui, view, "cpu", "PSI full avg10");
    });
}

// ── GPU Tab ─────────────────────────────────

fn render_gpu(ui: &mut egui::Ui, view: &View, store: &Arc<Mutex<Store>>) {
    ui.heading("GPU");

    // Fréquences courantes (lecture seule)
    section(ui, "Current");
    ui.horizontal(|ui| {
        metric_mhz(ui, view, "gpu", "Cur Freq");
        metric_mhz(ui, view, "gpu", "Act Freq");
    });
    ui.separator();

    // Interactive sliders (écritures sysfs — nécessite le chemin DRM du GPU)
    if crate::paths::gpu_dir().is_some() {
        section(ui, "Frequency Scaling");
        ui.label("Drag to adjust. Applies immediately.");
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("RP1 (Min):");
                if let Some(val) = freq_slider(
                    ui, view, "gpu", "Min Freq", "RP1 Min", "RP0 Max", "Min Freq",
                ) {
                    let _ = crate::profiles::set_gpu_min_mhz(val);
                }
            });
            ui.vertical(|ui| {
                ui.label("RP0 (Max):");
                if let Some(val) = freq_slider(
                    ui, view, "gpu", "Max Freq", "RP1 Min", "RP0 Max", "Max Freq",
                ) {
                    let _ = crate::profiles::set_gpu_max_mhz(val);
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
    time_chart(ui, store, "gpu", "Busy", egui::Color32::LIGHT_GREEN, 300.0);
}

// ── Memory Tab ──────────────────────────────

fn render_memory(ui: &mut egui::Ui, view: &View, store: &Arc<Mutex<Store>>) {
    ui.heading("Memory");

    if let Some(used_pct) = view.num("memory", "Used%") {
        level_bar(
            ui,
            used_pct,
            format!("RAM Used: {used_pct:.1}%"),
            LOAD_WARN,
            LOAD_CRIT,
            false,
        );
    }
    ui.separator();

    section(ui, "RAM");
    egui::Grid::new("mem").num_columns(2).show(ui, |ui| {
        metric(ui, view, "memory", "Total");
        metric(ui, view, "memory", "Used");
        metric(ui, view, "memory", "Available");
        metric(ui, view, "memory", "Free");
        metric(ui, view, "memory", "Buffers");
        metric(ui, view, "memory", "Cached");
    });
    ui.separator();

    time_chart(
        ui,
        store,
        "memory",
        "Used%",
        egui::Color32::LIGHT_YELLOW,
        300.0,
    );
    ui.separator();

    section(ui, "Swap");
    egui::Grid::new("swap").num_columns(2).show(ui, |ui| {
        metric(ui, view, "memory", "Swap Total");
        metric(ui, view, "memory", "Swap Used");
        metric(ui, view, "memory", "Swap Free");
        metric(ui, view, "memory", "Swap Cached");
    });
    ui.separator();

    section(ui, "Pressure Stall (avg10)");
    egui::Grid::new("mem_psi").num_columns(2).show(ui, |ui| {
        metric(ui, view, "memory", "PSI some avg10");
        metric(ui, view, "memory", "PSI full avg10");
    });
}

// ── Battery Tab ─────────────────────────────

fn render_battery(ui: &mut egui::Ui, view: &View) {
    ui.heading("Battery");

    // Barre de capacité — bas = critique (seuils inversés)
    if let Some(cap) = view.num("battery", "Capacity") {
        level_bar(
            ui,
            cap,
            format!("🔋 {cap:.0}%"),
            BATTERY_WARN,
            BATTERY_CRIT,
            true,
        );
    }
    ui.separator();

    section(ui, "Power");
    egui::Grid::new("bat_power").num_columns(2).show(ui, |ui| {
        metric(ui, view, "battery", "Power");
        metric(ui, view, "battery", "Voltage");
    });
    ui.separator();

    section(ui, "Energy");
    egui::Grid::new("bat_energy").num_columns(2).show(ui, |ui| {
        metric(ui, view, "battery", "Energy Now");
        metric(ui, view, "battery", "Energy Full");
        metric(ui, view, "battery", "Energy Design");
    });
    ui.separator();

    section(ui, "Health");
    egui::Grid::new("bat_health").num_columns(2).show(ui, |ui| {
        metric(ui, view, "battery", "Wear");
        metric(ui, view, "battery", "Cycles");
    });
    ui.separator();

    section(ui, "Status");
    egui::Grid::new("bat_status").num_columns(2).show(ui, |ui| {
        metric(ui, view, "battery", "Status");
        metric(ui, view, "battery", "Manufacturer");
        metric(ui, view, "battery", "Technology");
        metric(ui, view, "battery", "AC Online");
    });
}

// ── Thermal Tab ─────────────────────────────

fn render_thermal(ui: &mut egui::Ui, view: &View) {
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
                    let color = threshold_color(temp, ZONE_TEMP_WARN, ZONE_TEMP_CRIT);
                    ui.label(&s.label);
                    ui.colored_label(color, format!("{temp:.1}°C"));
                    ui.end_row();
                }
            }
        });
}

// ── I/O Tab ─────────────────────────────────

fn render_io(ui: &mut egui::Ui, view: &View) {
    ui.heading("I/O");

    let io_samples = view.by_kind("io");
    for dev in &["nvme0n1", "sda"] {
        section(ui, &format!("Disk: {dev}"));
        let dev_samples: Vec<&Sample> = io_samples
            .iter()
            .filter(|s| s.label.starts_with(dev))
            .copied()
            .collect();

        if dev_samples.is_empty() {
            ui.label("No data.");
            continue;
        }

        egui::Grid::new(format!("io_{dev}"))
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for s in &dev_samples {
                    ui.label(&s.label);
                    ui.colored_label(VALUE_COLOR, s.display());
                    ui.end_row();
                }
            });
        ui.separator();
    }

    // Pressure stall
    section(ui, "Pressure Stall (avg10)");
    egui::Grid::new("io_psi").num_columns(2).show(ui, |ui| {
        metric(ui, view, "io", "PSI some avg10");
        metric(ui, view, "io", "PSI full avg10");
    });
}

// ── Network Tab ─────────────────────────────

fn render_network(ui: &mut egui::Ui, view: &View) {
    ui.heading("Network");

    section(ui, "Throughput");
    egui::Grid::new("net_rate").num_columns(2).show(ui, |ui| {
        metric(ui, view, "network", "RX Rate");
        metric(ui, view, "network", "TX Rate");
    });
    ui.separator();

    section(ui, "Total");
    egui::Grid::new("net_total").num_columns(2).show(ui, |ui| {
        metric(ui, view, "network", "RX Bytes");
        metric(ui, view, "network", "TX Bytes");
    });
    ui.separator();

    section(ui, "Temperature");
    metric(ui, view, "network", "WiFi Temp");
}
