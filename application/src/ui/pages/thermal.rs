//! Page Thermal : zones thermiques du SoC + températures CPU.

use crate::data::UiState;
use crate::ui::widgets::{
    threshold_tone, Card, EmptyState, MetricRow, SectionTitle,
    ZONE_TEMP_CRIT, ZONE_TEMP_WARN,
};
use dioxus::prelude::*;

#[component]
pub fn Thermal(state: Signal<UiState>) -> Element {
    let s = state.read();
    let has_cpu_temps = s.cpu.package_temp.is_some()
        || s.cpu.core_temps.iter().any(|t| t.is_some());

    if s.thermal.is_empty() && !has_cpu_temps {
        return rsx! {
            EmptyState {
                title: "No thermal data",
                text: "No thermal zone is exposed in sysfs on this machine. \
                       On a Surface, zones appear as ACPI zones, NVMe sensors and the CPU package.",
            }
        };
    }

    rsx! {
        // ── Zones thermiques ──
        SectionTitle { title: "Thermal zones" }
        if s.thermal.is_empty() {
            div { class: "empty-text", "No ACPI/NVMe zone exposed — only CPU sensors are available." }
        } else {
            div { class: "grid g-zones",
                for (name, temp) in &s.thermal {
                    {let tone = threshold_tone(*temp, ZONE_TEMP_WARN, ZONE_TEMP_CRIT);
                    rsx! {
                        div { key: "{name}", class: "zone-card",
                            div { class: "zone-name", "{name}" }
                            div { class: "zone-temp text-{tone}",
                                "{temp:.1}"
                                span { class: "unit", " °C" }
                            }
                            div { class: "level-track", style: "margin-top:8px;",
                                div { class: "level-fill {tone}", style: {format!("width: {:.0}%", temp.clamp(3.0, 100.0))} }
                            }
                        }
                    }}
                }
            }
        }

        // ── Capteurs CPU ──
        if has_cpu_temps {
            SectionTitle { title: "CPU sensors" }
            Card { title: "Coretemp",
                MetricRow {
                    label: "Package",
                    value: opt_temp(s.cpu.package_temp),
                }
                for (i, t) in s.cpu.core_temps.iter().enumerate() {
                    MetricRow {
                        key: "ct{i}",
                        label: "Core {i}",
                        value: opt_temp(*t),
                    }
                }
                div { class: "card-note",
                    "Throttling starts around 100 °C on the i5-1035G4 — the fan curve usually engages long before."
                }
            }
        }
    }
}

fn opt_temp(v: Option<f64>) -> Option<String> {
    v.map(|t| format!("{t:.1} °C"))
}
