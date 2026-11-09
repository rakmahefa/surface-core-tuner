//! Page Memory : RAM, swap, pression mémoire et historique d'utilisation.

use crate::data::UiState;
use crate::ui::widgets::{ fmt_split,
    fmt_bytes, fmt_pct, opt_fmt, Card, Chart, Kpi, MetricRow, SectionTitle, LOAD_CRIT, LOAD_WARN,
};
use dioxus::prelude::*;

#[component]
pub fn Memory(state: Signal<UiState>) -> Element {
    let s = state.read();

    let used_pct = s.mem.used_pct;
    let swap_used_pct = match (s.mem.swap_used, s.mem.swap_total) {
        (Some(u), Some(t)) if t > 0.0 => Some(u / t * 100.0),
        _ => None,
    };

    rsx! {
        // ── KPI ──
        div { class: "grid g-kpi",
            Kpi {
                label: "RAM used",
                value: used_pct.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: match (opt_fmt(s.mem.used, fmt_bytes), opt_fmt(s.mem.total, fmt_bytes)) {
                    (Some(u), Some(t)) => Some(format!("{u} / {t}")),
                    (Some(u), None) => Some(u),
                    _ => None,
                },
                tone: used_pct.map(|v| crate::ui::widgets::threshold_tone(v, LOAD_WARN, LOAD_CRIT).to_string()),
            }
            Kpi {
                label: "Swap used",
                value: swap_used_pct.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: opt_fmt(s.mem.swap_used, fmt_bytes),
            }
            Kpi {
                label: "Available",
                value: match s.mem.available { Some(v) => fmt_split(v, fmt_bytes).0, None => "—".to_string() },
                unit: Some(match s.mem.available { Some(v) => fmt_split(v, fmt_bytes).1, None => "—".to_string() }),
                sub: Some("for new workloads without swapping".to_string()),
            }
            Kpi {
                label: "Memory pressure",
                value: opt_fmt(s.mem.psi_some, |v| format!("{v:.2}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: Some("PSI some, avg10".to_string()),
            }
        }

        // ── RAM + Swap ──
        div { class: "grid g-2",
            Card { title: "RAM",
                MetricRow { label: "Total", value: opt_fmt(s.mem.total, fmt_bytes) }
                MetricRow { label: "Used", value: opt_fmt(s.mem.used, fmt_bytes) }
                MetricRow { label: "Available", value: opt_fmt(s.mem.available, fmt_bytes) }
                MetricRow { label: "Free", value: opt_fmt(s.mem.free, fmt_bytes) }
                MetricRow { label: "Buffers", value: opt_fmt(s.mem.buffers, fmt_bytes) }
                MetricRow { label: "Cached", value: opt_fmt(s.mem.cached, fmt_bytes) }
            }
            Card { title: "Swap",
                MetricRow { label: "Total", value: opt_fmt(s.mem.swap_total, fmt_bytes) }
                MetricRow { label: "Used", value: opt_fmt(s.mem.swap_used, fmt_bytes) }
                MetricRow { label: "Free", value: opt_fmt(s.mem.swap_free, fmt_bytes) }
                MetricRow { label: "Cached", value: opt_fmt(s.mem.swap_cached, fmt_bytes) }
                div { class: "card-note",
                    "Swap on a Surface SSD is cheap but adds wear — keep an eye on the used ratio."
                }
            }
        }

        // ── Graphe ──
        SectionTitle { title: "History" }
        Card {
            Chart {
                title: "RAM used",
                series: s.charts.mem_used.clone(),
                color: "var(--s-mem)",
                current: opt_fmt(s.charts.mem_used.last(), fmt_pct),
                cid: "mem-used",
                fmt_y: |v| format!("{v:.0}%"),
            }
        }

        // ── Pression ──
        SectionTitle { title: "Pressure stall information" }
        Card {
            MetricRow { label: "PSI some (avg10)", value: opt_fmt(s.mem.psi_some, |v| format!("{v:.2} %")) }
            MetricRow { label: "PSI full (avg10)", value: opt_fmt(s.mem.psi_full, |v| format!("{v:.2} %")) }
            div { class: "card-note",
                "Share of time the system stalled for memory — sustained values above a few percent mean real pressure."
            }
        }
    }
}
