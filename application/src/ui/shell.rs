//! Coquille applicative : barre latérale de navigation, entête de page,
//! zone de contenu et barre d'état.
//!
//! Navigation par signal — pas de routeur URL : l'application est une
//! fenêtre desktop, l'état de navigation vit dans un `Signal<Page>` fourni
//! par le composant racine.

use crate::data::UiState;
use crate::ui::{icons, pages, widgets::Badge, Page};
use dioxus::prelude::*;

#[component]
pub fn Shell(page: Signal<Page>, state: Signal<UiState>) -> Element {
    let current = page();
    rsx! {
        div { class: "app",
            Sidebar { page: page }
            div { class: "main",
                Topbar { page: current }
                div { class: "content",
                    div { class: "content-inner",
                        {match current {
                            Page::Overview => rsx! { pages::overview::Overview { state: state } },
                            Page::Power => rsx! { pages::profiles::Power { state: state } },
                            Page::Cpu => rsx! { pages::cpu::Cpu { state: state } },
                            Page::Gpu => rsx! { pages::gpu::Gpu { state: state } },
                            Page::Memory => rsx! { pages::memory::Memory { state: state } },
                            Page::Storage => rsx! { pages::storage::Storage { state: state } },
                            Page::Network => rsx! { pages::network::Network { state: state } },
                            Page::Battery => rsx! { pages::battery::Battery { state: state } },
                            Page::Thermal => rsx! { pages::thermal::Thermal { state: state } },
                        }}
                    }
                }
                Statusbar {}
            }
        }
    }
}

/// Icône de navigation associée à une page.
fn page_icon(p: Page) -> Element {
    match p {
        Page::Overview => rsx! { icons::Gauge {} },
        Page::Power => rsx! { icons::Zap {} },
        Page::Cpu => rsx! { icons::Cpu {} },
        Page::Gpu => rsx! { icons::Monitor {} },
        Page::Memory => rsx! { icons::Memory {} },
        Page::Storage => rsx! { icons::Drive {} },
        Page::Network => rsx! { icons::Wifi {} },
        Page::Battery => rsx! { icons::Battery {} },
        Page::Thermal => rsx! { icons::Thermometer {} },
    }
}

#[component]
fn Sidebar(page: Signal<Page>) -> Element {
    let groups: [(&str, &[Page]); 3] = [
        ("Dashboard", &[Page::Overview]),
        ("Control", &[Page::Power, Page::Cpu, Page::Gpu]),
        (
            "Monitor",
            &[
                Page::Memory,
                Page::Storage,
                Page::Network,
                Page::Battery,
                Page::Thermal,
            ],
        ),
    ];

    rsx! {
        aside { class: "sidebar",
            div { class: "brand",
                div { class: "brand-mark", "N" }
                div {
                    div { class: "brand-name", "NYVOARA" }
                    div { class: "brand-sub", "Power & Performance" }
                }
            }
            nav {
                for (group, entries) in groups {
                    div { key: "{group}", class: "nav-group", "{group}" }
                    for p in entries {
                        div {
                            key: "{p:?}",
                            class: if *p == page() { "nav-item active" } else { "nav-item" },
                            onclick: move |_| page.set(*p),
                            {page_icon(*p)}
                            span { "{p.title()}" }
                        }
                    }
                }
            }
            div { class: "sidebar-footer",
                span { span { class: "live-dot" }, "v0.1.0" }
                span { "5 Hz" }
            }
        }
    }
}

#[component]
fn Topbar(page: Page) -> Element {
    let state = use_context::<Signal<UiState>>();
    let s = state.read();

    // Profil actif à droite — repère permanent du réglage courant.
    let profile_chip = s.active_profile.as_ref().map(|p| {
        rsx! { Badge { text: p.clone(), tone: "accent".to_string() } }
    });

    // Puces batterie / secteur.
    let battery_chip = s.bat.capacity.map(|cap| {
        let tone = crate::ui::widgets::threshold_tone(
            100.0 - cap,
            crate::ui::widgets::BATTERY_WARN,
            crate::ui::widgets::BATTERY_CRIT,
        )
        .to_string();
        rsx! {
            Badge { text: format!("Battery {cap:.0}%"), tone: tone }
        }
    });
    let ac_chip = s.bat.ac_online.map(|ac| {
        if ac > 0.0 {
            rsx! { Badge { text: "AC power".to_string(), tone: "ok".to_string() } }
        } else {
            rsx! { Badge { text: "On battery".to_string(), tone: "warn".to_string() } }
        }
    });

    rsx! {
        header { class: "topbar",
            div {
                div { class: "page-title", "{page.title()}" }
                div { class: "page-sub", "{page.subtitle()}" }
            }
            div { class: "topbar-right",
                {battery_chip}
                {ac_chip}
                {profile_chip}
            }
        }
    }
}

#[component]
fn Statusbar() -> Element {
    let state = use_context::<Signal<UiState>>();
    let s = state.read();

    let source_label: &str = if cfg!(any(feature = "demo", target_family = "wasm")) {
        "demo data · 5 Hz"
    } else {
        "sysfs · 5 Hz"
    };

    rsx! {
        footer { class: "statusbar",
            if let Some(cap) = s.bat.capacity {
                {let tone = crate::ui::widgets::threshold_tone(
                    100.0 - cap,
                    crate::ui::widgets::BATTERY_WARN,
                    crate::ui::widgets::BATTERY_CRIT,
                );
                rsx! {
                    span { "Battery ", b { class: "text-{tone}", "{cap:.0}%" } }
                    span { class: "sep", "·" }
                }}
            }
            if let Some(avg) = s.cpu.avg_load() {
                {let tone = crate::ui::widgets::threshold_tone(
                    avg,
                    crate::ui::widgets::LOAD_WARN,
                    crate::ui::widgets::LOAD_CRIT,
                );
                rsx! {
                    span { "Load ", b { class: "text-{tone}", "{avg:.0}%" } }
                    span { class: "sep", "·" }
                }}
            }
            if let Some(freq) = s.cpu.cores.first().and_then(|c| c.freq) {
                span { "CPU0 ", b { "{freq:.0} MHz" } }
                span { class: "sep", "·" }
            }
            if let Some(t) = s.cpu.package_temp {
                {let tone = crate::ui::widgets::threshold_tone(
                    t,
                    crate::ui::widgets::CPU_TEMP_WARN,
                    crate::ui::widgets::CPU_TEMP_CRIT,
                );
                rsx! {
                    span { "Package ", b { class: "text-{tone}", "{t:.0} °C" } }
                    span { class: "sep", "·" }
                }}
            }
            span { "{source_label}" }
        }
    }
}
