//! Page Storage : débits et totaux par disque, pression d'attente I/O.

use crate::data::UiState;
use crate::ui::widgets::{ fmt_split,
    fmt_bytes, opt_fmt, Card, Chart, Kpi, MetricRow, SectionTitle,
};
use crate::ui::icons::{ArrowDown, ArrowUp};
use dioxus::prelude::*;

#[component]
pub fn Storage(state: Signal<UiState>) -> Element {
    let s = state.read();

    let primary = s.io.devices.iter().find(|d| d.present);
    let read_bps = primary.and_then(|d| d.read_bps);
    let write_bps = primary.and_then(|d| d.write_bps);

    rsx! {
        // ── KPI ──
        div { class: "grid g-kpi",
            Kpi {
                label: "Disk read",
                value: match read_bps { Some(v) => fmt_split(v, |v| fmt_bytes(v) + "/s").0, None => "—".to_string() },
                unit: Some(match read_bps { Some(v) => fmt_split(v, |v| fmt_bytes(v) + "/s").1, None => "—".to_string() }),
                sub: primary.map(|d| d.name.clone()),
            }
            Kpi {
                label: "Disk write",
                value: match write_bps { Some(v) => fmt_split(v, |v| fmt_bytes(v) + "/s").0, None => "—".to_string() },
                unit: Some(match write_bps { Some(v) => fmt_split(v, |v| fmt_bytes(v) + "/s").1, None => "—".to_string() }),
                sub: None,
            }
            Kpi {
                label: "I/O pressure",
                value: opt_fmt(s.io.psi_some, |v| format!("{v:.2}")).unwrap_or_else(|| "—".to_string()),
                unit: "%",
                sub: Some("PSI some, avg10".to_string()),
            }
        }

        // ── Débits du disque principal ──
        SectionTitle { title: "Throughput" }
        Card {
            div { class: "rate-pair",
                div { class: "rate-box",
                    div { class: "rate-label", span { style: "display:inline-flex;vertical-align:-2px;margin-right:4px;", ArrowDown {} }, "READ" }
                    div { class: "rate-value", {opt_fmt(read_bps, |v| fmt_bytes(v) + "/s").unwrap_or_else(|| "—".to_string())} }
                }
                div { class: "rate-box",
                    div { class: "rate-label", span { style: "display:inline-flex;vertical-align:-2px;margin-right:4px;", ArrowUp {} }, "WRITE" }
                    div { class: "rate-value", {opt_fmt(write_bps, |v| fmt_bytes(v) + "/s").unwrap_or_else(|| "—".to_string())} }
                }
            }
        }

        // ── Une carte par disque ──
        SectionTitle { title: "Devices" }
        div { class: "grid g-2",
            for dev in &s.io.devices {
                Card {
                    key: "{dev.name}",
                    title: dev.name.clone(),
                    if dev.present {
                        MetricRow { label: "Read throughput", value: opt_fmt(dev.read_bps, |v| format!("{}/s", fmt_bytes(v))) }
                        MetricRow { label: "Write throughput", value: opt_fmt(dev.write_bps, |v| format!("{}/s", fmt_bytes(v))) }
                        MetricRow { label: "Read IOPS", value: opt_fmt(dev.read_iops, |v| format!("{v:.0}")) }
                        MetricRow { label: "Write IOPS", value: opt_fmt(dev.write_iops, |v| format!("{v:.0}")) }
                        MetricRow { label: "Total read", value: opt_fmt(dev.total_reads, fmt_bytes) }
                        MetricRow { label: "Total written", value: opt_fmt(dev.total_writes, fmt_bytes) }
                    } else {
                        div { class: "empty-text", "Device not present on this machine." }
                    }
                }
            }
        }

        // ── Graphe + pression ──
        SectionTitle { title: "History" }
        div { class: "grid g-2",
            Card {
                Chart {
                    title: "Disk read",
                    series: s.charts.io_read.clone(),
                    color: "var(--s-io)",
                    current: opt_fmt(s.charts.io_read.last(), fmt_bytes),
                    cid: "io-read",
                    fmt_y: fmt_bytes,
                }
            }
            Card { title: "Pressure stall information",
                MetricRow { label: "PSI some (avg10)", value: opt_fmt(s.io.psi_some, |v| format!("{v:.2} %")) }
                MetricRow { label: "PSI full (avg10)", value: opt_fmt(s.io.psi_full, |v| format!("{v:.2} %")) }
                div { class: "card-note",
                    "Share of time at least one task waited on storage."
                }
            }
        }
    }
}
