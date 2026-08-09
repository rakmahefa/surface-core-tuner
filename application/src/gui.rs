//! Application graphique — egui dashboard.
//!
//! Ce module ne contient que la coquille applicative : fenêtre eframe,
//! thème, panneaux (entête avec onglets, barre de statut, contenu) et le
//! routage vers les onglets. Le rendu de chaque onglet et les helpers
//! d'affichage partagés vivent dans [`components`].
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
            // Thème global : rétro clair « Game Boy » + cibles tactiles élargies (Surface).
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
                // Crème type entête du site (le corps, lui, reste papier).
                fill: ctx.style().visuals.window_fill,
                inner_margin: egui::Margin::symmetric(10.0, 8.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("⚡ NYVOARA").color(palette().value));
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
                    let color = palette().threshold(100.0 - cap, BATTERY_WARN, BATTERY_CRIT);
                    ui.colored_label(color, format!("🔋 {cap:.0}%"));
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
        let mut profile_ctrl = std::mem::take(&mut self.profile_tab);
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                Tab::Profiles => profiles::render_profiles(ui, &view, &mut profile_ctrl),
                Tab::Cpu => cpu::render_cpu(ui, &view, &self.store, &self.governors),
                Tab::Gpu => gpu::render_gpu(ui, &view, &self.store),
                Tab::Memory => memory::render_memory(ui, &view, &self.store),
                Tab::Battery => battery::render_battery(ui, &view),
                Tab::Thermal => thermal::render_thermal(ui, &view),
                Tab::Io => io::render_io(ui, &view),
                Tab::Network => network::render_network(ui, &view),
            });
        });
        self.profile_tab = profile_ctrl;
    }
}

fn tab_button(ui: &mut egui::Ui, active: &mut Tab, tab: Tab, label: &str) {
    let selected = *active == tab;
    // Majuscules façon « pixel-font » du site de référence.
    if ui
        .selectable_label(selected, label.to_uppercase())
        .clicked()
    {
        *active = tab;
    }
}
