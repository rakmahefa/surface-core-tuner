//! Lancement de l'interface graphique (Dioxus).
//!
//! - Natif : fenêtre desktop via `dioxus-desktop` (webview système).
//! - wasm : rendu navigateur (build `--target wasm32-unknown-unknown`),
//!   utilisé pour prévisualiser l'UI — les données sont alors synthétiques.

use crate::app::App;

#[cfg(not(target_family = "wasm"))]
pub fn run() {
    use dioxus::desktop::{Config, WindowBuilder};
    use dioxus::desktop::tao::dpi::LogicalSize;

    let cfg = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("NyVoara — Power & Performance")
                .with_inner_size(LogicalSize::new(1180.0, 760.0))
                .with_min_inner_size(LogicalSize::new(940.0, 600.0))
                .with_resizable(true),
        )
        // Évite le flash blanc au chargement de la webview.
        .with_background_color((12, 14, 17, 255));

    dioxus::LaunchBuilder::new()
        .with_cfg(cfg)
        .launch(App);
}

#[cfg(target_family = "wasm")]
pub fn run() {
    dioxus::launch(App);
}
