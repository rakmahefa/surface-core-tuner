//! Thèmes egui — identité visuelle du dashboard.
//!
//! Regroupe au même endroit tout ce qui touche au rendu :
//! - `Palette` : couleurs sémantiques (muted, valeurs, seuils ok/warn/crit),
//!   consommées par la GUI pour teinter texte, barres et fonds ;
//! - `Theme` : `Visuals` clair « rétro Game Boy » + `Style` tactile (Surface)
//!   + la palette ; s'applique au contexte egui via [`Theme::apply`] ;
//! - l'accès global [`theme`] (construit une fois, `static`), qui évite de
//!   faire transiter le thème par toutes les fonctions d'affichage.
//!
//! La charte est reprise du site « Gen1Recomp Mods » (pokemon-recomp.netlify
//! .app) : fond papier ligné, crème, encre noire, rouge Pokéball, vert LCD
//! Game Boy, bordures encre épaisses et ombres décalées « néo-rétro ».

use eframe::egui::{self, Color32, Rounding, Stroke};
use std::sync::OnceLock;

// ── Charte du site de référence ─────────────────

/// Encre principale (texte, bordures) — `#171717`.
const INK: Color32 = Color32::from_rgb(23, 23, 23);
/// Fond « papier » des panneaux — `#fffaf0`.
const PAPER: Color32 = Color32::from_rgb(255, 250, 240);
/// Crème des entêtes / cartes — `#f4e7c5`.
const CREAM: Color32 = Color32::from_rgb(244, 231, 197);
/// Rouge Pokéball — `#a6202f`.
const RED: Color32 = Color32::from_rgb(166, 32, 47);
/// Vert LCD clair (écran Game Boy) — `#afc98f`.
const LCD: Color32 = Color32::from_rgb(175, 201, 143);
/// Vert LCD sombre (texte lisible sur crème) — `#425d47`.
const LCD_DARK: Color32 = Color32::from_rgb(66, 93, 71);
/// Gris sable des zones secondaires — `#e7e3d8`.
const GRAY: Color32 = Color32::from_rgb(231, 227, 216);

/// Palette sémantique — seuls points de référence pour teinter le rendu.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Texte secondaire (notes, valeurs absentes).
    pub muted: Color32,
    /// Valeurs numériques / accent (rouge Poké).
    pub value: Color32,
    /// « Tout va bien » (vert LCD sombre).
    pub ok: Color32,
    /// Même teinte en plus clair — cadres actifs, mentions « actif ».
    pub ok_soft: Color32,
    /// Attention (ambre).
    pub warn: Color32,
    /// Critique (rouge).
    pub crit: Color32,
    /// Même teinte en plus clair — cadres d'erreur.
    pub crit_soft: Color32,
}

impl Palette {
    /// Couleur de seuil : [`ok`](Self::ok) sous `warn`, [`warn`](Self::warn)
    /// sous `crit`, [`crit`](Self::crit) sinon.
    pub fn threshold(&self, value: f64, warn: f64, crit: f64) -> Color32 {
        if value >= crit {
            self.crit
        } else if value >= warn {
            self.warn
        } else {
            self.ok
        }
    }

    /// Même teinte avec une opacité repassée — fonds translucides (lignes
    /// actives, feedback teinté). `alpha` est hors gamma (0-255).
    pub fn tint(&self, color: Color32, alpha: u8) -> Color32 {
        let [r, g, b, _] = color.to_array();
        Color32::from_rgba_unmultiplied(r, g, b, alpha)
    }
}

/// Thème complet : `Visuals` clair rétro, `Style` tactile et palette.
#[derive(Clone, Debug)]
pub struct Theme {
    pub visuals: egui::Visuals,
    pub style: egui::Style,
    pub palette: Palette,
}

