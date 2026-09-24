//! Pixel guard dog for ratatui: oq qo'riqchi it (xavfsizlik ramzi).
//!
//! Har bir terminal katagi 2 ta vertikal piksel ("▀" yarim blok), shuning uchun
//! piksellar kvadrat ko'rinadi. Hajmi: 36x18 katak.
//!
//! Foydalanish:
//! ```ignore
//! let mut dog = GuardDog::new();
//! // har kadrda:
//! dog.tick();
//! frame.render_widget(&dog, area);   // area ichida markazlashadi
//! dog.set_alert(true);               // hushyor holat
//! dog.bark();                        // hurish
//! ```
//!
//! Rang qatlamlari (har biri alohida ishlaydi): truecolor -> ANSI-16 -> monoxrom.
//! `NO_COLOR` o'rnatilgan bo'lsa avtomatik monoxrom.

use std::time::Instant;

use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

pub const CELLS_W: u16 = 36;
pub const CELLS_H: u16 = 18;

const PX_W: usize = CELLS_W as usize;
const PX_H: usize = CELLS_H as usize * 2;
// it koordinatalaridan canvas koordinatalariga (quloq "hushyor" holatda tepaga chiqadi)
const OX: i32 = 2;
const OY: i32 = 1;

/* ------------------------------ Sprite ma'lumotlari ------------------------------ */
// o kontur, w oq, l/s soya, n burun, m og'iz, p pushti, y oltin, k qalqon belgisi,
// c/b bo'yinbog' (accent), . shaffof
const HEAD_HALF: &[&str] = &[
    ".......ooooooooo",
    ".....oowwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwssss",
    "....owwwwwwswnnn",
    "....owwwwwswwwnn",
    "....owwwwwswwwwm",
    "....owwwwwswmwwm",
    "....owwwwwwswmmm",
    "....owwwwwwwssss",
    ".....owwwwwwwwww",
    "......oooooooooo",
];
const EAR: &[&str] = &[
    "..oo..", ".owwo.", ".owwwo", "owpwwo", "owpwwo", "owpwwo", "owwwwo", "owwwww",
];
const BODY_HALF: &[&str] = &[
    "........owwwwwww",
    "........owwwwwww",
    ".......owwwwwwww",
    "......owwwwwwwww",
    ".....owwwwwwwwww",
    ".....owwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwswwwwwo",
    "...owwwwswwwwwwo",
    "...owwwswwwwswwo",
    "...ooooooooooooo",
];
const COLLAR_HALF: &[&str] = &[".......obbbbbbbb", ".......occcccccc", ".......ooooooooo"];
const SHIELD: &[&str] = &[
    "oooooooo", "oyyyyyko", "oyyyykyo", "oykykyyo", "oyykyyyo", ".oyyyyo.", "..oyyo..", "...oo...",
];
const TIP_MID: &[&str] = &["...oo.", "..owwo", "..owwo", ".owwwo"];
const TIP_RIGHT: &[&str] = &["....oo", "...owo", "..owwo", ".owwwo"];
const TIP_LEFT: &[&str] = &["..oo..", ".owwo.", ".owwo.", ".owwwo"];
const TAIL_BASE: &[&str] = &[".owwwo", ".owwo.", "owwwo.", "owwo..", "owwo..", "oooo.."];
const TAIL_ORDER: [&[&str]; 4] = [TIP_MID, TIP_RIGHT, TIP_MID, TIP_LEFT];

/* ------------------------------ Rang rejimi ------------------------------ */

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorMode {
    TrueColor,
    Ansi16,
    Mono,
}

impl ColorMode {
    pub fn detect() -> Self {
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            return Self::Mono;
        }
        let truecolor = std::env::var("COLORTERM")
            .map(|v| v.contains("truecolor") || v.contains("24bit"))
            .unwrap_or(false);
        if truecolor || std::env::var_os("WT_SESSION").is_some() {
            Self::TrueColor
        } else {
            Self::Ansi16
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::TrueColor => Self::Ansi16,
            Self::Ansi16 => Self::Mono,
            Self::Mono => Self::TrueColor,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::TrueColor => "truecolor",
            Self::Ansi16 => "ansi-16",
            Self::Mono => "mono",
        }
    }
}

/* ------------------------------ Sprite yordamchilari ------------------------------ */

type Sprite = Vec<Vec<u8>>;

fn parse(rows: &[&str]) -> Sprite {
    rows.iter().map(|r| r.bytes().collect()).collect()
}

fn mirrored(half: &[&str]) -> Sprite {
    rows_mirrored(&parse(half))
}

fn rows_mirrored(s: &Sprite) -> Sprite {
    s.iter()
        .map(|r| r.iter().copied().chain(r.iter().rev().copied()).collect())
        .collect()
}

