//! Page Power : profils prédéfinis et personnalisés, application,
//! suppression et capture des réglages courants.
//!
//! L'état du formulaire (nom saisi, bandeau de retour) vit dans des signaux
//! fournis par le composant racine : il survit aux changements de page.

use crate::data::UiState;
use crate::ui::widgets::{Badge, Card, SectionTitle};
use crate::profiles::Profile;
use dioxus::prelude::*;

/// État local de la page Power, persistant entre les navigations.
#[derive(Clone, Default)]
pub struct ProfileForm {
    pub new_name: String,
    pub feedback: Option<(bool, String)>,
}

#[component]
pub fn Power(state: Signal<UiState>) -> Element {
    let mut form = use_context::<Signal<ProfileForm>>();
    let s = state.read();
    let all = crate::profiles::load_all();
    let (predefined, custom) = crate::profiles::split(&all);
    let active = s.active_profile.clone();
    let f = form.read();
    let feedback = f.feedback.clone();
    let new_name = f.new_name.clone();
    drop(f);

    rsx! {
        // ── Retour d'action ──
        if let Some((ok, msg)) = feedback {
            div { class: if ok { "feedback ok" } else { "feedback err" }, "{msg}" }
        }

        // ── Profils prédéfinis ──
        SectionTitle { title: "Predefined profiles" }
        div { class: "profile-list",
            for p in predefined {
                ProfileCard {
                    key: "{p.name}",
                    profile: p.clone(),
                    active: active.as_deref() == Some(p.name.as_str()),
                    custom: false,
                    state: state,
                    form: form,
                }
            }
        }

        // ── Profils personnalisés ──
        SectionTitle { title: "Custom profiles" }
        if custom.is_empty() {
            div { class: "profile-empty",
                "None yet — capture your current settings below to reuse them later."
            }
        }
        div { class: "profile-list",
            for p in custom {
                ProfileCard {
                    key: "{p.name}",
                    profile: p.clone(),
                    active: active.as_deref() == Some(p.name.as_str()),
                    custom: true,
                    state: state,
                    form: form,
                }
            }
        }

        // ── Capture des réglages courants ──
        SectionTitle { title: "Save current settings" }
        Card {
            div { class: "card-note",
                "Captures the current governor and frequency caps of every core (and GPU) as a reusable profile."
            }
            div { class: "save-form",
                input {
                    class: "field",
                    r#type: "text",
                    placeholder: "Profile name — e.g. Silent evening",
                    value: "{new_name}",
                    oninput: move |e| form.write().new_name = e.value(),
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| save(state, form),
                    "Save profile"
                }
            }
        }

        // ── Fonctionnement ──
        SectionTitle { title: "How profiles work" }
        Card {
            div { class: "card-note",
                "Predefined profiles cap CPU and GPU frequencies as a share of the hardware limits: \
                 Battery Saver holds the CPU at 30% and the GPU at 25% of their maximum, Balanced keeps \
                 everything at stock under the powersave governor, and Performance switches every core \
                 to the performance governor."
            }
            div { class: "card-note",
                "Applying a profile writes sysfs values immediately — no reboot needed. Custom profiles \
                 snapshot the exact per-core minimum and maximum frequencies, so they restore precisely \
                 what was captured, including mixed per-core setups. The ACTIVE badge follows the live \
                 readings: it moves when the current settings drift from a profile."
            }
        }
    }
}

