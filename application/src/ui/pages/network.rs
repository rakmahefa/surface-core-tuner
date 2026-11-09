//! Page Network : débits instantanés, totaux transférés et température WiFi.

use crate::data::UiState;
use crate::ui::widgets::{ fmt_split,
    fmt_bytes, fmt_kibps, fmt_temp, opt_fmt, Card, Chart, Kpi, MetricRow, SectionTitle,
};
use crate::ui::icons::{ArrowDown, ArrowUp};
use dioxus::prelude::*;

#[component]
pub fn Network(state: Signal<UiState>) -> Element {
    let s = state.read();

    rsx! {
        // ── KPI ──
        div { class: "grid g-kpi",
            Kpi {
                label: "Download",
                value: match s.net.rx_rate { Some(v) => fmt_split(v, fmt_kibps).0, None => "—".to_string() },
                unit: Some(match s.net.rx_rate { Some(v) => fmt_split(v, fmt_kibps).1, None => "—".to_string() }),
                sub: None,
            }
            Kpi {
                label: "Upload",
                value: match s.net.tx_rate { Some(v) => fmt_split(v, fmt_kibps).0, None => "—".to_string() },
                unit: Some(match s.net.tx_rate { Some(v) => fmt_split(v, fmt_kibps).1, None => "—".to_string() }),
                sub: Some("outgoing traffic".to_string()),
            }
            Kpi {
                label: "Total downloaded",
                value: match s.net.rx_bytes { Some(v) => fmt_split(v, fmt_bytes).0, None => "—".to_string() },
                unit: Some(match s.net.rx_bytes { Some(v) => fmt_split(v, fmt_bytes).1, None => "—".to_string() }),
                sub: Some("since boot".to_string()),
            }
            Kpi {
                label: "WiFi temperature",
                value: opt_fmt(s.net.wifi_temp, |v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string()),
                unit: "°C",
                sub: Some("AX200/AX201 radio".to_string()),
            }
        }

        // ── Débits ──
        SectionTitle { title: "Throughput" }
        Card {
            div { class: "rate-pair",
                div { class: "rate-box",
                    div { class: "rate-label", span { style: "display:inline-flex;vertical-align:-2px;margin-right:4px;", ArrowDown {} }, "DOWNLOAD" }
                    div { class: "rate-value", {opt_fmt(s.net.rx_rate, fmt_kibps).unwrap_or_else(|| "—".to_string())} }
                }
                div { class: "rate-box",
                    div { class: "rate-label", span { style: "display:inline-flex;vertical-align:-2px;margin-right:4px;", ArrowUp {} }, "UPLOAD" }
                    div { class: "rate-value", {opt_fmt(s.net.tx_rate, fmt_kibps).unwrap_or_else(|| "—".to_string())} }
                }
            }
        }

        // ── Graphes RX / TX ──
        SectionTitle { title: "History" }
        div { class: "grid g-2",
            Card {
                Chart {
                    title: "Download rate",
                    series: s.charts.net_rx.clone(),
                    color: "var(--s-rx)",
                    current: opt_fmt(s.charts.net_rx.last(), fmt_kibps),
                    cid: "net-rx",
                    fmt_y: fmt_kibps,
                }
            }
            Card {
                Chart {
                    title: "Upload rate",
                    series: s.charts.net_tx.clone(),
                    color: "var(--s-tx)",
                    current: opt_fmt(s.charts.net_tx.last(), fmt_kibps),
                    cid: "net-tx",
                    fmt_y: fmt_kibps,
                }
            }
        }

        // ── Totaux ──
        SectionTitle { title: "Totals" }
        Card {
            MetricRow { label: "Total downloaded", value: opt_fmt(s.net.rx_bytes, fmt_bytes) }
            MetricRow { label: "Total uploaded", value: opt_fmt(s.net.tx_bytes, fmt_bytes) }
            MetricRow { label: "WiFi temperature", value: opt_fmt(s.net.wifi_temp, fmt_temp) }
        }
    }
}
