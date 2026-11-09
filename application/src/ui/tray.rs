//! Icône de notification (zone de tray) et menu contextuel.
//!
//! Desktop uniquement — le web n'a pas de barre de notification.
//!
//! Comportement mis en place :
//! - l'icône NyVoara vit dans la zone de notification dès le lancement ;
//! - clic gauche : restaure la fenêtre (comportement natif dioxus-desktop) ;
//! - clic droit : menu contextuel — Show / Minimize to tray / Quit ;
//! - le bouton ✕ de la fenêtre la masque (`WindowCloseBehaviour::WindowHides`
//!   configuré dans [`crate::gui`]) : l'application continue de tourner en
//!   tâche de fond, l'échantillonnage et l'infobulle restent actifs ;
//! - l'infobulle de l'icône reflète l'état live (batterie, charge CPU).
//!
//! Notes plateformes :
//! - Linux : l'appindicator n'a pas d'infobulle — `set_tooltip` est un no-op,
//!   le menu s'ouvre au clic droit ;
//! - Windows/macOS : infobulle + menu au clic droit.

use crate::data::UiState;
use dioxus::desktop::trayicon::{self, init_tray_icon, use_tray_icon};
use dioxus::desktop::{
    icon_from_memory, use_tray_menu_event_handler, window, WindowCloseBehaviour,
};
use dioxus::prelude::*;

/// Identifiants des entrées du menu contextuel.
mod ids {
    pub const SHOW: &str = "nyvoara-show";
    pub const HIDE: &str = "nyvoara-hide";
    pub const QUIT: &str = "nyvoara-quit";
}

/// Met en place l'icône de notification, son menu contextuel et les
/// gestionnaires d'événements associés.
///
/// Hook à appeler une seule fois depuis le composant racine, après le
/// `provide_context` de l'état machine — l'ordre des hooks garantit que
/// l'icône existe avant sa première lecture.
pub fn use_tray() {
    // 1. Création de l'icône et de son menu — fournie via le contexte
    //    dioxus (`use_tray_icon` la relit juste après).
    use_hook(|| {
        init_tray_icon(build_menu(), tray_icon());
    });

    // 2. Menu contextuel : afficher / réduire / quitter.
    use_tray_menu_event_handler(|event| match event.id().0.as_str() {
        ids::SHOW => {
            let w = window();
            w.set_visible(true);
            w.set_focus();
        }
        ids::HIDE => {
            window().set_visible(false);
        }
        ids::QUIT => {
            // Quitter proprement : par défaut, ✕ ne fait que masquer la
            // fenêtre — on rétablit une vraie fermeture avant de close(),
            // ce qui termine l'application (`exit_on_last_window_close`).
            let w = window();
            w.set_close_behavior(WindowCloseBehaviour::WindowCloses);
            w.close();
        }
        _ => {}
    });

    // 3. Infobulle live : batterie + charge moyenne. Le texte n'est poussé
    //    au système que lorsqu'il change (le moteur tick à 5 Hz) ; le cache
    //    vit dans un hook non réactif pour ne pas re-déclencher l'effet.
    let tray = use_tray_icon();
    let state: Signal<UiState> = use_context();
    let last = use_hook(|| std::cell::RefCell::new(String::new()));
    use_effect(move || {
        let Some(tray) = tray.as_ref() else { return };
        let tooltip = status_text(&state.read());
        let mut last = last.borrow_mut();
        if *last != tooltip {
            _ = tray.set_tooltip(Some(tooltip.clone()));
            *last = tooltip;
        }
    });
}

/// Construit le menu contextuel de l'icône de notification.
fn build_menu() -> trayicon::DioxusTrayMenu {
    use dioxus::desktop::muda::{MenuItem, PredefinedMenuItem};

    let menu = trayicon::DioxusTrayMenu::new();
    let show = MenuItem::with_id(ids::SHOW, "Show NyVoara", true, None);
    let hide = MenuItem::with_id(ids::HIDE, "Minimize to tray", true, None);
    let quit = MenuItem::with_id(ids::QUIT, "Quit NyVoara", true, None);

    menu.append_items(&[&show, &hide, &PredefinedMenuItem::separator(), &quit])
        .expect("construction du menu tray");

    menu
}

/// Décode l'icône embarquée (thème Graphite, 128×128) pour le tray.
fn tray_icon() -> Option<trayicon::DioxusTrayIcon> {
    icon_from_memory(include_bytes!("../../assets/icon.png")).ok()
}

/// Texte d'infobulle : nom + métriques disponibles.
fn status_text(s: &UiState) -> String {
    let mut parts: Vec<String> = vec!["NyVoara".into()];
    if let Some(cap) = s.bat.capacity {
        parts.push(format!("Battery {cap:.0}%"));
    }
    if let Some(load) = s.cpu.avg_load() {
        parts.push(format!("Load {load:.0}%"));
    }
    parts.join(" · ")
}