#[component]
fn ProfileCard(
    profile: Profile,
    active: bool,
    custom: bool,
    state: Signal<UiState>,
    mut form: Signal<ProfileForm>,
) -> Element {
    let s = state.read();
    let description = describe_profile(&profile, &s.view);

    rsx! {
        div { class: if active { "profile-card active" } else { "profile-card" },
            div {
                div { class: "profile-name",
                    "{profile.name}"
                    if custom {
                        span { class: "profile-desc", " · custom" }
                    }
                }
                div { class: "profile-desc", "{description}" }
            }
            div { class: "profile-actions",
                if active {
                    Badge { text: "ACTIVE", tone: "ok" }
                }
                button {
                    class: if active { "btn btn-sm" } else { "btn btn-primary btn-sm" },
                    disabled: active,
                    onclick: {
                        let name = profile.name.clone();
                        move |_| {
                            let all = crate::profiles::load_all();
                            let feedback = match crate::profiles::find(&all, &name) {
                                Some(p) => match p.apply() {
                                    Ok(()) => (true, format!("Profile « {name} » applied.")),
                                    Err(e) => (false, format!("{name}: {e}")),
                                },
                                None => (false, format!("Profile « {name} » not found.")),
                            };
                            form.write().feedback = Some(feedback);
                        }
                    },
                    "Apply"
                }
                if custom {
                    button {
                        class: "btn btn-danger btn-sm",
                        onclick: {
                            let name = profile.name.clone();
                            move |_| {
                                let feedback = match crate::profiles::remove_custom(&name) {
                                    Ok(_) => (true, format!("Profile « {name} » deleted.")),
                                    Err(e) => (false, e),
                                };
                                form.write().feedback = Some(feedback);
                            }
                        },
                        "Delete"
                    }
                }
            }
        }
    }
}

/// Capture les réglages courants et persiste le profil.
fn save(state: Signal<UiState>, mut form: Signal<ProfileForm>) {
    let s = state.read();
    let name = form.read().new_name.trim().to_string();

    let feedback = if name.is_empty() {
        Some((false, "A name is required.".to_string()))
    } else if crate::profiles::predefined()
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case(&name))
    {
        Some((false, "Name already used by a predefined profile.".to_string()))
    } else {
        match Profile::from_store(&s.view, &name) {
            Some(p) => match crate::profiles::add_custom(&p) {
                Ok(_) => {
                    form.write().new_name.clear();
                    Some((true, format!("Profile « {name} » saved.")))
                }
                Err(e) => Some((false, e)),
            },
            None => Some((
                false,
                "Nothing to capture: no governor or frequency readings available.".to_string(),
            )),
        }
    };
    form.write().feedback = feedback;
}

/// Décrit un profil avec les valeurs cibles réelles (depuis les limites HW).
fn describe_profile(p: &Profile, view: &crate::core::View) -> String {
    use crate::core::ReadLatest;
    let mut parts: Vec<String> = Vec::new();
    if let Some(g) = &p.governor {
        parts.push(g.clone());
    } else if let Some(govs) = &p.core_governor {
        if govs.iter().all(|g| g == &govs[0]) {
            parts.push(govs[0].clone());
        }
    }
    if let Some(r) = p.cpu_max {
        match view.num("cpu", "HW Max Freq") {
            Some(hw) => parts.push(format!("CPU max {:.0} MHz", hw * r)),
            None => parts.push(format!("CPU max {:.0}%", r * 100.0)),
        }
    }
    if let Some(v) = &p.core_min {
        parts.push(range_text("CPU min", v));
    }
    if let Some(v) = &p.core_max {
        parts.push(range_text("CPU max", v));
    }
    if let Some(m) = p.gpu_min {
        parts.push(format!("GPU min {m:.0} MHz"));
    }
    if let Some(r) = p.gpu_max {
        match view.num("gpu", "RP0 Max") {
            Some(hw) => parts.push(format!("GPU max {:.0} MHz", hw * r)),
            None => parts.push(format!("GPU max {:.0}%", r * 100.0)),
        }
    }
    if parts.is_empty() {
        parts.push("no settings recorded".to_string());
    }
    parts.join(" · ")
}

/// Formate une plage de fréquences par cœur (valeur unique si uniforme).
fn range_text(prefix: &str, values: &[f64]) -> String {
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if lo == hi {
        format!("{prefix} {lo:.0} MHz")
    } else {
        format!("{prefix} {lo:.0}–{hi:.0} MHz")
    }
}
