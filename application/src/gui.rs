//! Application graphique — egui dashboard.
//!
//! Ce module ne contient que la coquille applicative : fenêtre eframe,
//! thème, panneaux (entête avec onglets en pilules, barre de statut,
//! contenu) et le routage vers les onglets. Le rendu de chaque onglet et
//! les helpers d'affichage partagés vivent dans [`components`].
//!
//! Architecture :
//! - Thread d'arrière-plan qui échantillonne les sources à `FPS` (5 Hz)
//! - Store partagé via `Arc<Mutex<>>`
//! - egui rafraîchi à chaque nouvel échantillon via request_repaint()
//! - Le rendu lit un `View` (snapshot pris sous lock bref) — le Mutex
//!   n'est jamais tenu pendant le rendu, et les graphes n'extraient
//!   leurs points que sous des locks brefs.

use crate::components::profiles::ProfileTab;
use crate::components::{
    battery, cpu, gpu, io, memory, network, profiles, thermal, BATTERY_CRIT, BATTERY_WARN,
};
use crate::core::*;
use crate::library;
use crate::themes::{palette, theme};
use eframe::{egui, Frame};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
        profile_tab: ProfileTab::default(),
        governors: None,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([820.0, 560.0])
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
            // Thème global : rétro clair « Game Boy » raffiné + cibles
            // tactiles (Surface). Aucun emoji : polices non couvrantes.
            theme().apply(&cc.egui_ctx);

            // Spawn le thread de polling, avec ctx pour request_repaint
            let ctx = cc.egui_ctx.clone();
            let poller_store = store.clone();
            std::thread::spawn(move || {
                let mut sampler = library::build_sampler();
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

#[derive(PartialEq, Clone, Copy)]
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

        // ── Header + onglets ──
        egui::TopBottomPanel::top("header")
            .show_separator_line(false)
            .frame(egui::Frame {
                fill: ctx.style().visuals.window_fill,
                inner_margin: egui::Margin::symmetric(14.0, 8.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("NYVOARA")
                            .monospace()
                            .size(19.0)
                            .strong()
                            .color(palette().accent),
                    );
                    ui.label(
                        egui::RichText::new("Power & Performance Dashboard")
                            .small()
                            .color(palette().muted),
                    );
                    // Profil actif visible en permanence, à droite de l'entête.
                    if let Some(p) = self
                        .profile_tab
                        .profiles
                        .iter()
                        .find(|p| p.is_active(&view))
                    {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                crate::components::badge(ui, "ACTIVE", palette().ok_soft);
                                ui.label(
                                    egui::RichText::new(&p.name)
                                        .monospace()
                                        .color(palette().value),
                                );
                            },
                        );
                    }
                });
                ui.add_space(6.0);
                // Bande d'onglets en pilules : l'onglet actif est plein
                // (rouge + texte blanc), les autres restent des boutons
                // standards (survol crème). Aucun glyphe d'emoji.
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

        // ── Status bar (essentiel : batterie, AC, charge moyenne, CPU0) ──
        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame {
                fill: ctx.style().visuals.window_fill,
                inner_margin: egui::Margin::symmetric(14.0, 6.0),
                ..Default::default()
            })
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Batterie : capacité colorée selon les seuils (bas = critique)
                    if let Some(cap) = view.num("battery", "Capacity") {
                        let color = palette().threshold(100.0 - cap, BATTERY_WARN, BATTERY_CRIT);
                        ui.label(
                            egui::RichText::new(format!("Battery {cap:.0}%"))
                                .monospace()
                                .strong()
                                .color(color),
                        );
                        ui.separator();
                    }
                    if let Some(ac) = view.num("battery", "AC Online") {
                        if ac > 0.0 {
                            ui.colored_label(palette().ok, "AC connected");
                        } else {
                            ui.colored_label(palette().warn, "On battery");
                        }
                        ui.separator();
                    }
                    // Charge moyenne des cœurs visibles (dérivée des CPUx Load).
                    let loads: Vec<f64> = (0..8)
                        .filter_map(|c| view.num("cpu", &format!("CPU{c} Load")))
                        .collect();
                    if !loads.is_empty() {
                        let avg = loads.iter().sum::<f64>() / loads.len() as f64;
                        let color =
                            palette().threshold(avg, super::components::LOAD_WARN, super::components::LOAD_CRIT);
                        ui.label(
                            egui::RichText::new(format!("Load avg {avg:.0}%"))
                                .monospace()
                                .strong()
                                .color(color),
                        );
                        ui.separator();
                    }
                    if let Some(freq) = view.num("cpu", "CPU0 Freq") {
                        ui.label(
                            egui::RichText::new(format!("CPU0 {freq:.0} MHz"))
                                .monospace()
                                .color(palette().value),
                        );
                    }
                });
            });

        // ── Content ──
        let mut profile_ctrl = std::mem::take(&mut self.profile_tab);
        egui::CentralPanel::default()
            .frame(egui::Frame {
                // Fill explicite : un Frame sans fill est transparent et
                // laisse voir la couleur de clear (sombre) sous le panel.
                fill: ctx.style().visuals.panel_fill,
                inner_margin: egui::Margin::symmetric(14.0, 10.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                    Tab::Profiles => profiles::render_profiles(ui, &view, &mut profile_ctrl),
                    Tab::Cpu => cpu::render_cpu(ui, &view, &self.store, &self.governors),
                    Tab::Gpu => gpu::render_gpu(ui, &view, &self.store),
                    Tab::Memory => memory::render_memory(ui, &view, &self.store),
                    Tab::Battery => battery::render_battery(ui, &view),
                    Tab::Thermal => thermal::render_thermal(ui, &view),
                    Tab::Io => io::render_io(ui, &view, &self.store),
                    Tab::Network => network::render_network(ui, &view, &self.store),
                });
            });
        self.profile_tab = profile_ctrl;
    }
}

/// Pilule d'onglet : bouton monospace à liseré encre ; l'onglet actif est
/// rempli de rouge avec texte blanc — impossible à manquer, aucun glyphe
/// de remplacement possible (rendu 100 % bouton, pas de sélection egui).
fn tab_button(ui: &mut egui::Ui, active: &mut Tab, tab: Tab, label: &str) {
    let selected = *active == tab;
    let ink = egui::Color32::from_rgb(23, 23, 23);
    let text = egui::RichText::new(label)
        .monospace()
        .strong()
        .color(if selected {
            egui::Color32::WHITE
        } else {
            palette().value
        });
    let button = if selected {
        egui::Button::new(text)
            .fill(palette().accent)
            .stroke(egui::Stroke::new(1.5_f32, ink))
            .rounding(6.0)
            .min_size(egui::vec2(0.0, 26.0))
    } else {
        egui::Button::new(text)
            .stroke(egui::Stroke::new(1.5_f32, ink))
            .rounding(6.0)
            .min_size(egui::vec2(0.0, 26.0))
    };
    if ui.add(button).clicked() {
        *active = tab;
    }
}
