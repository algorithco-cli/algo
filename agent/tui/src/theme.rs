//! Login visual system: one [`Theme`] with semantic slots, three tiers.
//!
//! Tiers: truecolor → ANSI-16 → mono. `NO_COLOR` (non-empty) forces mono.
//! `--color truecolor|ansi16|mono` overrides detection.
//!
//! Palette (background `#0B0D12`). Contrast ratios below are WCAG 2.1
//! relative-luminance ratios vs the background, enforced by
//! [`tests::truecolor_slots_meet_aa`]:
//!
//! | slot   | hex       | ratio |
//! |--------|-----------|-------|
//! | fg     | `#EDEEF2` | 16.8  |
//! | muted  | `#A7B0C2` | 8.9   |
//! | accent | `#A78BFA` | 7.1   |
//! | ok     | `#4ADE80` | 11.2  |
//! | warn   | `#FBBF24` | 11.6  |
//! | err    | `#F87171` | 7.0   |
//!
//! The previous tokens (`#6B6A7B` muted ≈ 2.6, `#6D4AFF` brand ≈ 3.2) failed
//! AA on near-black and are not used on the login screen anymore.
//! ANSI-16 uses bright variants (assumed dark terminal); mono uses no color
//! at all — hierarchy comes from bold/dim/underline + symbol + text.

use ratatui::style::{Color, Modifier, Style};

use crate::dog::ColorMode;

/// Semantic color slots for the login screen. No hex in widget code —
/// widgets ask the theme for a ready [`Style`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Which tier this theme was built for.
    pub layer: ColorMode,
    /// Base background.
    pub bg: Color,
    /// Default foreground.
    pub fg: Color,
    /// Secondary foreground (always ≥ 4.5:1 on `bg` in truecolor).
    pub muted: Color,
    /// Single accent (lighter violet, ≥ 4.5:1 on `bg` in truecolor).
    pub accent: Color,
    /// Focused-card border (= accent).
    pub border_focus: Color,
    /// Unfocused-card border (dimmed at the use site).
    pub border_idle: Color,
    /// Success.
    pub ok: Color,
    /// Warning / pending.
    pub warn: Color,
    /// Error.
    pub err: Color,
}

impl Theme {
    /// Build a theme for a color tier.
    pub fn new(layer: ColorMode) -> Self {
        match layer {
            ColorMode::TrueColor => Self {
                layer,
                bg: Color::Rgb(11, 13, 18),
                fg: Color::Rgb(237, 238, 242),
                muted: Color::Rgb(167, 176, 194),
                accent: Color::Rgb(167, 139, 250),
                border_focus: Color::Rgb(167, 139, 250),
                border_idle: Color::Rgb(167, 176, 194),
                ok: Color::Rgb(74, 222, 128),
                warn: Color::Rgb(251, 191, 36),
                err: Color::Rgb(248, 113, 113),
            },
            ColorMode::Ansi16 => Self {
                layer,
                bg: Color::Reset,
                fg: Color::White,
                muted: Color::Gray,
                accent: Color::LightMagenta,
                border_focus: Color::LightMagenta,
                border_idle: Color::Gray,
                ok: Color::LightGreen,
                warn: Color::Yellow,
                err: Color::LightRed,
            },
            ColorMode::Mono => Self {
                // No color: every slot is `Reset`; styles below add
                // bold/dim/underline instead. Never color alone.
                layer,
                bg: Color::Reset,
                fg: Color::Reset,
                muted: Color::Reset,
                accent: Color::Reset,
                border_focus: Color::Reset,
                border_idle: Color::Reset,
                ok: Color::Reset,
                err: Color::Reset,
                warn: Color::Reset,
            },
        }
    }

    /// `true` when no ANSI color may be emitted.
    pub fn is_mono(&self) -> bool {
        self.layer == ColorMode::Mono
    }

    fn paint(&self, slot: Color, mods: Modifier) -> Style {
        if self.is_mono() {
            Style::default().add_modifier(mods)
        } else {
            Style::default().fg(slot).add_modifier(mods)
        }
    }

    /// Default body text.
    pub fn fg(&self) -> Style {
        self.paint(self.fg, Modifier::empty())
    }

    /// Default body text, bold (titles, focused action rows).
    pub fn fg_bold(&self) -> Style {
        self.paint(self.fg, Modifier::BOLD)
    }

