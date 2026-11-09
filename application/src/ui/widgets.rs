#![allow(unpredictable_function_pointer_comparisons)] // comparaison de fn ptr générée par #[component] (props PartialEq)

//! Briques d'interface partagées par toutes les pages.
//!
//! Style piloté par `assets/styles.css` (classes stables, aucune valeur de
//! présentation dans le rsx). Les seuils de code couleur sont centralisés
//! ici — accordables en un point unique.

use crate::core::Unit;
use crate::data::Series;
use dioxus::prelude::*;

// ── Seuils (identiques à l'ancienne GUI egui) ──
pub const LOAD_WARN: f64 = 50.0; // % charge / busy / mémoire
pub const LOAD_CRIT: f64 = 80.0;
pub const CPU_TEMP_WARN: f64 = 70.0; // °C package / cœurs
pub const CPU_TEMP_CRIT: f64 = 85.0;
pub const ZONE_TEMP_WARN: f64 = 60.0; // °C zones thermiques
pub const ZONE_TEMP_CRIT: f64 = 80.0;
/// Batterie : seuils appliqués à « 100 − capacité ».
pub const BATTERY_WARN: f64 = 50.0;
pub const BATTERY_CRIT: f64 = 80.0;

// ── Helpers de formatage ─────────────────────

/// Normalise « nombre UNITÉ » : insère une espace si le formatage du noyau
/// (Unit::fmt) colle l'unité au nombre (ex. « 400.0MHz » → « 400.0 MHz »).
fn spaced(s: String) -> String {
    if let Some(i) = s.find(|c: char| c.is_alphabetic()) {
        if i > 0 && !s[..i].ends_with(' ') {
            let mut out = String::with_capacity(s.len() + 1);
            out.push_str(&s[..i]);
            out.push(' ');
            out.push_str(&s[i..]);
            return out;
        }
    }
    s
}

/// Scinde « 2.3 MiB/s » en ("2.3", "MiB/s") pour les KPI (valeur en grand,
/// unité en petit).
pub fn fmt_split(v: f64, f: fn(f64) -> String) -> (String, String) {
    let s = f(v);
    match s.find(|c: char| c.is_alphabetic()) {
        Some(i) if i > 0 => (s[..i].trim().to_string(), s[i..].trim().to_string()),
        _ => (s, String::new()),
    }
}

pub fn fmt_mhz(v: f64) -> String {
    spaced(Unit::Mhz.fmt(v))
}
pub fn fmt_bytes(v: f64) -> String {
    spaced(Unit::Bytes.fmt(v))
}
pub fn fmt_kibps(v: f64) -> String {
    spaced(Unit::KiBps.fmt(v))
}
pub fn fmt_pct(v: f64) -> String {
    spaced(Unit::Percent.fmt(v))
}
pub fn fmt_temp(v: f64) -> String {
    format!("{v:.1} °C")
}
pub fn fmt_w(v: f64) -> String {
    spaced(Unit::W.fmt(v))
}
pub fn opt_fmt(v: Option<f64>, f: fn(f64) -> String) -> Option<String> {
    v.map(f)
}

/// Ton de seuil : `ok` sous `warn`, `warn` sous `crit`, `crit` sinon.
pub fn threshold_tone(value: f64, warn: f64, crit: f64) -> &'static str {
    if value >= crit {
        "crit"
    } else if value >= warn {
        "warn"
    } else {
        "ok"
    }
}

// ── Carte ─────────────────────────────────────

#[component]
pub fn Card(title: Option<String>, children: Element) -> Element {
    rsx! {
        section { class: "card",
            {title.map(|t| rsx! { div { class: "card-title", "{t}" } })}
            {children}
        }
    }
}

/// Titre de groupe à l'intérieur du contenu (hors carte).
#[component]
pub fn SectionTitle(title: String) -> Element {
    rsx! { div { class: "section-title", "{title}" } }
}

// ── KPI ───────────────────────────────────────

