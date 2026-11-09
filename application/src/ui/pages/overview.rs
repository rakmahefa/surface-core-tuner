//! Page Overview : instantané de la machine — KPI, activité, ressources,
//! bascule rapide de profil.

use crate::data::UiState;
use crate::ui::widgets::{
    fmt_bytes, fmt_kibps, fmt_mhz, fmt_pct, opt_fmt, Card, Chart, Kpi, LevelBar, SectionTitle,
    BATTERY_WARN, CPU_TEMP_WARN, LOAD_CRIT, LOAD_WARN,
};
use dioxus::prelude::*;

#[component]
pub fn Overview(state: Signal<UiState>) -> Element {
    let s = state.read();

    let avg_load = s.cpu.avg_load();
    let load_tone = avg_load
        .map(|v| crate::ui::widgets::threshold_tone(v, LOAD_WARN, LOAD_CRIT).to_string());
    let cpu0_freq = s.cpu.cores.first().and_then(|c| c.freq);
    let temp_tone = s
        .cpu
        .package_temp
        .map(|t| crate::ui::widgets::threshold_tone(t, CPU_TEMP_WARN, crate::ui::widgets::CPU_TEMP_CRIT).to_string());
    let bat_tone = s
        .bat
        .capacity
        .map(|c| crate::ui::widgets::threshold_tone(100.0 - c, BATTERY_WARN, crate::ui::widgets::BATTERY_CRIT).to_string());

    rsx! {
        // ── KPI ──
        div { class: "grid g-kpi",
            Kpi {
                label: "CPU load",
                value: avg_load.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: opt_fmt(cpu0_freq, fmt_mhz),
                tone: load_tone,
            }
            Kpi {
                label: "GPU",
                value: s.gpu.busy.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: opt_fmt(s.gpu.cur_freq, fmt_mhz),
            }
            Kpi {
                label: "Memory",
                value: s.mem.used_pct.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: match (opt_fmt(s.mem.used, fmt_bytes), opt_fmt(s.mem.total, fmt_bytes)) {
                    (Some(u), Some(t)) => Some(format!("{u} / {t}")),
                    (Some(u), None) => Some(u),
                    _ => None,
                },
            }
            Kpi {
                label: "Battery",
                value: s.bat.capacity.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: s.bat.power.map(|p| format!("{:.1} W draw", p)),
                tone: bat_tone,
            }
            Kpi {
                label: "Package temp",
                value: s.cpu.package_temp.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "°C",
                sub: s.cpu.core_temps.iter().flatten().copied().fold(None::<f64>, |a, v| Some(a.map_or(v, |m| m.max(v)))).map(|m| format!("hottest core {m:.0} °C")),
                tone: temp_tone,
            }
        }

        // ── Activité ──
        SectionTitle { title: "Activity" }
        div { class: "grid g-2",
            Card {
                Chart {
                    title: "CPU load",
                    series: s.charts.cpu_load.clone(),
                    color: "var(--s-cpu)",
                    current: opt_fmt(s.charts.cpu_load.last(), fmt_pct),
                    cid: "ov-cpu",
                    fmt_y: |v| format!("{v:.0}%"),
                }
            }
            Card {
                Chart {
                    title: "GPU busy",
                    series: s.charts.gpu_busy.clone(),
                    color: "var(--s-gpu)",
                    current: opt_fmt(s.charts.gpu_busy.last(), fmt_pct),
                    cid: "ov-gpu",
                    fmt_y: |v| format!("{v:.0}%"),
                }
            }
        }

        // ── Ressources ──
        SectionTitle { title: "Resources" }
        div { class: "grid g-3",
            Card {
                Chart {
                    title: "RAM used",
                    series: s.charts.mem_used.clone(),
                    color: "var(--s-mem)",
                    current: opt_fmt(s.charts.mem_used.last(), fmt_pct),
                    cid: "ov-mem",
                    fmt_y: |v| format!("{v:.0}%"),
                }
            }
            Card {
                Chart {
                    title: "Disk read",
                    series: s.charts.io_read.clone(),
                    color: "var(--s-io)",
                    current: opt_fmt(s.charts.io_read.last(), fmt_bytes),
                    cid: "ov-io",
                    fmt_y: fmt_bytes,
                }
            }
            Card {
                Chart {
                    title: "Download",
                    series: s.charts.net_rx.clone(),
                    color: "var(--s-rx)",
                    current: s.charts.net_rx.last().map(|v| crate::ui::widgets::fmt_kibps(v)),
                    cid: "ov-rx",
                    fmt_y: fmt_kibps,
                }
            }
        }

        // ── Alimentation ──
        SectionTitle { title: "Power" }
        div { class: "grid g-2",
            QuickProfiles { state: state },
            Card { title: "Battery",
                if let Some(cap) = s.bat.capacity {
                    LevelBar {
                        name: "Charge",
                        value: cap,
                        warn: BATTERY_WARN,
                        crit: crate::ui::widgets::BATTERY_CRIT,
                        invert: true,
                    }
                    div { class: "card-note",
                        {match (s.bat.ac_online, s.bat.status.clone()) {
                            (Some(ac), _) if ac > 0.0 => "Connected to AC power.".to_string(),
                            (_, Some(st)) => format!("Status: {st}."),
                            _ => "Discharging.".to_string(),
                        }}
                    }
                } else {
                    div { class: "card-note",
                        "No battery detected on this machine (virtual machine, desktop, or removed battery)."
                    }
                }
            }
        }
    }
}

/// Bascule rapide entre les profils prédéfinis.
#[component]
fn QuickProfiles(state: Signal<UiState>) -> Element {
    let s = state.read();
    let all = crate::profiles::load_all();
    let (predefined, _) = crate::profiles::split(&all);
    let active = s.active_profile.clone();

    rsx! {
        Card { title: "Quick profile",
            div { class: "profile-list",
                for p in predefined {
                    {let is_active = active.as_deref() == Some(p.name.as_str());
                    let name = p.name.clone();
                    let desc = describe(&p);
                    rsx! {
                        div {
                            key: "{name}",
                            class: if is_active { "profile-card active" } else { "profile-card" },
                            onclick: move |_| {
                                let all = crate::profiles::load_all();
                                if let Some(profile) = crate::profiles::find(&all, &name) {
                                    let _ = profile.apply();
                                }
                            },
                            div {
                                div { class: "profile-name", "{name}" }
                                div { class: "profile-desc", "{desc}" }
                            }
                            div { class: "profile-actions",
                                if is_active {
                                    crate::ui::widgets::Badge { text: "ACTIVE", tone: "ok" }
                                }
                            }
                        }
                    }}
                }
            }
            div { class: "card-note", "Applied system-wide — fine-tune from the Power page." }
        }
    }
}

/// Description courte d'un profil prédéfini (fractions des limites HW).
fn describe(p: &crate::profiles::Profile) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(g) = &p.governor {
        parts.push(g.clone());
    }
    if let Some(r) = p.cpu_max {
        parts.push(format!("CPU cap {:.0}%", r * 100.0));
    }
    if let Some(r) = p.gpu_max {
        parts.push(format!("GPU cap {:.0}%", r * 100.0));
    }
    parts.join(" · ")
}
