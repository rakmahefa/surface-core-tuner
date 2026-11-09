//! Composant racine.
//!
//! Responsabilités :
//! - injecter la feuille de style (un seul `style` global) ;
//! - créer les signaux partagés : état machine [`UiState`], navigation,
//!   formulaire de la page Power ;
//! - lancer la boucle d'échantillonnage ([`crate::engine::Engine`]) comme
//!   tâche asynchrone : un tick toutes les 200 ms pousse un nouvel état dans
//!   le signal, qui re-rend uniquement les composants abonnés.

use crate::data::UiState;
use crate::engine::Engine;
use crate::ui::pages::profiles::ProfileForm;
use crate::ui::{shell::Shell, Page};
use dioxus::prelude::*;

#[allow(non_snake_case)]
pub fn App() -> Element {
    let state: Signal<UiState> = use_signal(UiState::default);
    let page: Signal<Page> = use_signal(|| Page::Overview);
    let form: Signal<ProfileForm> = use_signal(ProfileForm::default);

    // Contextes : pages et coquille lisent les signaux sans les faire
    // transiter par les props.
    provide_context(state);
    provide_context(page);
    provide_context(form);

    // ── Boucle de données ──
    // Le moteur vit dans un `use_ref` (léger, non cloné) ; la tâche async
    // tick toutes les 200 ms : lecture sysfs (quelques ms), snapshot, push
    // dans le signal — seuls les composants abonnés re-rendent.
    let mut engine = use_signal(Engine::new);
    let mut state_loop = state;
    use_future(move || async move {
        loop {
            let snapshot = engine.write().tick();
            state_loop.set(snapshot);
            tick_sleep().await;
        }
    });

    rsx! {
        style { {include_str!("../assets/styles.css")} }
        Shell { page: page, state: state }
    }
}

/// Attente asynchrone entre deux ticks — tokio en natif, gloo en wasm.
async fn tick_sleep() {
    #[cfg(target_family = "wasm")]
    {
        gloo_timers::future::TimeoutFuture::new(200).await;
    }
    #[cfg(not(target_family = "wasm"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}