#[component]
pub fn Kpi(
    label: String,
    value: String,
    unit: Option<String>,
    sub: Option<String>,
    tone: Option<String>,
) -> Element {
    let tone_class = tone.map(|t| format!("text-{t}")).unwrap_or_default();
    // « % » reste collé au nombre ; les autres unités prennent une espace.
    let unit_label = unit.map(|u| if u == "%" { u } else { format!(" {u}") });
    rsx! {
        div { class: "card kpi",
            div { class: "kpi-label", "{label}" }
            div { class: "kpi-value {tone_class}",
                "{value}"
                {unit_label.map(|u| rsx! { span { class: "kpi-unit", "{u}" } })}
            }
            {sub.map(|s| rsx! { div { class: "kpi-sub", "{s}" } })}
        }
    }
}

// ── Lignes de métriques ───────────────────────

#[component]
pub fn MetricRow(label: String, value: Option<String>) -> Element {
    rsx! {
        div { class: "metric-row",
            span { class: "metric-label", "{label}" }
            {match value {
                Some(v) => rsx! { span { class: "metric-value", "{v}" } },
                None => rsx! { span { class: "metric-value nodata", "—" } },
            }}
        }
    }
}

#[component]
pub fn MetricList(children: Element) -> Element {
    rsx! { div { class: "metric-list", {children} } }
}

/// Construit une liste de lignes depuis des paires (libellé, valeur).
#[component]
pub fn MetricPairs(pairs: Vec<(String, Option<String>)>) -> Element {
    rsx! {
        div { class: "metric-list",
            for (label, value) in pairs {
                MetricRow { key: "{label}", label: label, value: value }
            }
        }
    }
}

// ── Badge ─────────────────────────────────────

#[component]
pub fn Badge(text: String, tone: String) -> Element {
    rsx! { span { class: "badge {tone}", "{text}" } }
}

// ── Barre de niveau ───────────────────────────

#[component]
pub fn LevelBar(name: String, value: f64, warn: f64, crit: f64, invert: bool) -> Element {
    let badness = if invert { 100.0 - value } else { value };
    let tone = threshold_tone(badness, warn, crit);
    let pct = value.clamp(0.0, 100.0);
    rsx! {
        div { class: "level",
            div { class: "level-head",
                span { class: "level-name", "{name}" }
                span { class: "level-value text-{tone}", "{value:.1}%" }
            }
            div { class: "level-track",
                div { class: "level-fill {tone}", style: "width: {pct:.1}%" }
            }
        }
    }
}

// ── État vide ─────────────────────────────────

#[component]
pub fn EmptyState(title: String, text: String) -> Element {
    rsx! {
        div { class: "empty",
            div { class: "empty-title", "{title}" }
            div { class: "empty-text", "{text}" }
        }
    }
}

// ── Avertissement min > max ───────────────────

#[component]
pub fn RangeWarning(show: bool) -> Element {
    rsx! {
        if show {
            div { class: "range-warning", "Minimum is above maximum — the kernel will clamp these values." }
        }
    }
}

// ── Graphe temporel (SVG) ─────────────────────

const CHART_W: f64 = 600.0;
const CHART_H: f64 = 132.0;

/// Points de la polyligne + chemin de l'aire sous la courbe.
fn chart_paths(points: &[f64], y_top: f64) -> (String, String) {
    let n = points.len();
    if n == 0 {
        return (String::new(), String::new());
    }
    let x = |i: usize| if n <= 1 { CHART_W } else { CHART_W * i as f64 / (n - 1) as f64 };
    let y = |v: f64| CHART_H - (v / y_top).clamp(0.0, 1.0) * CHART_H;

    let mut line = String::with_capacity(n * 12);
    for (i, v) in points.iter().enumerate() {
        if i > 0 {
            line.push(' ');
        }
        line.push_str(&format!("{:.1},{:.1}", x(i), y(*v)));
    }
    let mut area = format!("M0,{CHART_H:.1} L{line} L{CHART_W:.1},{CHART_H:.1} Z");
    if n == 1 {
        // Un seul point : aire dégénérée, on trace une ligne verticale fine.
        area = format!("M0,{CHART_H:.1} L{line} Z");
    }
    (line, area)
}

