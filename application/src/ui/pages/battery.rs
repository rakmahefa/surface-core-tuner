//! Page Battery : charge, puissance instantanée, énergie et santé.

use crate::data::UiState;
use crate::ui::widgets::{
    fmt_w, opt_fmt, Badge, Card, EmptyState, Kpi, LevelBar, MetricRow, SectionTitle,
    BATTERY_CRIT, BATTERY_WARN,
};
use crate::ui::icons::Plug;
use dioxus::prelude::*;

#[component]
pub fn Battery(state: Signal<UiState>) -> Element {
    let s = state.read();

    if !s.bat.present {
        return rsx! {
            EmptyState {
                title: "No battery detected",
                text: "This machine exposes no battery in sysfs (virtual machine, desktop, \
                       or removed battery). Plug detection may still work.",
            }
        };
    }

    // Autonomie estimée : énergie restante / puissance instantanée.
    let remaining = match (s.bat.energy_now, s.bat.power) {
        (Some(e), Some(p)) if p > 0.5 => Some(e / p),
        _ => None,
    };
    let remaining_sub = remaining.map(|h| format!("≈ {h:.1} h at current draw"));

    rsx! {
        // ── KPI ──
        div { class: "grid g-kpi",
            Kpi {
                label: "Charge",
                value: s.bat.capacity.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: remaining_sub,
                tone: s.bat.capacity.map(|c| crate::ui::widgets::threshold_tone(
                    100.0 - c, BATTERY_WARN, BATTERY_CRIT,
                ).to_string()),
            }
            Kpi {
                label: "Power draw",
                value: opt_fmt(s.bat.power, |v| format!("{v:.1}")).unwrap_or_else(|| "—".to_string()),
                unit: "W",
                sub: s.bat.status.clone().map(|s| s.to_lowercase()),
            }
            Kpi {
                label: "Energy",
                value: opt_fmt(s.bat.energy_now, |v| format!("{v:.1}")).unwrap_or_else(|| "—".to_string()),
                unit: "Wh",
                sub: match (s.bat.energy_full, s.bat.energy_design) {
                    (Some(f), Some(d)) => Some(format!("full {f:.1} Wh · design {d:.1} Wh")),
                    (Some(f), None) => Some(format!("full {f:.1} Wh")),
                    _ => None,
                },
            }
            Kpi {
                label: "Wear",
                value: opt_fmt(s.bat.wear, |v| format!("{v:.1}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: opt_fmt(s.bat.cycles, |v| format!("{v:.0} cycles")),
            }
        }

        // ── Charge + état ──
        SectionTitle { title: "Status" }
        Card {
            if let Some(cap) = s.bat.capacity {
                LevelBar {
                    name: "Charge",
                    value: cap,
                    warn: BATTERY_WARN,
                    crit: BATTERY_CRIT,
                    invert: true,
                }
            }
            div { style: "display:flex;gap:8px;margin-top:6px;",
                if let Some(ac) = s.bat.ac_online {
                    if ac > 0.0 {
                        span { style: "display:inline-flex;align-items:center;gap:5px;",
                            Plug {}
                            Badge { text: "AC POWER", tone: "ok" }
                        }
                    } else {
                        Badge { text: "DISCHARGING", tone: "warn" }
                    }
                }
            }
        }

        // ── Groupes ──
        div { class: "grid g-2",
            Card { title: "Power",
                MetricRow { label: "Power draw", value: opt_fmt(s.bat.power, fmt_w) }
                MetricRow { label: "Voltage", value: opt_fmt(s.bat.voltage, |v| format!("{v:.2} V")) }
            }
            Card { title: "Health",
                MetricRow { label: "Wear", value: opt_fmt(s.bat.wear, |v| format!("{v:.1} %")) }
                MetricRow { label: "Cycle count", value: opt_fmt(s.bat.cycles, |v| format!("{v:.0}")) }
                div { class: "card-note",
                    "Wear compares the current full-charge capacity against the design capacity — it only grows."
                }
            }
        }
        div { class: "grid g-2",
            Card { title: "Energy",
                MetricRow { label: "Remaining", value: opt_fmt(s.bat.energy_now, |v| format!("{v:.2} Wh")) }
                MetricRow { label: "Full charge capacity", value: opt_fmt(s.bat.energy_full, |v| format!("{v:.2} Wh")) }
                MetricRow { label: "Design capacity", value: opt_fmt(s.bat.energy_design, |v| format!("{v:.2} Wh")) }
            }
            Card { title: "Identity",
                MetricRow { label: "Status", value: s.bat.status.clone() }
                MetricRow { label: "Manufacturer", value: s.bat.manufacturer.clone() }
                MetricRow { label: "Technology", value: s.bat.technology.clone() }
                MetricRow { label: "AC online", value: s.bat.ac_online.map(|v| if v > 0.0 { "yes".to_string() } else { "no".to_string() }) }
            }
        }
    }
}