impl Theme {
    /// Construit le thème « NyVoara » : clair rétro Game Boy — fond papier,
    /// accents rouge Poké, contrôles monospace à liseré encre, ombres
    /// décalées et grandes cibles tactiles (Surface).
    fn build_retro() -> Self {
        let palette = Palette {
            muted: Color32::from_rgb(122, 108, 79),
            value: RED,
            ok: LCD_DARK,
            ok_soft: Color32::from_rgb(77, 138, 80),
            warn: Color32::from_rgb(166, 106, 0),
            crit: RED,
            crit_soft: Color32::from_rgb(155, 45, 39),
        };

        let mut visuals = egui::Visuals::light();

        // Fonds : hiérarchie « papier → crème → extrême » du site.
        visuals.panel_fill = PAPER;
        visuals.window_fill = CREAM;
        visuals.extreme_bg_color = Color32::from_rgb(247, 235, 222);
        visuals.faint_bg_color = GRAY;
        visuals.code_bg_color = LCD;
        visuals.override_text_color = Some(INK);
        visuals.hyperlink_color = RED;
        visuals.warn_fg_color = palette.warn;
        visuals.error_fg_color = palette.crit;

        // Fenêtres / menus : coins doux, liseré encre, ombre décalée
        // « néo-rétro » (reprise du `7px 7px 0` du site).
        visuals.window_rounding = Rounding::same(10.0);
        visuals.menu_rounding = Rounding::same(6.0);
        visuals.window_stroke = Stroke::new(2.0_f32, INK);
        let offset_shadow = egui::Shadow {
            offset: egui::vec2(4.0, 4.0),
            blur: 2.0,
            spread: 0.0,
            color: INK,
        };
        visuals.window_shadow = offset_shadow;
        visuals.popup_shadow = offset_shadow;

        // Sélection : rouge Poké + texte blanc (onglet actif, textes).
        visuals.selection.bg_fill = RED;
        visuals.selection.stroke = Stroke::new(2.0_f32, Color32::WHITE);

        // Contrôles : coins doux, bordures encre nettes, survol rosé
        // (le site rosé les liens avec `rgba(166,32,47,.1)`).
        let rounding = Rounding::same(5.0);
        visuals.widgets.noninteractive.rounding = rounding;
        visuals.widgets.inactive.rounding = rounding;
        visuals.widgets.hovered.rounding = rounding;
        visuals.widgets.active.rounding = rounding;

        visuals.widgets.noninteractive.bg_fill = Color32::WHITE;
        visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(238, 233, 214);
        visuals.widgets.noninteractive.bg_stroke =
            Stroke::new(1.0_f32, Color32::from_rgb(214, 205, 172));
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, INK);

        visuals.widgets.inactive.bg_fill = Color32::WHITE;
        visuals.widgets.inactive.weak_bg_fill = PAPER;
        visuals.widgets.inactive.bg_stroke = Stroke::new(2.0_f32, INK);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, INK);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(252, 236, 231);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(246, 231, 226);
        visuals.widgets.hovered.bg_stroke = Stroke::new(2.0_f32, INK);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, INK);

        visuals.widgets.active.bg_fill = Color32::from_rgb(232, 220, 192);
        visuals.widgets.active.weak_bg_fill = Color32::from_rgb(224, 210, 180);
        visuals.widgets.active.bg_stroke = Stroke::new(2.0_f32, INK);
        visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, INK);

        visuals.slider_trailing_fill = true;

        // Style global : typographie « monospace » pour les titres et
        // boutons (équivalent du Courier New du site), espacement et cibles
        // tactiles élargis (Surface).
        let mut style = egui::Style::default();
        style.visuals = visuals.clone();
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(23.0, egui::FontFamily::Monospace),
        );
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(14.0, egui::FontFamily::Monospace),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(10.5, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        );
        style.spacing.item_spacing = egui::vec2(10.0, 6.0);
        style.spacing.button_padding = egui::vec2(6.0, 3.0);
        style.spacing.interact_size = egui::vec2(40.0, 24.0);
        style.spacing.slider_width = 140.0;

        Self {
            visuals,
            style,
            palette,
        }
    }

    /// Installe le thème sur un contexte egui (visuals + style).
    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.visuals.clone());
        ctx.set_style(self.style.clone());
    }

    /// Couleur de série temporelle pour les graphes — homogène entre les
    /// blocs de métriques (index stable par source). Teintes mid-tone,
    /// lisibles sur fond papier.
    pub fn series(&self, index: usize) -> Color32 {
        const SERIES: [Color32; 4] = [
            Color32::from_rgb(47, 109, 163), // CPU / charge — bleu
            Color32::from_rgb(74, 141, 90),  // GPU — vert
            Color32::from_rgb(190, 130, 33), // Mémoire — or
            Color32::from_rgb(166, 32, 47),  // Température — rouge Poké
        ];
        SERIES[index % SERIES.len()]
    }
}

/// Le thème rétro de l'application — construit une seule fois (lazy),
/// accessible depuis tout le module d'affichage.
pub fn theme() -> &'static Theme {
    static RETRO: OnceLock<Theme> = OnceLock::new();
    RETRO.get_or_init(Theme::build_retro)
}

/// Raccourci vers la palette du thème courant.
pub fn palette() -> &'static Palette {
    &theme().palette
}
