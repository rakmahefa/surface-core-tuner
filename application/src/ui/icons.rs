//! Icônes SVG inline — traits fins (style Lucide), 16 px.
//!
//! Dessinées à la main pour rester sobres et éviter toute dépendance :
//! `stroke: currentColor` hérite de la couleur du texte parent.

#![allow(non_snake_case)] // les composants doivent être capitalisés

use dioxus::prelude::*;

const S: &str = "0 0 24 24";

fn svg_shell(children: Element) -> Element {
    rsx! {
        svg {
            width: "16", height: "16", view_box: "{S}",
            fill: "none", stroke: "currentColor",
            stroke_width: "1.7", stroke_linecap: "round", stroke_linejoin: "round",
            {children}
        }
    }
}

/// Capteurs de vitesse — page Overview.
pub fn Gauge() -> Element {
    svg_shell(rsx! {
        path { d: "M3.5 12a8.5 8.5 0 1 1 17 0" }
        line { x1: "12", y1: "14", x2: "15.2", y2: "9.8" }
        circle { cx: "12", cy: "14", r: "1.1", fill: "currentColor", stroke: "none" }
    })
}

/// Éclair — profils de puissance.
pub fn Zap() -> Element {
    svg_shell(rsx! { path { d: "M13 2 3.5 13.5h7L9.5 22 21 10.5h-7L13 2z" } })
}

/// Puce CPU.
pub fn Cpu() -> Element {
    svg_shell(rsx! {
        rect { x: "5", y: "5", width: "14", height: "14", rx: "2" }
        rect { x: "9.5", y: "9.5", width: "5", height: "5", rx: "1" }
        path { d: "M9 2.5v2M15 2.5v2M9 19.5v2M15 19.5v2M2.5 9h2M2.5 15h2M19.5 9h2M19.5 15h2" }
    })
}

/// Écran — GPU.
pub fn Monitor() -> Element {
    svg_shell(rsx! {
        rect { x: "2.5", y: "4", width: "19", height: "12.5", rx: "2" }
        path { d: "M8.5 20.5h7M12 16.5v4" }
    })
}

/// Barrette mémoire.
pub fn Memory() -> Element {
    svg_shell(rsx! {
        path { d: "M3 15.5h18v-6a1 1 0 0 0-1-1h-1.5a1 1 0 0 1-1-1 1 1 0 0 0-1-1H7.5a1 1 0 0 0-1 1 1 1 0 0 1-1 1H4a1 1 0 0 0-1 1z" }
        path { d: "M7 13.5v-2M11 13.5v-2M15 13.5v-2M19 13.5v-2M6 15.5v3M18 15.5v3" }
    })
}

/// Disque — stockage.
pub fn Drive() -> Element {
    svg_shell(rsx! {
        path { d: "M4.5 5.5h15l2.5 6v5a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-5z" }
        path { d: "M2 11.5h20" }
        path { d: "M6.5 15.5h.01M10 15.5h.01" }
    })
}

/// WiFi — réseau.
pub fn Wifi() -> Element {
    svg_shell(rsx! {
        path { d: "M2.5 9.5a14.5 14.5 0 0 1 19 0" }
        path { d: "M6 13a9.5 9.5 0 0 1 12 0" }
        path { d: "M9.5 16.5a4.6 4.6 0 0 1 5 0" }
        path { d: "M12 20h.01" }
    })
}

/// Batterie.
pub fn Battery() -> Element {
    svg_shell(rsx! {
        rect { x: "2.5", y: "7.5", width: "16", height: "9", rx: "2" }
        path { d: "M21.5 11v2" }
        path { d: "M6.5 10.5v3M10.5 10.5v3" }
    })
}

/// Thermomètre.
pub fn Thermometer() -> Element {
    svg_shell(rsx! {
        path { d: "M14.5 4.5v9.3a4 4 0 1 1-5 0V4.5a2.5 2.5 0 0 1 5 0z" }
        line { x1: "12", y1: "9", x2: "12", y2: "14.5" }
    })
}

/// Prise secteur — alimentation externe.
pub fn Plug() -> Element {
    svg_shell(rsx! {
        path { d: "M9.5 7.5V3M14.5 7.5V3" }
        path { d: "M6.5 7.5h11V12a5.5 5.5 0 0 1-11 0z" }
        path { d: "M12 17.5v3.5" }
    })
}

/// Flèche montante (débit montant).
pub fn ArrowUp() -> Element {
    svg_shell(rsx! { path { d: "M12 19V5M6 11l6-6 6 6" } })
}

/// Flèche descendante (débit descendant).
pub fn ArrowDown() -> Element {
    svg_shell(rsx! { path { d: "M12 5v14M6 13l6 6 6-6" } })
}