    /// Secondary text.
    pub fn muted(&self) -> Style {
        if self.is_mono() {
            Style::default().add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(self.muted)
        }
    }

    /// Focused-card border: the ONE accent element per focused card.
    pub fn border_focus(&self) -> Style {
        self.paint(self.border_focus, Modifier::empty())
    }

    /// Unfocused-card border: always dimmed.
    pub fn border_idle(&self) -> Style {
        if self.is_mono() {
            Style::default().add_modifier(Modifier::DIM)
        } else {
            Style::default()
                .fg(self.border_idle)
                .add_modifier(Modifier::DIM)
        }
    }

    /// Focused-card title.
    pub fn title_focus(&self) -> Style {
        self.paint(self.fg, Modifier::BOLD)
    }

    /// Unfocused-card title.
    pub fn title_idle(&self) -> Style {
        if self.is_mono() {
            Style::default().add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(self.muted)
        }
    }

    /// Status styles. Every status also carries a symbol + words.
    pub fn ok(&self) -> Style {
        self.paint(self.ok, Modifier::BOLD)
    }

    pub fn warn(&self) -> Style {
        self.paint(self.warn, Modifier::BOLD)
    }

    pub fn err(&self) -> Style {
        self.paint(self.err, Modifier::BOLD)
    }

    /// Accent text (spinner glyph). Used sparingly: at most one accent
    /// element besides the focused border per card.
    pub fn accent(&self) -> Style {
        self.paint(self.accent, Modifier::BOLD)
    }
}

/// WCAG 2.1 relative luminance for `Color::Rgb`, else `None`.
/// Used by the AA regression test; kept public for external audits.
#[allow(dead_code)]
pub fn luminance(color: Color) -> Option<f64> {
    let (r, g, b) = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => return None,
    };
    let f = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    Some(0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b))
}

/// WCAG 2.1 contrast ratio for two `Color::Rgb` colors, else `None`.
/// Used by the AA regression test; kept public for external audits.
#[allow(dead_code)]
pub fn contrast_ratio(a: Color, b: Color) -> Option<f64> {
    let (l1, l2) = (luminance(a)?, luminance(b)?);
    let (hi, lo) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    Some((hi + 0.05) / (lo + 0.05))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truecolor_slots_meet_aa() {
        let t = Theme::new(ColorMode::TrueColor);
        for (name, slot) in [
            ("fg", t.fg),
            ("muted", t.muted),
            ("accent", t.accent),
            ("border_focus", t.border_focus),
            ("border_idle", t.border_idle),
            ("ok", t.ok),
            ("warn", t.warn),
            ("err", t.err),
        ] {
            let ratio = contrast_ratio(t.bg, slot)
                .unwrap_or_else(|| panic!("{name} tier must be Rgb for ratio math"));
            assert!(
                ratio >= 4.5,
                "{name} contrast {ratio:.2}:1 on bg is below AA 4.5:1"
            );
        }
    }

    #[test]
    fn mono_emits_no_color() {
        let t = Theme::new(ColorMode::Mono);
        for style in [
            t.fg(),
            t.fg_bold(),
            t.muted(),
            t.border_focus(),
            t.border_idle(),
            t.title_focus(),
            t.title_idle(),
            t.ok(),
            t.warn(),
            t.err(),
            t.accent(),
        ] {
            assert_eq!(style.fg, None, "mono style must not set fg: {style:?}");
            assert_eq!(style.bg, None, "mono style must not set bg: {style:?}");
        }
    }

    #[test]
    fn documented_ratios_hold() {
        // Locks the module docs to measured values (tolerance for f32 math).
        let bg = Color::Rgb(11, 13, 18);
        for (name, color, documented) in [
            ("fg", Color::Rgb(237, 238, 242), 16.8),
            ("muted", Color::Rgb(167, 176, 194), 8.9),
            ("accent", Color::Rgb(167, 139, 250), 7.1),
            ("ok", Color::Rgb(74, 222, 128), 11.2),
            ("warn", Color::Rgb(251, 191, 36), 11.6),
            ("err", Color::Rgb(248, 113, 113), 7.0),
        ] {
            let measured = contrast_ratio(bg, color).expect("Rgb math");
            assert!(
                (measured - documented).abs() < 0.15,
                "{name} docs say {documented}, measured {measured:.2}"
            );
        }
    }
}