fn flip(s: &Sprite) -> Sprite {
    s.iter()
        .map(|r| r.iter().rev().copied().collect())
        .collect()
}

/// Konturga tegib turgan oq piksellarga yengil soya beradi.
fn shade(s: Sprite) -> Sprite {
    let mut out = s.clone();
    for y in 0..s.len() {
        for x in 0..s[y].len() {
            if s[y][x] != b'w' {
                continue;
            }
            let left = x.checked_sub(1).map(|i| s[y][i]);
            let right = s[y].get(x + 1).copied();
            let below = s.get(y + 1).and_then(|r| r.get(x)).copied();
            if left == Some(b'o') || right == Some(b'o') || below == Some(b'o') {
                out[y][x] = b'l';
            }
        }
    }
    out
}

struct Sprites {
    head: Sprite,
    ear_l: Sprite,
    ear_r: Sprite,
    body: Sprite,
    collar: Sprite,
    shield: Sprite,
    tail: Vec<Sprite>,
}

impl Sprites {
    fn new() -> Self {
        let ear = shade(parse(EAR));
        let tail = TAIL_ORDER
            .iter()
            .map(|tip| {
                let rows: Vec<&str> = tip.iter().chain(TAIL_BASE.iter()).copied().collect();
                shade(parse(&rows))
            })
            .collect();
        Self {
            head: shade(mirrored(HEAD_HALF)),
            ear_r: flip(&ear),
            ear_l: ear,
            body: shade(mirrored(BODY_HALF)),
            collar: mirrored(COLLAR_HALF),
            shield: parse(SHIELD),
            tail,
        }
    }
}

struct Canvas {
    px: Vec<u8>, // 0 = shaffof
}

impl Canvas {
    fn new() -> Self {
        Self {
            px: vec![0; PX_W * PX_H],
        }
    }

    fn put(&mut self, x: i32, y: i32, ch: u8) {
        let (x, y) = (x + OX, y + OY);
        if x < 0 || y < 0 || x >= PX_W as i32 || y >= PX_H as i32 {
            return;
        }
        self.px[y as usize * PX_W + x as usize] = ch;
    }

