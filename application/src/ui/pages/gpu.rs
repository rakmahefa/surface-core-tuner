//! Page GPU : fréquences, plafonds RP1/RP0, utilisation et RC6.

use crate::data::UiState;
use crate::ui::widgets::{
    fmt_mhz, fmt_pct, opt_fmt, Card, Chart, EmptyState, FreqSlider, Kpi, MetricRow, RangeWarning,
    SectionTitle, LOAD_CRIT, LOAD_WARN,
};
use dioxus::prelude::*;

#[component]
pub fn Gpu(state: Signal<UiState>) -> Element {
    let s = state.read();

    if !s.gpu.present {
        return rsx! {
            EmptyState {
                title: "No GPU metrics",
                text: "No Intel graphics directory was found in sysfs (/sys/class/drm/card*/gt_*). \
                       On a Surface this usually means the i915 driver is not loaded.",
            }
        };
    }

    let hw = match (s.gpu.rp1_min, s.gpu.rp0_max) {
        (Some(lo), Some(hi)) if hi > lo => Some((lo, hi)),
        _ => None,
    };
    let min_max_inverted = matches!(
        (s.gpu.min_freq, s.gpu.max_freq),
        (Some(min), Some(max)) if min > max
    );

    rsx! {
        // ── KPI ──
        div { class: "grid g-kpi",
            Kpi {
                label: "Busy",
                value: s.gpu.busy.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: None,
                tone: s.gpu.busy.map(|v| crate::ui::widgets::threshold_tone(v, LOAD_WARN, LOAD_CRIT).to_string()),
            }
            Kpi {
                label: "Current freq",
                value: opt_fmt(s.gpu.cur_freq, |v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "MHz",
                sub: opt_fmt(s.gpu.act_freq, |v| format!("actual {v:.0} MHz")),
            }
            Kpi {
                label: "RC6 residency",
                value: s.gpu.rc6.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: Some("time spent in the deepest idle state".to_string()),
            }
            Kpi {
                label: "RPS",
                value: opt_fmt(s.gpu.rps_cur, |v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "MHz",
                sub: Some("render power sparing".to_string()),
            }
        }

        // ── Contrôle + lectures ──
        div { class: "grid g-2",
            Card { title: "Frequency limits",
                if let Some((lo, hi)) = hw {
                    div { class: "card-note",
                        "RP1 is the minimum sustained clock, RP0 the maximum sustained clock. \
                         Commits on release — applies to the whole GPU."
                    }
                    FreqSlider {
                        name: "RP1 — minimum",
                        lo: lo, hi: hi,
                        value: s.gpu.min_freq,
                        on_apply: move |v| { let _ = crate::profiles::set_gpu_min_mhz(v); },
                    }
                    FreqSlider {
                        name: "RP0 — maximum",
                        lo: lo, hi: hi,
                        value: s.gpu.max_freq,
                        on_apply: move |v| { let _ = crate::profiles::set_gpu_max_mhz(v); },
                    }
                    RangeWarning { show: min_max_inverted }
                } else {
                    div { class: "card-note",
                        "RP0/RP1 hardware bounds are not exposed on this machine — controls disabled."
                    }
                }
            }
            Card { title: "Readings",
                MetricRow { label: "Requested frequency", value: opt_fmt(s.gpu.cur_freq, fmt_mhz) }
                MetricRow { label: "Actual frequency", value: opt_fmt(s.gpu.act_freq, fmt_mhz) }
                MetricRow { label: "Boost frequency", value: opt_fmt(s.gpu.boost, fmt_mhz) }
                MetricRow { label: "Busy", value: opt_fmt(s.gpu.busy, fmt_pct) }
                MetricRow { label: "RC6 residency", value: opt_fmt(s.gpu.rc6, fmt_pct) }
            }
        }

        // ── RPS ──
        SectionTitle { title: "Render power sparing" }
        Card {
            MetricRow { label: "RPS current", value: opt_fmt(s.gpu.rps_cur, fmt_mhz) }
            MetricRow { label: "RPS minimum", value: opt_fmt(s.gpu.rps_min, fmt_mhz) }
            MetricRow { label: "RPS maximum", value: opt_fmt(s.gpu.rps_max, fmt_mhz) }
            MetricRow { label: "RPS boost", value: opt_fmt(s.gpu.rps_boost, fmt_mhz) }
            div { class: "card-note",
                "The RPS floor/ceiling the driver enforces between RP1 and RP0."
            }
        }

        // ── Graphe ──
        SectionTitle { title: "History" }
        Card {
            Chart {
                title: "GPU busy",
                series: s.charts.gpu_busy.clone(),
                color: "var(--s-gpu)",
                current: opt_fmt(s.charts.gpu_busy.last(), fmt_pct),
                cid: "gpu-busy",
                fmt_y: |v| format!("{v:.0}%"),
            }
        }
    }
}
