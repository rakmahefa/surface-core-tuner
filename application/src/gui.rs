//! Lancement de l'interface graphique (Dioxus).
//!
//! - Natif : fenêtre desktop via `dioxus-desktop` (webview système).
//! - wasm : rendu navigateur (build `--target wasm32-unknown-unknown`),
//!   utilisé pour prévisualiser l'UI — les données sont alors synthétiques.

use crate::app::App;

#[cfg(not(target_family = "wasm"))]
pub fn run() {
    use dioxus::desktop::tao::dpi::LogicalSize;
    use dioxus::desktop::{muda, Config, WindowBuilder, WindowCloseBehaviour};

    // Icône applicative (barre des tâches) — même visuel que l'icône de
    // notification : carte graphite, carré dégradé bleu, « N » blanc.
    let icon: Option<dioxus::desktop::tao::window::Icon> =
        dioxus::desktop::icon_from_memory(include_bytes!("../assets/icon.png")).ok();

    let cfg = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("NyVoara — Power & Performance")
                .with_inner_size(LogicalSize::new(1180.0, 760.0))
                .with_min_inner_size(LogicalSize::new(940.0, 600.0))
                .with_resizable(true),
        )
        // Pas de barre de menus native (File/Edit/Window…) : le dashboard
        // possède sa propre navigation latérale, la fenêtre reste sobre.
        // Sans cela, dioxus-desktop installe son menu par défaut.
        .with_menu(None::<muda::Menu>)
        // Le bouton ✕ masque la fenêtre au lieu de quitter : l'application
        // continue de tourner en tâche de fond, réduite dans la barre de
        // notification (icône + menu contextuel — voir `ui::tray`).
        .with_close_behaviour(WindowCloseBehaviour::WindowHides)
        // Un clic gauche sur l'icône de notification restaure la fenêtre.
        .with_tray_icon_show_window_on_click(true)
        // Évite le flash blanc au chargement de la webview.
        .with_background_color((12, 14, 17, 255));

    let cfg = match icon {
        Some(icon) => cfg.with_icon(icon),
        None => cfg,
    };

    dioxus::LaunchBuilder::new()
        .with_cfg(cfg)
        .launch(App);
}

#[cfg(target_family = "wasm")]
pub fn run() {
    dioxus::launch(App);
}