    fn blit(&mut self, s: &Sprite, x: i32, y: i32) {
        for (r, row) in s.iter().enumerate() {
            for (c, &ch) in row.iter().enumerate() {
                if ch != b'.' {
                    self.put(x + c as i32, y + r as i32, ch);
                }
            }
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, ch: u8) {
        for yy in 0..h {
            for xx in 0..w {
                self.put(x + xx, y + yy, ch);
            }
        }
    }

    fn get(&self, x: usize, y: usize) -> Option<u8> {
        match self.px[y * PX_W + x] {
            0 => None,
            ch => Some(ch),
        }
    }
}

/// `cycle` ms davrning [lo, hi) oralig'ida (delay'dan keyin) faolmi?
fn in_window(t: u64, cycle: u64, delay: u64, lo: f32, hi: f32) -> bool {
    if t < delay {
        return false;
    }
    let p = ((t - delay) % cycle) as f32 / cycle as f32;
    p >= lo && p < hi
}

/* ------------------------------ GuardDog ------------------------------ */

pub struct GuardDog {
    start: Instant,
    last: Instant,
    t: u64, // ms, oxirgi tick'dagi vaqt
    wag_phase: f32,
    alert: bool,
    animated: bool,
    bark_start: u64,
    bark_until: u64,
    accent: (u8, u8, u8),
    mode: ColorMode,
    sprites: Sprites,
}

impl Default for GuardDog {
    fn default() -> Self {
        Self::new()
    }
}

impl GuardDog {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last: now,
            t: 0,
            wag_phase: 0.0,
            alert: false,
            animated: true,
            bark_start: 0,
            bark_until: 0,
            accent: (0x2f, 0x6f, 0xed),
            mode: ColorMode::detect(),
            sprites: Sprites::new(),
        }
    }

    /// Har kadrda bir marta chaqiring (~30 FPS yetarli).
    pub fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        self.t = now.duration_since(self.start).as_millis() as u64;
        if self.animated {
            let period = if self.barking() {
                0.25
            } else if self.alert {
                0.4
            } else {
                0.8
            };
            self.wag_phase = (self.wag_phase + dt / period).fract();
        }
    }

    pub fn set_alert(&mut self, on: bool) {
        self.alert = on;
    }
    pub fn toggle_alert(&mut self) {
        self.alert = !self.alert;
    }
    pub fn is_alert(&self) -> bool {
        self.alert || self.barking()
    }
    pub fn barking(&self) -> bool {
        self.t < self.bark_until
    }
    pub fn bark(&mut self) {
        self.bark_start = self.t;
        self.bark_until = self.t + 700;
    }
    /// `false` bo'lsa it harakatsiz (reduced-motion uchun).
    pub fn set_animated(&mut self, on: bool) {
        self.animated = on;
    }
    pub fn is_animated(&self) -> bool {
        self.animated
    }
    pub fn set_accent(&mut self, r: u8, g: u8, b: u8) {
        self.accent = (r, g, b);
    }
    pub fn set_color_mode(&mut self, mode: ColorMode) {
        self.mode = mode;
    }
    pub fn color_mode(&self) -> ColorMode {
        self.mode
    }

    /* ---- kadr yig'ish ---- */

    fn compose(&self) -> Canvas {
        let mut c = Canvas::new();
        let anim = self.animated;
        let t = if anim { self.t } else { 0 };
        let barking = self.barking();
        let alert = self.alert || barking;

        // dum va tana
        let frame = ((self.wag_phase * 4.0) as usize) % 4;
        c.blit(&self.sprites.tail[frame], 28, 25);
        c.blit(&self.sprites.body, 0, 22);

        // bosh guruhi: nafas olish + hurishdagi silkinish
        let breath = if anim && (t / 900) % 2 == 1 { 1 } else { 0 };
        let shake = if barking {
            [0, -1, 0, 1][(((self.t - self.bark_start) / 50) % 4) as usize]
        } else {
            0
        };
        let (hx, hy) = (shake, breath);

        // quloqlar: hushyorlikda tepaga, vaqti-vaqti bilan qimirlaydi
        let up = if alert { -1 } else { 0 };
        let tw_l = anim && in_window(t, 5300, 0, 0.93, 0.97);
        let tw_r = anim && in_window(t, 7100, 1700, 0.93, 0.97);
        c.blit(&self.sprites.ear_l, 4 + hx, hy + up + tw_l as i32);
        c.blit(&self.sprites.ear_r, 22 + hx, hy + up + tw_r as i32);
        c.blit(&self.sprites.head, hx, 6 + hy);

        // ko'zlar (hushyor: ko'k) va ko'z qisish
        let eye = if alert { b'E' } else { b'e' };
        let blink = anim && in_window(t, 4600, 0, 0.94, 0.98);
        for ex in [9, 21] {
            if blink {
                c.rect(ex + hx, 11 + hy, 2, 1, eye);
            } else {
                c.rect(ex + hx, 9 + hy, 2, 4, eye);
                c.put(ex + hx, 9 + hy, b'g');
            }
        }

        // ochiq og'iz
        if barking {
            c.rect(12 + hx, 17 + hy, 8, 2, b'm');
            c.rect(13 + hx, 19 + hy, 6, 1, b'm');
            c.rect(14 + hx, 20 + hy, 4, 1, b'p');
        }

        // bo'yinbog' va qalqon
        c.blit(&self.sprites.collar, 0, 21);
        c.blit(&self.sprites.shield, 12, 23);

        // "!" belgisi
        if alert {
            let hop = if anim && (t / 250) % 2 == 1 { -1 } else { 0 };
            c.rect(15, hop, 2, 3, b'x');
            c.rect(15, 4 + hop, 2, 1, b'x');
        }
        c
    }

    /* ---- ranglar ---- */

    fn color(&self, ch: u8) -> Color {
        let (ar, ag, ab) = self.accent;
        if self.mode == ColorMode::Ansi16 {
            return match ch {
                b'o' | b's' => Color::DarkGray,
                b'w' | b'g' => Color::White,
                b'l' => Color::Gray,
                b'n' | b'm' | b'e' => Color::Black,
                b'p' => Color::LightRed,
                b'y' => Color::Yellow,
                b'k' | b'c' => Color::Blue,
                b'b' => Color::LightBlue,
                b'E' => Color::Cyan,
                _ => Color::Red, // x
            };
        }
        let lighten = |v: u8| (v as f32 + (255.0 - v as f32) * 0.35).round() as u8;
        match ch {
            b'o' => Color::Rgb(0x2d, 0x32, 0x50),
            b'w' | b'g' => Color::Rgb(255, 255, 255),
            b'l' => Color::Rgb(0xe3, 0xe9, 0xf4),
            b's' => Color::Rgb(0xc3, 0xcd, 0xe1),
            b'n' | b'm' => Color::Rgb(0x1c, 0x1f, 0x33),
            b'p' => Color::Rgb(0xf5, 0xa8, 0xb8),
            b'y' => Color::Rgb(0xfb, 0xbf, 0x24),
            b'k' => Color::Rgb(0x1e, 0x3a, 0x8a),
            b'c' => Color::Rgb(ar, ag, ab),
            b'b' => Color::Rgb(lighten(ar), lighten(ag), lighten(ab)),
            b'e' => Color::Rgb(0x1b, 0x1e, 0x2e),
            b'E' => Color::Rgb(0x38, 0xbd, 0xf8),
            _ => Color::Rgb(0xf4, 0x3f, 0x5e), // x
        }
    }

    /// Monoxrom rejimda faqat "siyoh" piksellar (kontur, burun, ko'z, belgi) chiziladi.
    fn is_ink(ch: u8) -> bool {
        matches!(ch, b'o' | b'n' | b'm' | b'e' | b'E' | b'k' | b'x')
    }

    /// Bitta terminal katagi: (belgi, fg, bg). `None` = tegilmaydi.
    fn cell(
        &self,
        cv: &Canvas,
        col: usize,
        row: usize,
    ) -> Option<(char, Option<Color>, Option<Color>)> {
        let top = cv.get(col, row * 2);
        let bot = cv.get(col, row * 2 + 1);
        if self.mode == ColorMode::Mono {
            let ink = |p: Option<u8>| p.is_some_and(Self::is_ink);
            return match (ink(top), ink(bot)) {
                (false, false) => None,
                (true, false) => Some(('▀', None, None)),
                (false, true) => Some(('▄', None, None)),
                (true, true) => Some(('█', None, None)),
            };
        }
        match (top, bot) {
            (None, None) => None,
            (Some(t), None) => Some(('▀', Some(self.color(t)), None)),
            (None, Some(b)) => Some(('▄', Some(self.color(b)), None)),
            (Some(t), Some(b)) => {
                let (ct, cb) = (self.color(t), self.color(b));
                if ct == cb {
                    Some(('█', Some(ct), None))
                } else {
                    Some(('▀', Some(ct), Some(cb)))
                }
            }
        }
    }

    /// Hozirgi kadrni ANSI matn sifatida qaytaradi (TUI'siz, masalan CLI banner uchun).
    pub fn to_ansi(&self) -> String {
        let cv = self.compose();
        let mut out = String::new();
        for row in 0..CELLS_H as usize {
            for col in 0..PX_W {
                match self.cell(&cv, col, row) {
                    None => out.push(' '),
                    Some((ch, fg, bg)) => {
                        if let Some(f) = fg {
                            out.push_str(&sgr(f, false));
                        }
                        if let Some(b) = bg {
                            out.push_str(&sgr(b, true));
                        }
                        out.push(ch);
                        if fg.is_some() || bg.is_some() {
                            out.push_str("\x1b[0m");
                        }
                    }
                }
            }
            out.push('\n');
        }
        out
    }
}