#[component]
pub fn Chart(
    title: String,
    series: Series,
    color: String,
    current: Option<String>,
    cid: String,
    fmt_y: fn(f64) -> String,
) -> Element {
    let points = &series.points;
    let y_top = series.y_top();
    let (line, area) = chart_paths(points, y_top);
    let mins = (series.window_secs / 60.0).round() as i64;
    let scale = format!("0–{}", fmt_y(y_top));
    rsx! {
        div {
            div { class: "chart-head",
                span { class: "chart-title", "{title}" }
                span { class: "chart-meta",
                    {current.as_ref().map(|c| rsx! {
                        span { class: "value", "{c}" }
                        " · "
                    })}
                    "last {mins} min · scale {scale}"
                }
            }
            if points.len() >= 2 {
                svg {
                    class: "chart",
                    view_box: "0 0 {CHART_W:.0} {CHART_H:.0}",
                    preserve_aspect_ratio: "none",
                    defs {
                        linearGradient {
                            id: "grad-{cid}",
                            x1: "0", y1: "0", x2: "0", y2: "1",
                            stop { offset: "0%", stop_color: "{color}", stop_opacity: "0.20" }
                            stop { offset: "100%", stop_color: "{color}", stop_opacity: "0.0" }
                        }
                    }
                    path { d: "M0,33 L600,33", stroke: "#1a1f26", stroke_width: "1" }
                    path { d: "M0,66 L600,66", stroke: "#1a1f26", stroke_width: "1" }
                    path { d: "M0,99 L600,99", stroke: "#1a1f26", stroke_width: "1" }
                    path { d: "{area}", fill: "url(#grad-{cid})", stroke: "none" }
                    polyline {
                        points: "{line}",
                        fill: "none",
                        stroke: "{color}",
                        stroke_width: "2",
                        vector_effect: "non-scaling-stroke",
                        stroke_linejoin: "round",
                    }
                }
            } else {
                div { class: "chart-empty", "collecting data…" }
            }
        }
    }
}

// ── Contrôle segmenté ─────────────────────────

#[component]
pub fn Segmented(options: Vec<String>, active: String, on_select: EventHandler<String>) -> Element {
    rsx! {
        div { class: "segmented",
            for opt in options {
                button {
                    key: "{opt}",
                    class: if opt == active { "active" } else { "" },
                    onclick: move |_| on_select.call(opt.clone()),
                    "{opt}"
                }
            }
        }
    }
}

// ── Slider de fréquence ───────────────────────

/// Slider min→max : la valeur suit l'état machine tant que l'utilisateur
/// n'interagit pas ; pendant le drag, un tampon local évite les sauts (les
/// ticks 5 Hz écraseraient le pouce). L'application sysfs a lieu au
/// `onchange` (relâchement) — jamais en rafale pendant le drag.
#[component]
pub fn FreqSlider(
    name: String,
    lo: f64,
    hi: f64,
    value: Option<f64>,
    on_apply: EventHandler<f64>,
) -> Element {
    let mut pending = use_signal(|| None::<f64>);
    let shown = pending().or(value).unwrap_or(lo).clamp(lo, hi);
    // Remplissage de la piste (avant le pouce) — variable CSS consommée par
    // le pseudo-élément track de la feuille de style.
    let fill = if hi > lo {
        format!("--fill: {:.0}%", (shown - lo) / (hi - lo) * 100.0)
    } else {
        "--fill: 0%".to_string()
    };
    rsx! {
        div { class: "slider-row",
            div { class: "slider-head",
                span { class: "slider-name", "{name}" }
                span { class: "slider-value", "{shown:.0} MHz" }
            }
            input {
                r#type: "range",
                min: "{lo:.0}",
                max: "{hi:.0}",
                step: "50",
                value: "{shown:.0}",
                style: "{fill}",
                oninput: move |e| pending.set(e.value().parse().ok()),
                onchange: move |e| {
                    if let Ok(v) = e.value().parse::<f64>() {
                        on_apply.call(v.round());
                    }
                    pending.set(None);
                },
            }
        }
    }
}
