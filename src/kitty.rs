//! Pink Kitty herself: a white kitty with a big round face, in a snug light pink
//! scarf that goes over her head and round her neck. She is not a stored picture but
//! is drawn each time out of ellipses and triangles, so that she can smile, talk, wave
//! and wear her gifts, and so that there can be a small one for a small window.
//!
//! Everything is measured in the units of the drawing she was designed in, 44 wide
//! and 48 tall with (0, 0) at the top left of her; at `Size::Medium` a unit is a
//! pixel. The user chose her from a sheet of four scarves and three faces (the snug
//! hood, the smiling eyes); `tools/kitty-sheet.py` is that sheet.

use crate::picture::Picture;
use crate::theme::Rgb;

const WHITE: Rgb = (255, 255, 255);
const SHADE: Rgb = (228, 224, 238);
const LINE: Rgb = (96, 72, 96);
const PINK: Rgb = (255, 186, 210);
const PINK_DARK: Rgb = (238, 138, 176);
const PINK_LIGHT: Rgb = (255, 218, 232);
const ROSE: Rgb = (250, 110, 150);
const BLUSH: Rgb = (255, 200, 216);
const EYE: Rgb = (52, 40, 62);
const GOLD: Rgb = (255, 205, 60);

/// She is symmetrical about this line.
const CX: f32 = 21.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    /// Cells her picture takes: wide and tall. There is room in it around her for
    /// what she wears and holds.
    pub fn cells(self) -> (u16, u16) {
        match self {
            Size::Small => (30, 15),
            Size::Medium => (60, 28),
            Size::Large => (120, 56),
        }
    }

    /// Pixels to one unit of her drawing.
    pub fn scale(self) -> f32 {
        match self {
            Size::Small => 0.5,
            Size::Medium => 1.0,
            Size::Large => 2.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Eyes {
    /// Her usual eyes: two happy arches.
    Smile,
    /// Wide open, when something surprises her.
    Open,
    Hearts,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mouth {
    Smile,
    /// Talking or eating.
    Open,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arms {
    Down,
    /// Both paws in the air.
    Up,
    /// One paw waving.
    Wave,
}

pub struct Gift {
    pub name: &'static str,
    /// A pattern for her scarf: she wears one of these at a time.
    pub scarf: bool,
}

/// What she can be given, in the order of the dressing room.
pub const GIFTS: [Gift; 12] = [
    Gift { name: "Flower", scarf: false },
    Gift { name: "Dotty scarf", scarf: true },
    Gift { name: "Magic wand", scarf: false },
    Gift { name: "Glasses", scarf: false },
    Gift { name: "Candy scarf", scarf: true },
    Gift { name: "Crown", scarf: false },
    Gift { name: "Balloon", scarf: false },
    Gift { name: "Heart scarf", scarf: true },
    Gift { name: "Butterfly", scarf: false },
    Gift { name: "Bell", scarf: false },
    Gift { name: "Fairy wings", scarf: false },
    Gift { name: "Sparkles", scarf: false },
];
const FLOWER: usize = 0;
const DOTS: usize = 1;
const WAND: usize = 2;
const GLASSES: usize = 3;
const STRIPES: usize = 4;
const CROWN: usize = 5;
const BALLOON: usize = 6;
const HEARTS: usize = 7;
const BUTTERFLY: usize = 8;
const BELL: usize = 9;
const WINGS: usize = 10;
const SPARKLES: usize = 11;

/// How she looks at one moment.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Look {
    pub eyes: Eyes,
    pub mouth: Mouth,
    pub arms: Arms,
    /// Seconds, for whatever moves by itself: her tail, a waving paw, the butterfly.
    pub time: f32,
    /// Which of `GIFTS` she has on.
    pub worn: [bool; GIFTS.len()],
}

#[cfg(test)]
impl Look {
    pub fn calm() -> Look {
        Look { eyes: Eyes::Smile, mouth: Mouth::Smile, arms: Arms::Down, time: 0.0, worn: [false; GIFTS.len()] }
    }
}

fn ell(cx: f32, cy: f32, rx: f32, ry: f32) -> impl Fn(f32, f32) -> bool + Copy {
    move |x, y| ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
}

fn tri(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> impl Fn(f32, f32) -> bool + Copy {
    let side = |p: (f32, f32), q: (f32, f32), x: f32, y: f32| (x - q.0) * (p.1 - q.1) - (p.0 - q.0) * (y - q.1);
    move |x, y| {
        let d = [side(a, b, x, y), side(b, c, x, y), side(c, a, x, y)];
        !(d.iter().any(|&v| v < 0.0) && d.iter().any(|&v| v > 0.0))
    }
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> impl Fn(f32, f32) -> bool + Copy {
    move |x, y| x > x0 && x <= x1 && y > y0 && y <= y1
}

/// Everything within `thick` of the line from `a` to `b`.
fn stroke(a: (f32, f32), b: (f32, f32), thick: f32) -> impl Fn(f32, f32) -> bool + Copy {
    move |x, y| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let along = (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        (x - a.0 - along * dx).powi(2) + (y - a.1 - along * dy).powi(2) <= thick * thick
    }
}

/// The small Pink Kitty, pixel by pixel: at half the size there are too few pixels
/// for ellipses to come out well, so she is drawn by hand. Her face is left empty
/// and her gifts are still drawn as shapes, so it matches the big one at half scale.
/// `#` is her outline, `p` her scarf and `W` her fur.
const SMALL: [&str; 24] = [
    "...#..............#...",
    "..#p#............#p#..",
    "..#pp#..######..#pp#..",
    "..#ppp##pppppp##ppp#..",
    ".#pppppppppppppppppp#.",
    ".#ppppp########ppppp#.",
    "#pppp##WWWWWWWW##pppp#",
    "#ppp#WWWWWWWWWWWW#ppp#",
    "#pp#WWWWWWWWWWWWWW#pp#",
    "#pp#WWWWWWWWWWWWWW#pp#",
    "#pp#WWWWWWWWWWWWWW#pp#",
    "#pp#WWWWWWWWWWWWWW#pp#",
    "#pp#WWWWWWWWWWWWWW#pp#",
    "#pp#WWWWWWWWWWWWWW#pp#",
    "#ppp#WWWWWWWWWWWW#ppp#",
    "#pppp##WWWWWWWW##pppp#",
    ".#pppp##########pppp#.",
    "..##pppppppppppppp##..",
    "..#pppppppppppppppp#..",
    "...###pppppppp#####...",
    "....#WWWWWWW#ppp#W#...",
    "....#WWWWWWW#ppp#WW#..",
    "....#WWW#WWW####WW#...",
    ".....###.###...##.....",
];

/// Draws shapes given in her units onto a picture of some size.
struct Painter {
    picture: Picture,
    /// Pixels to a unit.
    scale: f32,
    /// The pixel where her (0, 0) is.
    origin: (i32, i32),
}

impl Painter {
    /// Where the middle of a pixel is, in her units.
    fn unit_at(&self, px: i32, py: i32) -> (f32, f32) {
        (((px - self.origin.0) as f32 + 0.5) / self.scale - 0.5, ((py - self.origin.1) as f32 + 0.5) / self.scale - 0.5)
    }

    /// The pixel a place in her units falls in.
    fn pixel_at(&self, x: f32, y: f32) -> (i32, i32) {
        let to = |v: f32, origin: i32| origin + ((v + 0.5) * self.scale - 0.5).round() as i32;
        (to(x, self.origin.0), to(y, self.origin.1))
    }

    /// One pixel, in her units: the thinnest thing that can be drawn.
    fn pixel(&self) -> f32 {
        1.0 / self.scale
    }

    fn covered(&self, shape: impl Fn(f32, f32) -> bool) -> Vec<(i32, i32)> {
        let mut pixels = Vec::new();
        for py in 0..self.picture.h {
            for px in 0..self.picture.w {
                let (x, y) = self.unit_at(px, py);
                if shape(x, y) {
                    pixels.push((px, py));
                }
            }
        }
        pixels
    }

    /// Fills a shape, with a line one pixel wide around it if one is given.
    fn paint(&mut self, shape: impl Fn(f32, f32) -> bool, color: Rgb, line: Option<Rgb>) {
        let pixels = self.covered(&shape);
        if let Some(line) = line {
            for &(px, py) in &pixels {
                for (nx, ny) in [(px + 1, py), (px - 1, py), (px, py + 1), (px, py - 1)] {
                    let (x, y) = self.unit_at(nx, ny);
                    if !shape(x, y) {
                        self.picture.set(nx, ny, line);
                    }
                }
            }
        }
        for (px, py) in pixels {
            self.picture.set(px, py, color);
        }
    }

    /// The same on both sides of her.
    fn pair(&mut self, shape: impl Fn(f32, f32) -> bool, color: Rgb, line: Option<Rgb>) {
        self.paint(|x, y| shape(x, y) || shape(2.0 * CX - x, y), color, line);
    }

    /// Recolors the pixels of a shape that are now `only`.
    fn tint(&mut self, shape: impl Fn(f32, f32) -> bool, color: Rgb, only: Rgb) {
        for (px, py) in self.covered(shape) {
            if self.picture.get(px, py) == Some(only) {
                self.picture.set(px, py, color);
            }
        }
    }

    /// Single pixels and their mirror images, counted from her top left corner in
    /// pixels of this size: for the small things of her face, which are drawn by
    /// hand for each size.
    fn dots(&mut self, color: Rgb, pixels: &[(i32, i32)]) {
        let wide = (44.0 * self.scale) as i32;
        for &(x, y) in pixels {
            self.picture.set(self.origin.0 + x, self.origin.1 + y, color);
            self.picture.set(self.origin.0 + wide - 1 - x, self.origin.1 + y, color);
        }
    }

    /// The same without the mirror image.
    fn dot(&mut self, color: Rgb, x: i32, y: i32) {
        self.picture.set(self.origin.0 + x, self.origin.1 + y, color);
    }

    /// A darker edge along the bottom of a piece of scarf and a lighter one on top.
    fn folds(&mut self, shape: impl Fn(f32, f32) -> bool + Copy) {
        let p = self.pixel();
        self.tint(|x, y| shape(x, y) && !shape(x, y + p), PINK_DARK, PINK);
        self.tint(|x, y| shape(x, y) && !shape(x, y - p), PINK_LIGHT, PINK);
    }
}

/// Her picture.
pub fn picture(look: &Look, size: Size) -> Picture {
    // The large one is the medium one with every pixel doubled, so that she stays the
    // same drawing; the small one is drawn afresh at half the size.
    let small = size == Size::Small;
    let (w, h) = if small { (30, 30) } else { (60, 56) };
    let mut p = Painter { picture: Picture::new(w, h), scale: if small { 0.5 } else { 1.0 }, origin: if small { (4, 4) } else { (8, 6) } };
    let (px, worn, time) = (p.pixel(), look.worn, look.time);
    let line = Some(LINE);

    // Behind her: wings, and a balloon on its string.
    if worn[WINGS] {
        p.pair(ell(5.5, 37.5, 7.0, 6.5), (205, 235, 255), line);
    }
    if worn[BALLOON] {
        let drift = (time * 1.3).sin() * 0.5;
        p.paint(stroke((43.0 + drift, 13.0), (38.0, 38.0), 0.35 * px), LINE, None);
        p.paint(ell(43.0 + drift, 7.0, 4.3, 6.0), (110, 190, 255), line);
        p.tint(ell(41.2 + drift, 4.5, 1.2 * px, 1.2 * px), (200, 232, 255), (110, 190, 255));
    }

    if small {
        for (y, row) in SMALL.iter().enumerate() {
            for (x, key) in row.bytes().enumerate() {
                let color = match key {
                    b'#' => LINE,
                    b'p' => PINK,
                    b'W' => WHITE,
                    _ => continue,
                };
                p.dot(color, x as i32, y as i32);
            }
        }
    } else {
        // Her tail, body and feet. She sits on the ground, so they are flat underneath.
        p.paint(ell(34.0 + (time * 3.0).sin(), 41.0, 3.0, 5.0), WHITE, line);
        let ground = 47.6;
        let body = move |x: f32, y: f32| ell(CX, 41.0, 9.5, 7.0)(x, y) && y < ground;
        p.paint(body, WHITE, line);
        p.tint(|x, y| body(x, y) && !ell(CX, 39.5, 9.0, 6.5)(x, y), SHADE, WHITE);
        p.pair(move |x, y| ell(16.5, 46.6, 3.2, 1.5)(x, y) && y < ground, WHITE, line);

        // The scarf over her head, with her ears under it, and her face looking out.
        p.pair(tri((5.0, 15.0), (7.0, 1.0), (18.0, 7.0)), PINK, line);
        let hood = ell(CX, 19.5, 19.5, 16.0);
        p.paint(hood, PINK, line);
        p.tint(|x, y| hood(x, y) && !ell(CX, 18.5, 19.0, 15.5)(x, y), PINK_DARK, PINK);
        p.tint(|x, y| ell(CX, 17.0, 17.0, 13.0)(x, y) && !ell(CX, 18.0, 17.0, 13.0)(x, y), PINK_LIGHT, PINK);
        p.paint(ell(CX, 21.5, 14.5, 11.5), WHITE, line);

        // The scarf round her neck, and the end of it hanging down.
        let end = |x: f32, y: f32| rect(25.5, 35.5, 30.5, 43.5)(x, y) || ell(28.0, 43.5, 2.5, 1.5)(x, y);
        p.paint(end, PINK, line);
        p.tint(rect(25.5, 41.5, 30.5, 42.5), PINK_DARK, PINK);
        let wrap = ell(CX, 35.5, 11.5, 3.2);
        p.paint(wrap, PINK, line);
        p.folds(wrap);
    }

    // A pattern for the scarf, on the plain pink of it only.
    let pattern = [DOTS, STRIPES, HEARTS].into_iter().find(|&gift| worn[gift]);
    if let Some(pattern) = pattern {
        for y in 0..p.picture.h {
            for x in 0..p.picture.w {
                if p.picture.get(x, y) != Some(PINK) {
                    continue;
                }
                let (i, j) = (x - p.origin.0 + 64, y - p.origin.1 + 64);
                let color = match pattern {
                    DOTS => ((i % 4 == 1 && j % 4 == 1) || (i % 4 == 3 && j % 4 == 3)).then_some(WHITE),
                    STRIPES => ((i + j) / 2 % 2 == 0).then_some((255, 150, 190)),
                    // A heart is three pixels wide, or one when she is small.
                    _ if small => (i % 4 == 1 && j % 4 == 1).then_some(ROSE),
                    _ => matches!((i % 6, j % 6), (1 | 3, 1) | (1..=3, 2) | (2, 3)).then_some(ROSE),
                };
                if let Some(color) = color {
                    p.picture.set(x, y, color);
                }
            }
        }
    }
    if worn[BELL] {
        p.paint(ell(CX, 37.8, 2.2, 2.2), GOLD, line);
        if !small {
            p.dots(LINE, &[(21, 38), (21, 39)]);
        }
    }

    // Glasses go on before the eyes, so that the eyes are never hidden.
    if worn[GLASSES] {
        let rim = |x: f32, y: f32| ell(13.5, 23.0, 4.4, 4.4)(x, y) && !ell(13.5, 23.0, 4.4 - 1.1 * px, 4.4 - 1.1 * px)(x, y);
        p.pair(rim, (90, 70, 200), None);
        p.paint(rect(17.5, 23.0 - px, 25.5, 23.0), (90, 70, 200), None);
    }

    // Her face, by hand for each size.
    let open = (120, 50, 80);
    if small {
        match look.eyes {
            Eyes::Smile => p.dots(EYE, &[(7, 10), (6, 11), (8, 11)]),
            Eyes::Open => p.dots(EYE, &[(7, 10), (7, 11)]),
            Eyes::Hearts => p.dots(ROSE, &[(6, 10), (8, 10), (6, 11), (7, 11), (8, 11), (7, 12)]),
        }
        p.dots(BLUSH, &[(5, 13), (6, 13)]);
        p.dots(ROSE, &[(10, 12)]);
        match look.mouth {
            Mouth::Smile => p.dots(LINE, &[(9, 13), (10, 14)]),
            Mouth::Open => p.dots(open, &[(10, 13), (10, 14)]),
        }
    } else {
        match look.eyes {
            Eyes::Smile => p.dots(EYE, &[(11, 24), (12, 23), (13, 22), (14, 22), (15, 23), (16, 24)]),
            Eyes::Open => {
                p.pair(ell(13.5, 22.5, 2.2, 2.8), EYE, None);
                // The light catches both eyes from the same side.
                for x in [12, 28] {
                    for (i, j) in [(0, 21), (1, 21), (0, 22), (3, 24)] {
                        p.dot(WHITE, x + i, j);
                    }
                }
            }
            Eyes::Hearts => p.dots(
                ROSE,
                &[(11, 21), (12, 21), (14, 21), (15, 21), (11, 22), (12, 22), (13, 22), (14, 22), (15, 22), (12, 23), (13, 23), (14, 23), (13, 24)],
            ),
        }
        p.pair(ell(10.5, 27.5, 2.2, 1.2), BLUSH, None);
        p.dots(ROSE, &[(21, 26), (20, 26), (21, 27)]);
        match look.mouth {
            Mouth::Smile => p.dots(LINE, &[(21, 28), (20, 29), (19, 29), (18, 28)]),
            Mouth::Open => {
                p.paint(ell(CX, 29.6, 2.3, 1.9), open, None);
                p.dots(ROSE, &[(21, 30)]);
            }
        }
    }

    // On her head.
    if worn[FLOWER] {
        let (x, y) = (7.5, 9.5);
        let petals = |a: f32, b: f32| {
            ell(x - 2.2, y, 1.9, 1.9)(a, b) || ell(x + 2.2, y, 1.9, 1.9)(a, b) || ell(x, y - 2.2, 1.9, 1.9)(a, b) || ell(x, y + 2.2, 1.9, 1.9)(a, b)
        };
        p.paint(petals, WHITE, line);
        p.paint(ell(x, y, 1.3 * px.max(1.0), 1.3 * px.max(1.0)), GOLD, None);
    }
    if worn[CROWN] {
        let crown = |x: f32, y: f32| {
            rect(14.5, 0.5, 28.5, 4.5)(x, y)
                || tri((14.5, 1.0), (14.5, -3.0), (19.0, 1.0))(x, y)
                || tri((18.5, 1.0), (21.5, -4.0), (24.5, 1.0))(x, y)
                || tri((24.0, 1.0), (28.5, -3.0), (28.5, 1.0))(x, y)
        };
        p.paint(crown, GOLD, line);
        if !small {
            p.dots((230, 60, 90), &[(21, 2), (21, 3)]);
            p.dots((80, 160, 255), &[(17, 3)]);
        }
    }
    if worn[BUTTERFLY] {
        let flap = 0.55 + 0.45 * (time * 7.0).sin().abs();
        let (x, y) = (37.0, -1.0);
        let wings = |a: f32, b: f32| ell(x - 2.4 * flap, y, 2.4 * flap + 0.4, 3.0)(a, b) || ell(x + 2.4 * flap, y, 2.4 * flap + 0.4, 3.0)(a, b);
        p.paint(wings, (176, 124, 255), line);
        p.paint(stroke((x, y - 2.5), (x, y + 2.5), 0.4 * px), LINE, None);
    }

    // Her paws, when they are up.
    match look.arms {
        Arms::Down => {}
        Arms::Up => p.pair(ell(5.0, 29.0, 2.8, 3.2), WHITE, line),
        Arms::Wave => p.paint(ell(38.5 + (time * 11.0).sin() * 1.6, 28.5 - (time * 11.0).cos().abs(), 2.8, 3.2), WHITE, line),
    }

    if worn[WAND] {
        p.paint(stroke((6.0, 45.0), (0.0, 29.0), 0.5 * px), (150, 100, 60), None);
        let star = |x: f32, y: f32| tri((-0.5, 20.5), (-5.0, 28.5), (4.0, 28.5))(x, y) || tri((-0.5, 31.0), (-5.0, 23.0), (4.0, 23.0))(x, y);
        p.paint(star, (255, 225, 80), line);
    }
    if worn[SPARKLES] {
        for (i, &(x, y)) in [(-4.0, 6.0), (48.0, 24.0), (-5.0, 38.0), (47.0, 44.0), (11.0, -3.0), (31.0, -3.0)].iter().enumerate() {
            let glow = (time * 4.0 + i as f32 * 1.7).sin();
            if glow > 0.0 {
                let (cx, cy) = p.pixel_at(x, y);
                p.picture.set(cx, cy, (255, 225, 90));
                if !small && glow > 0.5 {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        p.picture.set(cx + dx, cy + dy, (255, 240, 170));
                    }
                }
            }
        }
    }

    if size == Size::Large { p.picture.scaled(2) } else { p.picture }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(picture: &Picture, color: Rgb) -> usize {
        (0..picture.h).flat_map(|y| (0..picture.w).map(move |x| (x, y))).filter(|&(x, y)| picture.get(x, y) == Some(color)).count()
    }

    #[test]
    fn she_fits_her_cells_and_is_the_same_on_both_sides() {
        for size in [Size::Small, Size::Medium, Size::Large] {
            let picture = picture(&Look::calm(), size);
            assert_eq!(picture.cells(), size.cells(), "{size:?}");
            // White, pink and an outline, and nothing touching the edge of the picture.
            assert!(count(&picture, WHITE) > 80 && count(&picture, PINK) > 80 && count(&picture, LINE) > 50, "{size:?}");
            for x in 0..picture.w {
                assert_eq!((picture.get(x, 0), picture.get(x, picture.h - 1)), (None, None), "{size:?}");
            }
            // Her head is symmetrical: only her tail and the end of her scarf are not.
            for y in 0..picture.h * 5 / 8 {
                for x in 0..picture.w {
                    assert_eq!(picture.get(x, y), picture.get(picture.w - 1 - x, y), "{size:?} at {x}, {y}");
                }
            }
        }
    }

    #[test]
    fn every_face_and_every_gift_changes_the_picture() {
        for size in [Size::Small, Size::Medium] {
            let calm = picture(&Look { time: 0.3, ..Look::calm() }, size);
            for look in [
                Look { eyes: Eyes::Open, time: 0.3, ..Look::calm() },
                Look { eyes: Eyes::Hearts, time: 0.3, ..Look::calm() },
                Look { mouth: Mouth::Open, time: 0.3, ..Look::calm() },
                Look { arms: Arms::Up, time: 0.3, ..Look::calm() },
                Look { arms: Arms::Wave, time: 0.3, ..Look::calm() },
            ] {
                assert_ne!(picture(&look, size), calm, "{look:?} at {size:?}");
            }
            let mut seen = vec![calm];
            for gift in 0..GIFTS.len() {
                let mut worn = [false; GIFTS.len()];
                worn[gift] = true;
                // A moment at which the sparkles are lit.
                let with = picture(&Look { worn, time: 0.3, ..Look::calm() }, size);
                assert!(!seen.contains(&with), "{} at {size:?}", GIFTS[gift].name);
                seen.push(with);
            }
            // Wearing everything at once fits in the picture too.
            let all = picture(&Look { worn: [true; GIFTS.len()], time: 0.3, ..Look::calm() }, size);
            for x in 0..all.w {
                assert_eq!((all.get(x, 0), all.get(x, all.h - 1)), (None, None));
            }
            for y in 0..all.h {
                assert_eq!((all.get(0, y), all.get(all.w - 1, y)), (None, None));
            }
        }
        assert_eq!(GIFTS.iter().filter(|gift| gift.scarf).count(), 3);
        // A name has to fit its button in the dressing room of the smallest window.
        assert!(GIFTS.iter().all(|gift| gift.name.len() <= 11));
    }
}