fn sgr(c: Color, bg: bool) -> String {
    let off = if bg { 10 } else { 0 };
    let code = match c {
        Color::Black => 30,
        Color::Red => 31,
        Color::Yellow => 33,
        Color::Blue => 34,
        Color::Cyan => 36,
        Color::Gray => 37,
        Color::DarkGray => 90,
        Color::LightRed => 91,
        Color::LightBlue => 94,
        Color::White => 97,
        Color::Rgb(r, g, b) => {
            return format!("\x1b[{};2;{};{};{}m", 38 + off, r, g, b);
        }
        _ => 39,
    };
    format!("\x1b[{}m", code + off)
}

impl Widget for &GuardDog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cv = self.compose();
        let ox = area.x + area.width.saturating_sub(CELLS_W) / 2;
        let oy = area.y + area.height.saturating_sub(CELLS_H) / 2;
        for row in 0..CELLS_H {
            for col in 0..CELLS_W {
                let (x, y) = (ox + col, oy + row);
                if x >= area.right() || y >= area.bottom() {
                    continue;
                }
                if let Some((ch, fg, bg)) = self.cell(&cv, col as usize, row as usize) {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(ch);
                    if let Some(f) = fg {
                        cell.set_fg(f);
                    }
                    if let Some(b) = bg {
                        cell.set_bg(b);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprites_are_rectangular_and_fit() {
        let s = Sprites::new();
        for sp in [&s.head, &s.ear_l, &s.body, &s.collar, &s.shield] {
            let w = sp[0].len();
            assert!(sp.iter().all(|r| r.len() == w));
        }
        assert_eq!(s.head[0].len(), 32);
        assert_eq!(s.body[0].len(), 32);
        assert!(s.tail.iter().all(|t| t.len() == 10 && t[0].len() == 6));
    }

    #[test]
    fn renders_inside_buffer_and_never_panics_on_small_area() {
        let mut dog = GuardDog::new();
        dog.bark();
        dog.set_alert(true);
        for mode in [ColorMode::TrueColor, ColorMode::Ansi16, ColorMode::Mono] {
            dog.set_color_mode(mode);
            let mut buf = Buffer::empty(Rect::new(0, 0, 40, 20));
            (&dog).render(Rect::new(0, 0, 40, 20), &mut buf);
            let mut tiny = Buffer::empty(Rect::new(0, 0, 10, 5));
            (&dog).render(Rect::new(0, 0, 10, 5), &mut tiny);
        }
    }
}
