//! Page CPU : charge par cœur, gouverneur, plafonds de fréquence,
//! températures et pression. Contrôle avancé par cœur en bloc replié.

use crate::data::UiState;
use crate::ui::widgets::{
    fmt_pct, opt_fmt, Card, Chart, FreqSlider, Kpi, MetricRow, RangeWarning, SectionTitle,
    Segmented, CPU_TEMP_CRIT, CPU_TEMP_WARN, LOAD_CRIT, LOAD_WARN,
};
use dioxus::prelude::*;

#[component]
pub fn Cpu(state: Signal<UiState>) -> Element {
    let s = state.read();
    let n = s.cpu.cores.len();

    let avg_load = s.cpu.avg_load();
    let hw = match (s.cpu.hw_min, s.cpu.hw_max) {
        (Some(lo), Some(hi)) if hi > lo => Some((lo, hi)),
        _ => None,
    };
    let governor = s.cpu.cores.first().and_then(|c| c.governor.clone()).unwrap_or_default();
    let min_max_inverted = matches!(
        (s.cpu.cores.first().and_then(|c| c.min), s.cpu.cores.first().and_then(|c| c.max)),
        (Some(min), Some(max)) if min > max
    );

    rsx! {
        // ── KPI ──
        div { class: "grid g-kpi",
            Kpi {
                label: "Average load",
                value: avg_load.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: None,
                tone: avg_load.map(|v| crate::ui::widgets::threshold_tone(v, LOAD_WARN, LOAD_CRIT).to_string()),
            }
            Kpi {
                label: "Package temp",
                value: s.cpu.package_temp.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "°C",
                sub: None,
                tone: s.cpu.package_temp.map(|t| crate::ui::widgets::threshold_tone(t, CPU_TEMP_WARN, CPU_TEMP_CRIT).to_string()),
            }
            Kpi {
                label: "CPU0 frequency",
                value: opt_fmt(s.cpu.cores.first().and_then(|c| c.freq), |v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "MHz",
                sub: hw.map(|(lo, hi)| format!("HW {lo:.0} – {hi:.0} MHz")),
            }
            Kpi {
                label: "Governor",
                value: if governor.is_empty() { "—" } else { governor.clone() },
                unit: None,
                sub: None,
            }
            Kpi {
                label: "Memory pressure",
                value: opt_fmt(s.cpu.psi_some, |v| format!("{v:.2}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: Some("PSI some, avg10".to_string()),
            }
        }

        // ── Contrôles + cœurs ──
        div { class: "grid g-2",
            Card { title: "Control",
                div { class: "metric-row",
                    span { class: "metric-label", "Scaling governor" }
                    if !s.cpu.governors.is_empty() && !governor.is_empty() {
                        Segmented {
                            options: s.cpu.governors.clone(),
                            active: governor,
                            on_select: move |g: String| { let _ = crate::profiles::set_cpu_governor(&g); },
                        }
                    } else {
                        span { class: "metric-value nodata", "unavailable" }
                    }
                }
                div { class: "level-sep" }
                if let Some((lo, hi)) = hw {
                    FreqSlider {
                        name: "Frequency minimum",
                        lo: lo,
                        hi: hi,
                        value: s.cpu.cores.first().and_then(|c| c.min),
                        on_apply: move |v| { let _ = crate::profiles::set_cpu_min_mhz(v); },
                    }
                    FreqSlider {
                        name: "Frequency maximum",
                        lo: lo,
                        hi: hi,
                        value: s.cpu.cores.first().and_then(|c| c.max),
                        on_apply: move |v| { let _ = crate::profiles::set_cpu_max_mhz(v); },
                    }
                    RangeWarning { show: min_max_inverted }
                    div { class: "card-note",
                        "Writes scaling_min_freq / scaling_max_freq on every core — applied when you release the slider. \
                         Hardware range {lo:.0}–{hi:.0} MHz."
                    }
                } else {
                    div { class: "card-note",
                        "Hardware frequency limits are not exposed on this machine — slider controls disabled."
                    }
                }
            }
            Card { title: "Per-core activity",
                if n == 0 {
                    div { class: "card-note", "No per-core readings on this machine." }
                } else {
                    div { class: "cores",
                        for (i, c) in s.cpu.cores.iter().enumerate() {
                            {let tone = crate::ui::widgets::threshold_tone(
                                c.load.unwrap_or(0.0), LOAD_WARN, LOAD_CRIT,
                            );
                            let val = c.load.map(|l| format!("{l:.0}%")).unwrap_or_else(|| "—".to_string());
                            let freq = c.freq.map(|f| format!("{f:.0} MHz")).unwrap_or_else(|| "—".to_string());
                            let width = format!("width: {:.1}%", c.load.unwrap_or(0.0).clamp(0.0, 100.0));
                            rsx! {
                            div { key: "core{i}", class: "core-cell",
                                div { class: "core-head",
                                    span { class: "core-id", "CPU{i}" }
                                    span { class: "core-load text-{tone}", "{val}" }
                                }
                                div { class: "core-freq", "{freq}" }
                                div { class: "core-bar level-track",
                                    div { class: "level-fill {tone}", style: "{width}" }
                                }
                            }
                        }}
                    }
                }
            }
                div { class: "card-note",
                    "Load thresholds: green below 50%, amber below 80%, red at 80% and above."
                }
            }
        }

        // ── Graphes ──
        SectionTitle { title: "History" }
        div { class: "grid g-2",
            Card {
                Chart {
                    title: "CPU0 load",
                    series: s.charts.cpu_load.clone(),
                    color: "var(--s-cpu)",
                    current: opt_fmt(s.charts.cpu_load.last(), fmt_pct),
                    cid: "cpu-load",
                    fmt_y: |v| format!("{v:.0}%"),
                }
            }
            Card {
                Chart {
                    title: "Package temperature",
                    series: s.charts.cpu_temp.clone(),
                    color: "var(--s-temp)",
                    current: opt_fmt(s.charts.cpu_temp.last(), |v| format!("{v:.1} °C")),
                    cid: "cpu-temp",
                    fmt_y: |v| format!("{v:.0} °C"),
                }
            }
        }

        // ── Températures + pression ──
        div { class: "grid g-2",
            Card { title: "Temperatures",
                MetricRow {
                    label: "Package",
                    value: opt_fmt(s.cpu.package_temp, |v| format!("{v:.1} °C")),
                }
                for (i, t) in s.cpu.core_temps.iter().enumerate() {
                    MetricRow {
                        key: "coretemp{i}",
                        label: "Core {i}",
                        value: opt_fmt(*t, |v| format!("{v:.1} °C")),
                    }
                }
            }
            Card { title: "Pressure stall information",
                MetricRow { label: "PSI some (avg10)", value: opt_fmt(s.cpu.psi_some, |v| format!("{v:.2} %")) }
                MetricRow { label: "PSI full (avg10)", value: opt_fmt(s.cpu.psi_full, |v| format!("{v:.2} %")) }
                div { class: "card-note",
                    "Share of time stalled waiting for CPU — some = at least one task, full = all non-idle tasks."
                }
            }
        }

        // ── Contrôle avancé par cœur ──
        details { class: "advanced",
            summary { "Per-core control (advanced)" }
            div { class: "card-note",
                "Set governor and frequency bounds individually. The governor may override these — check the readings after applying."
            }
            if let Some((lo, hi)) = hw {
                for (i, c) in s.cpu.cores.iter().enumerate() {
                    div { key: "adv{i}", class: "card", style: "margin-bottom: 10px;",
                        div { class: "card-title", "CPU{i}" }
                        if !s.cpu.governors.is_empty() {
                            if let Some(gov) = &c.governor {
                                Segmented {
                                    options: s.cpu.governors.clone(),
                                    active: gov.clone(),
                                    on_select: move |g: String| { let _ = crate::profiles::set_cpu_governor_core(i, &g); },
                                }
                            }
                        }
                        FreqSlider {
                            name: "Minimum",
                            lo: lo, hi: hi,
                            value: c.min,
                            on_apply: move |v| { let _ = crate::profiles::set_cpu_min_mhz_core(i, v); },
                        }
                        FreqSlider {
                            name: "Maximum",
                            lo: lo, hi: hi,
                            value: c.max,
                            on_apply: move |v| { let _ = crate::profiles::set_cpu_max_mhz_core(i, v); },
                        }
                    }
                }
            } else {
                div { class: "card-note", "Hardware limits unavailable." }
            }
        }
    }
}
