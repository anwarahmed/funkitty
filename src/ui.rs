//! All drawing: the layout of each screen, Pink Kitty in her place, the treats and the
//! balls of yarn, the keyboard, and the list of clickable rectangles (`App::buttons`),
//! which is rebuilt every time.
//!
//! Pictures are `Picture`s of square pixels, two to a cell. Words a child has to read
//! are drawn in the big letters of `font` wherever they fit, and as ordinary bold text
//! where they do not.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::{Action, App, BALLOON, FLY, Fun, Game, Play, Screen};
use crate::font;
use crate::kitty::{self, GIFTS, Size};
use crate::picture::Picture;
use crate::theme::{Rgb, Theme};
use crate::words::{BLANK, Level};

/// The window has to be at least this big: the size a terminal opens at.
pub const MIN: (u16, u16) = (80, 24);

const TITLE: &str = "Fun time with Pink Kitty";

fn color(c: Rgb) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

/// `a` with `t` (from 0 to 1) of `b` mixed in.
fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let part = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)).round() as u8;
    (part(a.0, b.0), part(a.1, b.1), part(a.2, b.2))
}

/// Dark or white, whichever can be read on `bg`.
fn ink(bg: Rgb) -> Rgb {
    let light = 0.299 * bg.0 as f32 + 0.587 * bg.1 as f32 + 0.114 * bg.2 as f32;
    if light > 150.0 { (33, 28, 38) } else { (255, 255, 255) }
}

fn fill(buf: &mut Buffer, r: Rect, bg: Rgb) {
    let r = r.intersection(buf.area);
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            buf[(x, y)].reset();
            buf[(x, y)].set_bg(color(bg));
        }
    }
}

/// One line of ordinary text, bold, on whatever background is there. Cut at `max` cells.
fn put(buf: &mut Buffer, x: u16, y: u16, text: &str, fg: Rgb, max: u16) {
    if x < buf.area.right() && y < buf.area.bottom() {
        buf.set_stringn(x, y, text, max as usize, Style::new().fg(color(fg)).add_modifier(Modifier::BOLD));
    }
}

/// The same, in the middle of `r`'s width, on row `y`.
fn centered(buf: &mut Buffer, r: Rect, y: u16, text: &str, fg: Rgb) {
    let width = (text.chars().count() as u16).min(r.width);
    put(buf, r.x + (r.width - width) / 2, y, text, fg, width);
}

/// Breaks ordinary text into lines of at most `width` characters.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// Rows of cells one line of big letters takes, with the space under it.
fn line_rows(scale: usize) -> u16 {
    font::rows(scale) + u16::from(scale > 1)
}

/// The lines `text` makes in big letters `scale` times the font's size, if it fits `r`.
fn big_lines(text: &str, r: Rect, scale: usize) -> Option<Vec<String>> {
    let lines = font::wrap(text, r.width as usize / scale)?;
    (!lines.is_empty() && lines.len() as u16 * line_rows(scale) <= r.height).then_some(lines)
}

/// Writes `text` in the middle of `r`, each letter in the color `paint` gives it (by
/// its place in the text, counting the spaces): in big letters up to `scale` times
/// the font's size where that fits (`scale` 0 for never), and otherwise as ordinary
/// bold text. `paint` may also change what is drawn for a letter.
fn letters(buf: &mut Buffer, r: Rect, text: &str, scale: usize, paint: &dyn Fn(usize, char) -> (char, Rgb)) {
    if r.width == 0 || r.height == 0 {
        return;
    }
    // The text as its lines will be: words with one space between them.
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    for scale in (1..=scale).rev() {
        let Some(lines) = big_lines(&text, r, scale) else { continue };
        // In pixels: the last row of a line is half a cell, so an odd number of rows
        // still looks centered.
        let tall = lines.len() as i32 * line_rows(scale) as i32 * 2 - if scale == 1 { 1 } else { 2 };
        let top = r.y as i32 * 2 + (r.height as i32 * 2 - tall).max(0) / 2;
        let mut place = 0;
        for (i, line) in lines.iter().enumerate() {
            let mut x = r.x as i32 + (r.width as i32 - (font::width(line) * scale) as i32) / 2;
            for c in line.chars() {
                let (shown, fg) = paint(place, c);
                let shown = shown.to_string();
                font::draw(buf, x, top + i as i32 * line_rows(scale) as i32 * 2, &shown, color(fg), scale);
                x += ((font::width(&c.to_string()) + 1) * scale) as i32;
                place += 1;
            }
            // The space the line was broken at.
            place += 1;
        }
        return;
    }
    // Ordinary text: spread out when there is room, since single letters matter here.
    let count = text.chars().count();
    let spread = count * 2 <= r.width as usize + 1 && count <= 24 && scale > 0;
    let lines = if spread { vec![text.clone()] } else { wrap(&text, r.width as usize) };
    let shown = lines.len().min(r.height as usize);
    let top = r.y + (r.height - shown as u16) / 2;
    let mut place = 0;
    for (i, line) in lines.iter().take(shown).enumerate() {
        let step = if spread { 2 } else { 1 };
        let wide = ((line.chars().count() * step + 1 - step) as u16).min(r.width);
        let mut x = r.x + (r.width - wide) / 2;
        for c in line.chars() {
            let (shown, fg) = paint(place, c);
            if x < r.right() {
                put(buf, x, top + i as u16, &shown.to_string(), fg, 1);
            }
            x += step as u16;
            place += 1;
        }
        place += 1;
    }
}

/// Writes `text` in the middle of `r` in one color: big where it fits, plain where not.
fn label(buf: &mut Buffer, r: Rect, text: &str, fg: Rgb, scale: usize) {
    if scale == 0 {
        // Never spread out: this is a caption, not something to type.
        let lines = wrap(text, r.width as usize);
        let shown = lines.len().min(r.height as usize);
        let top = r.y + (r.height - shown as u16) / 2;
        for (i, line) in lines.iter().take(shown).enumerate() {
            centered(buf, r, top + i as u16, line, fg);
        }
    } else if big_lines(text, r, 1).is_some() {
        letters(buf, r, text, scale, &|_, c| (c, fg));
    } else {
        label(buf, r, text, fg, 0);
    }
}

/// A line around the inside edge of `r`.
fn frame(buf: &mut Buffer, r: Rect, fg: Rgb) {
    let (right, bottom) = (r.right() - 1, r.bottom() - 1);
    for x in r.x + 1..right {
        put(buf, x, r.y, "━", fg, 1);
        put(buf, x, bottom, "━", fg, 1);
    }
    for y in r.y + 1..bottom {
        put(buf, r.x, y, "┃", fg, 1);
        put(buf, right, y, "┃", fg, 1);
    }
    for (x, y, corner) in [(r.x, r.y, "┏"), (right, r.y, "┓"), (r.x, bottom, "┗"), (right, bottom, "┛")] {
        put(buf, x, y, corner, fg, 1);
    }
}

/// A filled, clickable rectangle. When it is the one the marker is on, it has a frame
/// (or arrows at its ends when it is too flat for one) to show that Enter presses it.
/// Returns the part inside the frame, where its words go.
fn button(buf: &mut Buffer, app: &mut App, r: Rect, action: Option<Action>, bg: Rgb) -> Rect {
    fill(buf, r, bg);
    if let Some(action) = action {
        app.buttons.push((r, action));
    }
    let marked = action.is_some() && app.marker == action;
    let fg = ink(bg);
    if r.height >= 3 && r.width >= 4 {
        if marked {
            frame(buf, r, fg);
        }
        Rect::new(r.x + 2, r.y + 1, r.width - 4, r.height - 2)
    } else {
        if marked && r.width >= 4 {
            for y in r.top()..r.bottom() {
                put(buf, r.x, y, "▶", fg, 1);
                put(buf, r.right() - 1, y, "◀", fg, 1);
            }
        }
        Rect::new(r.x + 1.min(r.width), r.y, r.width.saturating_sub(2), r.height)
    }
}

/// A button with words on it.
fn worded(buf: &mut Buffer, app: &mut App, r: Rect, action: Option<Action>, bg: Rgb, text: &str, scale: usize) {
    let inside = button(buf, app, r, action, bg);
    label(buf, inside, text, ink(bg), scale);
}

/// `count` rectangles side by side across `r`, with a cell between them.
fn across(r: Rect, count: u16) -> Vec<Rect> {
    let wide = (r.width + 1) / count.max(1);
    (0..count).map(|i| Rect::new(r.x + i * wide, r.y, wide.saturating_sub(1), r.height)).collect()
}

/// The biggest Pink Kitty that fits a space this wide and tall.
fn kitty_size(wide: u16, high: u16) -> Size {
    [Size::Large, Size::Medium].into_iter().find(|size| size.cells().0 <= wide && size.cells().1 <= high).unwrap_or(Size::Small)
}

/// Draws her standing in the middle of the bottom of `r`, and notes where she is.
fn kitty(buf: &mut Buffer, app: &mut App, r: Rect, size: Size, rise: f32) {
    let (w, h) = size.cells();
    let (dx, dy) = app.hop();
    let x = r.x as i32 + (r.width as i32 - w as i32) / 2;
    let y = (r.bottom() as i32 - h as i32) * 2;
    let lift = (dy * size.scale() + rise * h as f32 * 2.0).round() as i32;
    kitty::picture(&app.look(), size).blit(buf, x + (dx * size.scale()).round() as i32, y + lift);
    app.kitty = Rect::new(x.max(0) as u16, r.bottom().saturating_sub(h), w, h).intersection(buf.area);
}

/// What she is saying, in a box with a point towards her on the left.
fn bubble(buf: &mut Buffer, r: Rect, text: &str, theme: &Theme) {
    if r.height == 0 || r.width < 6 {
        return;
    }
    fill(buf, r, theme.panel);
    if r.x > 0 {
        put(buf, r.x - 1, r.y + r.height / 2, "◀", theme.panel, 1);
    }
    let pad = u16::from(r.height >= 3);
    label(buf, Rect::new(r.x + 2, r.y + pad, r.width - 4, r.height - 2 * pad), text, theme.text, usize::from(r.height >= 6));
}

/// How tall her speech bubble is, where this many rows are to be shared out.
fn bubble_rows(high: u16) -> u16 {
    match high {
        0..22 => 3,
        22..30 => 5,
        _ => 6,
    }
}

/// The title, each letter in its own color, the colors wandering along it.
fn title(buf: &mut Buffer, r: Rect, app: &App, scale: usize) {
    let theme = app.theme();
    let shift = (app.time * 6.0) as usize;
    letters(buf, r, TITLE, scale, &|i, c| (c, theme.buttons[(i + shift) % theme.buttons.len()]));
}

/// The row at the top of a game: the way back, what is being played and how far on.
fn top(buf: &mut Buffer, app: &mut App, area: Rect, name: &str, repeat: bool) {
    let theme = app.theme();
    worded(buf, app, Rect::new(area.x, area.y, 12, 1), Some(Action::Back), theme.panel, "Esc Back", 0);
    if repeat {
        worded(buf, app, Rect::new(area.x + 13, area.y, 17, 1), Some(Action::Repeat), theme.panel, "Tab Say again", 0);
    }
    centered(buf, area, area.y, name, theme.text);
    if let Some((done, all)) = app.task.as_ref().map(|task| task.progress()) {
        let x = area.right().saturating_sub(all as u16 * 2 + 1);
        for i in 0..all {
            put(buf, x + i as u16 * 2, area.y, if i < done { "●" } else { "○" }, if i < done { theme.accent } else { theme.dim }, 1);
        }
    }
}

const KEYS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];

/// Rows the keyboard takes: with small keys or with big ones.
fn keyboard_rows(big: bool) -> u16 {
    if big { 11 } else { 4 }
}

/// A keyboard to click on, which also shows where the keys are: the one wanted next
/// is lit, when the game gives that away.
fn keyboard(buf: &mut Buffer, app: &mut App, r: Rect, big: bool, space: bool) {
    let theme = app.theme();
    let wanted = app.wanted();
    let (wide, high, gap) = if big { (5, 2, 1) } else { (3, 1, 0) };
    let left = r.x + r.width.saturating_sub(10 * (wide + 1) - 1) / 2;
    for (row, keys) in KEYS.iter().enumerate() {
        for (i, key) in keys.chars().enumerate() {
            let key_rect = Rect::new(left + row as u16 * (wide + 1) / 2 + i as u16 * (wide + 1), r.y + row as u16 * (high + gap), wide, high);
            let bg = if wanted == Some(key) { theme.accent } else { theme.key };
            fill(buf, key_rect, bg);
            app.buttons.push((key_rect, Action::Key(key)));
            put(buf, key_rect.x + wide / 2, key_rect.y + (high - 1) / 2, &key.to_string(), ink(bg), 1);
        }
    }
    if space {
        let bar = Rect::new(left + 2 * (wide + 1), r.y + 3 * (high + gap), 6 * (wide + 1) - 1, high);
        let bg = if wanted == Some(' ') { theme.accent } else { theme.key };
        fill(buf, bar, bg);
        app.buttons.push((bar, Action::Key(' ')));
        centered(buf, bar, bar.y + (high - 1) / 2, "space", ink(bg));
    }
}

/// A picture as rows of characters, and the color each character stands for.
type Sprite = (&'static [&'static str], &'static [(u8, Rgb)]);

/// What Pink Kitty is given to eat.
const TREATS: [Sprite; 6] = [
    (
        &["..BBBBB....B.", ".BBBBBBB..BB.", "BBKBBBBBBBBB.", "BBBBBBLBBBB..", "BBBBBBBBBBBB.", ".BBBBBBB..BB.", "..BBBBB....B."],
        &[(b'B', (90, 170, 240)), (b'K', (30, 40, 70)), (b'L', (150, 205, 250))],
    ),
    (
        &["..CCCCC..", ".CCKCCCC.", "CCCCCCKCC", "CCCCCCCCC", "CKCCKCCCC", "CCCCCCCCC", "CCCCCCKCC", ".CCKCCCC.", "..CCCCC.."],
        &[(b'C', (215, 160, 95)), (b'K', (110, 65, 40))],
    ),
    (
        &["....R....", "...PPP...", "..PPPPP..", ".PPWPPPP.", "PPPPPPPPP", ".YYYYYYY.", ".YOYOYOY.", "..YOYOY..", "..YYYYY.."],
        &[(b'R', (230, 50, 70)), (b'P', (255, 150, 195)), (b'W', (255, 215, 232)), (b'Y', (250, 215, 130)), (b'O', (225, 175, 85))],
    ),
    (
        &["..LLL..", "..WWW..", ".WWWWW.", ".WWWWW.", ".WBBBW.", ".WBBBW.", ".WWWWW.", ".WWWWW.", ".WWWWW."],
        &[(b'L', (80, 150, 235)), (b'W', (255, 255, 255)), (b'B', (150, 200, 250))],
    ),
    (
        &[".....GG..", "....S....", ".RRRSRRR.", "RRRRRRRRR", "RWRRRRRRR", "RRRRRRRRR", "RRRRRRRRR", ".RRRRRRR.", "..RR.RR.."],
        &[(b'G', (90, 185, 90)), (b'S', (120, 80, 50)), (b'R', (235, 65, 75)), (b'W', (255, 170, 175))],
    ),
    (&["......YYYY", "...YYYYYYY", "YYYYYOYYYY", "YYOYYYYYYY", "YYYYYYYOYY", "YOYYYYYYYY", "YYYYYYYYYY"], &[(b'Y', (255, 210, 70)), (b'O', (225, 160, 40))]),
];

/// The treat with this number, `times` its size, with a dark line around it.
fn treat(number: usize, times: i32) -> Picture {
    let (rows, palette) = TREATS[number % TREATS.len()];
    let mut picture = Picture::sprite(rows, palette);
    picture.outline((96, 72, 96));
    picture.scaled(times)
}

/// A ball of yarn `radius` pixels round, its strands turned by `roll`.
fn ball(radius: i32, tint: Rgb, roll: i32) -> Picture {
    let side = radius * 2 + 3;
    let mut picture = Picture::new(side, side);
    let dark = mix(tint, (60, 40, 70), 0.3);
    for y in 0..side {
        for x in 0..side {
            let (dx, dy) = (x - radius - 1, y - radius - 1);
            if dx * dx + dy * dy <= radius * radius + radius / 2 {
                // Strands wound across it, which move as it rolls.
                let strand = (dx * 2 + dy + roll).rem_euclid(if radius > 4 { 6 } else { 4 }) == 0;
                picture.set(x, y, if strand { dark } else { tint });
            }
        }
    }
    picture.outline((96, 72, 96));
    picture
}

/// The three parts of a game's screen: Pink Kitty on the left, the task beside her,
/// and whatever goes along the bottom.
struct Stage {
    size: Size,
    kitty: Rect,
    side: Rect,
    below: Rect,
}

/// Shares out `area` (everything under the top row), keeping `rows` at the bottom.
fn stage(area: Rect, rows: u16) -> Stage {
    let rows = rows.min(area.height.saturating_sub(15));
    let scene = Rect::new(area.x, area.y, area.width, area.height - rows);
    let size = kitty_size(scene.width.saturating_sub(48), scene.height);
    let wide = size.cells().0 + 1;
    Stage {
        size,
        kitty: Rect::new(scene.x, scene.y, wide, scene.height),
        side: Rect::new(scene.x + wide + 1, scene.y, scene.width - wide - 2, scene.height),
        below: Rect::new(area.x, scene.bottom(), area.width, rows),
    }
}

/// Snack time and "Kitty says": something to type, letter by letter.
fn typing(buf: &mut Buffer, app: &mut App, area: Rect) {
    let theme = app.theme();
    let Some(task) = &app.task else { return };
    let Play::Typing(typing) = &task.play else { return };
    let (game, level) = (task.game, task.level);
    let item = typing.items.get(typing.at).cloned().unwrap_or_default();
    let (at, typed, cheer) = (typing.at, typing.typed, typing.cheer);
    let concealed = app.concealed();

    let big = area.height >= 40 && area.width >= 100;
    let space = item.contains(' ');
    let stage = stage(area, keyboard_rows(big) + 1);
    kitty(buf, app, stage.kitty, stage.size, 0.0);
    let high = bubble_rows(stage.side.height);
    let mut bubble_rect = Rect::new(stage.side.x, stage.side.y, stage.side.width, high);
    let mut task_rect = Rect::new(stage.side.x, stage.side.y + high + 1, stage.side.width, stage.side.height.saturating_sub(high + 1));

    if game == Game::Snack {
        // The treat flies to her mouth when its word is typed. It waits above the
        // word, or, where a sentence needs every row there is, beside what she says.
        let times = match task_rect.height {
            0..16 => 1,
            16..30 => 2,
            _ => 3,
        };
        let picture = treat(at, times);
        let (wide, rows) = picture.cells();
        let beside = times == 1;
        let home = if beside {
            bubble_rect.width -= wide + 2;
            (bubble_rect.right() as f32 + 1.0, bubble_rect.y as f32 * 2.0)
        } else {
            (task_rect.x as f32 + (task_rect.width as f32 - wide as f32) / 2.0, task_rect.y as f32 * 2.0)
        };
        let mouth =
            (app.kitty.x as f32 + (app.kitty.width as f32 - wide as f32) / 2.0, (app.kitty.y as f32 + app.kitty.height as f32 * 0.5) * 2.0 - rows as f32);
        let flown = cheer.map_or(0.0, |age| if app.animations { (age / FLY).min(1.0) } else { 1.0 });
        if flown < 1.0 {
            let x = home.0 + (mouth.0 - home.0) * flown;
            // A little arc on the way.
            let y = home.1 + (mouth.1 - home.1) * flown - (flown * std::f32::consts::PI).sin() * 6.0;
            picture.blit(buf, x.round() as i32, y.round() as i32);
        }
        if !beside {
            task_rect = Rect::new(task_rect.x, task_rect.y + rows + 1, task_rect.width, task_rect.height.saturating_sub(rows + 1));
        }
    }
    bubble(buf, bubble_rect, &app.bubble, theme);

    let scale = if level == Level::Easy { 4 } else { 3 };
    let done = if cheer.is_some() { item.chars().count() } else { typed };
    letters(buf, task_rect, &item, scale, &|i, c| {
        if i < done {
            (c, theme.good)
        } else if concealed {
            ('_', if i == done { theme.accent } else { theme.dim })
        } else if i == done {
            (c, theme.accent)
        } else {
            (c, theme.text)
        }
    });
    keyboard(buf, app, Rect::new(stage.below.x, stage.below.y + 1, stage.below.width, stage.below.height.saturating_sub(1)), big, space);
}

/// Yarn balls: letters rolling towards her along the floor.
fn yarn(buf: &mut Buffer, app: &mut App, area: Rect) {
    let theme = app.theme();
    let big = area.height >= 40 && area.width >= 100;
    let stage = stage(area, keyboard_rows(big) + 1);
    // The floor, under her feet and all the way across.
    for x in stage.below.left()..stage.below.right() {
        put(buf, x, stage.below.y, "▀", theme.panel, 1);
    }
    let high = bubble_rows(stage.side.height).min(5);
    bubble(buf, Rect::new(stage.side.x, stage.side.y, stage.side.width, high), &app.bubble, theme);
    kitty(buf, app, stage.kitty, stage.size, 0.0);

    let radius = match stage.size {
        Size::Small => 3,
        Size::Medium => 6,
        Size::Large => 12,
    };
    let Some(task) = &app.task else { return };
    let Play::Yarn(yarn) = &task.play else { return };
    let (first, last) = (stage.kitty.right() as f32 - stage.size.cells().0 as f32 * 0.2, stage.below.right() as f32 - (radius * 2 + 3) as f32);
    let floor = stage.below.y as i32 * 2;
    let mut spots = Vec::new();
    for rolling in yarn.balls.iter().filter(|ball| ball.popped.is_none()) {
        let x = (first + (last - first) * rolling.x).round() as i32;
        let y = floor - radius * 2 - 3;
        let tint = theme.buttons[rolling.color % theme.buttons.len()];
        ball(radius, tint, x).blit(buf, x, y);
        let (cx, cy) = (x + radius + 1, y + radius + 1);
        let fg = ink(tint);
        if radius >= 6 {
            let scale = (radius / 6) as usize;
            // A patch behind the letter, so that the strands do not run through it.
            let patch = Rect::new((cx - 3 * scale as i32).max(0) as u16, ((cy - 4 * scale as i32) / 2).max(0) as u16, 7 * scale as u16, 4 * scale as u16);
            fill(buf, patch, tint);
            font::draw(buf, cx - (5 * scale as i32) / 2, cy - (7 * scale as i32) / 2, &rolling.letter.to_string(), color(fg), scale);
        } else if let (Ok(cx), Ok(cy)) = (u16::try_from(cx), u16::try_from(cy / 2)) {
            fill(buf, Rect::new(cx, cy, 1, 1), tint);
            put(buf, cx, cy, &rolling.letter.to_string(), fg, 1);
        }
        let spot = Rect::new(x.max(0) as u16, (y / 2).max(0) as u16, radius as u16 * 2 + 3, radius as u16 + 2).intersection(buf.area);
        spots.push((spot, Action::Key(rolling.letter)));
    }
    app.buttons.extend(spots);
    keyboard(buf, app, Rect::new(stage.below.x, stage.below.y + 1, stage.below.width, stage.below.height.saturating_sub(1)), big, false);
}

/// Story time: a sentence with a word missing, and three words to choose from.
fn story(buf: &mut Buffer, app: &mut App, area: Rect) {
    let theme = app.theme();
    let Some(task) = &app.task else { return };
    let Play::Quiz(quiz) = &task.play else { return };
    let Some(question) = quiz.items.get(quiz.at) else { return };
    let easy = task.level == Level::Easy;
    let (text, options, right, tried, solved) = (question.text.clone(), question.options.clone(), question.right, quiz.tried, quiz.solved.is_some());

    let high = match area.height {
        0..34 => 3,
        34..50 => 5,
        _ => 7,
    };
    let stage = stage(area, high + 2);
    kitty(buf, app, stage.kitty, stage.size, 0.0);
    let rows = bubble_rows(stage.side.height);
    bubble(buf, Rect::new(stage.side.x, stage.side.y, stage.side.width, rows), &app.bubble, theme);
    let sentence = Rect::new(stage.side.x, stage.side.y + rows + 1, stage.side.width, stage.side.height.saturating_sub(rows + 1));
    if solved {
        // The whole sentence, with its word in place and in the color of a right answer.
        let before = text.split(BLANK).next().unwrap_or("").chars().count();
        let word = options[right].chars().count();
        letters(buf, sentence, &text.replace(BLANK, &options[right]), 3, &|i, c| {
            (c, if (before..before + word).contains(&i) { theme.good } else { theme.text })
        });
    } else {
        letters(buf, sentence, &text, 3, &|_, c| (c, theme.text));
    }

    let row = Rect::new(stage.below.x + 1, stage.below.y + 1, stage.below.width - 2, high);
    if solved {
        let next = Rect::new(row.x + row.width / 4, row.y, row.width / 2, row.height);
        worded(buf, app, next, Some(Action::Next), theme.accent, "Next", 1);
    } else {
        for (i, r) in across(row, 3).into_iter().enumerate() {
            // On Easy the words are letters, and the letter's own key chooses it.
            let words = if easy { options[i].clone() } else { format!("{}: {}", (b'A' + i as u8) as char, options[i]) };
            if tried[i] {
                worded(buf, app, r, None, mix(theme.panel, theme.bg, 0.5), &words, 0);
            } else {
                worded(buf, app, r, Some(Action::Pick(i)), theme.buttons[i], &words, 2);
            }
        }
    }
}

fn play(buf: &mut Buffer, app: &mut App, area: Rect) {
    let Some(task) = &app.task else { return };
    let (game, kind) = (task.game, std::mem::discriminant(&task.play));
    top(buf, app, area, game.name(), game == Game::Says || game == Game::Story);
    let rest = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
    let Some(task) = &app.task else { return };
    debug_assert_eq!(kind, std::mem::discriminant(&task.play));
    match task.play {
        Play::Typing(_) => typing(buf, app, rest),
        Play::Yarn(_) => yarn(buf, app, rest),
        Play::Quiz(_) => story(buf, app, rest),
    }
}

/// How many rows the title takes in a window this big, and how big its letters are.
fn title_rows(area: Rect) -> (u16, usize) {
    let one_line = font::width(TITLE) as u16 + 2;
    if area.height >= 60 && area.width >= one_line * 2 {
        (line_rows(2) + 1, 2)
    } else if area.height >= 34 && area.width >= one_line {
        (line_rows(1) + 1, 1)
    } else if area.height >= 44 {
        (2 * line_rows(1) + 1, 1)
    } else {
        (1, 0)
    }
}

/// The home screen: Pink Kitty, the games, the level and the settings.
fn home(buf: &mut Buffer, app: &mut App, area: Rect) {
    let theme = app.theme();
    let (title_high, scale) = title_rows(area);
    title(buf, Rect::new(area.x, area.y, area.width, title_high), app, scale);

    // The settings, along the bottom.
    // Shorter words in a narrow window, where a fifth of it is not much.
    let roomy = area.width >= 100;
    let on = |on: bool| if on { "on" } else { "off" };
    let settings = [
        (if roomy { format!("T Theme: {}", theme.name) } else { format!("T {}", theme.name) }, Action::Theme),
        (format!("S Sound{} {}", if roomy { ":" } else { "" }, on(app.sound)), Action::Sound),
        (format!("M Motion{} {}", if roomy { ":" } else { "" }, on(app.animations)), Action::Motion),
        ("? Help".to_string(), Action::Help),
        ("Q Quit".to_string(), Action::Quit),
    ];
    let bottom = Rect::new(area.x + 1, area.bottom() - 2, area.width - 2, 1);
    for (r, (words, action)) in across(bottom, 5).into_iter().zip(settings) {
        worded(buf, app, r, Some(action), theme.panel, &words, 0);
    }

    let body = Rect::new(area.x, area.y + title_high, area.width, area.height - title_high - 3);
    let size = kitty_size(body.width.saturating_sub(48), body.height);
    let wide = size.cells().0 + 1;
    kitty(buf, app, Rect::new(body.x, body.y, wide, body.height), size, 0.0);
    let side = Rect::new(body.x + wide + 1, body.y, body.width - wide - 2, body.height);
    let high = bubble_rows(side.height);
    bubble(buf, Rect::new(side.x, side.y, side.width, high), &app.bubble, theme);

    // What is left under the bubble: three rows of two games, the levels, the gifts.
    let rest = side.height - high - 1;
    let level_high = if rest >= 20 { 3 } else { 1 };
    let game_high = ((rest - level_high - 5) / 3).clamp(1, 7);
    let mut y = side.y + high + 1;
    let games: [(String, Action); 6] = [
        (format!("1 {}", Game::Snack.name()), Action::Play(Game::Snack)),
        (format!("2 {}", Game::Says.name()), Action::Play(Game::Says)),
        (format!("3 {}", Game::Yarn.name()), Action::Play(Game::Yarn)),
        (format!("4 {}", Game::Story.name()), Action::Play(Game::Story)),
        ("5 Dress up".to_string(), Action::Dress),
        ("6 Surprise".to_string(), Action::Surprise),
    ];
    for (row, pair) in games.chunks(2).enumerate() {
        for (i, r) in across(Rect::new(side.x, y, side.width, game_high), 2).into_iter().enumerate() {
            let (words, action) = &pair[i];
            worded(buf, app, r, Some(*action), theme.buttons[row * 2 + i], words, 1);
        }
        y += game_high + 1;
    }
    for (r, level) in across(Rect::new(side.x, y, side.width, level_high), 3).into_iter().zip(Level::ALL) {
        let bg = if app.level == level { theme.accent } else { theme.panel };
        worded(buf, app, r, Some(Action::Level(level)), bg, &format!("{} ({})", level.name(), level.ages()), 0);
    }
    y += level_high + 1;
    if y < body.bottom() {
        let hearts = if app.gifts.hearts > 0 { format!("   Hearts: {}", app.gifts.hearts) } else { String::new() };
        centered(buf, side, y, &format!("Gifts: {} of {}{hearts}", app.gifts.count(), GIFTS.len()), theme.dim);
    }
}

/// A rainbow, rising from the left and coming down on the right as the party goes on.
fn rainbow(buf: &mut Buffer, area: Rect, age: f32, top: u16) {
    const BANDS: [Rgb; 6] = [(240, 80, 90), (255, 160, 60), (255, 220, 70), (100, 200, 110), (80, 160, 240), (160, 110, 230)];
    let (w, h) = (area.width as i32, area.height as i32 * 2);
    let radius = (w / 2 - 2).min(h - top as i32 * 2 - 2) as f32;
    let thick = (radius / 16.0).max(1.0);
    let mut picture = Picture::new(w, h);
    let shown = (age / 1.6).min(1.0) * std::f32::consts::PI;
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f32 - w as f32 / 2.0, (h - y) as f32);
            let far = (dx * dx + dy * dy).sqrt();
            let band = ((radius - far) / thick).floor();
            // From straight left, over the top, to straight right.
            if (0.0..BANDS.len() as f32).contains(&band) && dy.atan2(-dx) <= shown {
                picture.set(x, y, BANDS[band as usize]);
            }
        }
    }
    picture.blit(buf, area.x as i32, area.y as i32 * 2);
}

/// A game is finished: the headline, the gift, and what next.
fn party(buf: &mut Buffer, app: &mut App, area: Rect) {
    let theme = app.theme();
    let Some(party) = &app.party else { return };
    let (headline, gift, fun, age) = (party.headline, party.gift, party.fun, party.age);
    // She is as big as the window allows; the words share what is left above her.
    let buttons_high = if area.height >= 50 { 5 } else { 3 };
    let size = kitty_size(area.width, area.height.saturating_sub(buttons_high + 8));
    let spare = area.height - buttons_high - 1 - size.cells().1;
    let gift_high = if spare >= 14 { line_rows(1) } else { 1 };
    let head_high = (spare - gift_high).min(17);
    if fun == Fun::Rainbow && app.animations {
        rainbow(buf, area, age, head_high + gift_high);
    }
    // Each letter of the headline in its own color.
    letters(buf, Rect::new(area.x + 1, area.y, area.width - 2, head_high), headline, 3, &|i, c| (c, theme.buttons[i % theme.buttons.len()]));
    let present = match gift {
        Some(gift) => format!("A gift for Pink Kitty: {}!", GIFTS[gift].name),
        None => format!("A heart for you! You have {}.", app.gifts.hearts),
    };
    label(buf, Rect::new(area.x + 1, area.y + head_high, area.width - 2, gift_high), &present, theme.text, usize::from(gift_high > 1));

    let middle = Rect::new(area.x, area.y + head_high + gift_high, area.width, area.height - head_high - gift_high - buttons_high - 1);
    kitty(buf, app, middle, size, 0.0);
    let row = Rect::new(area.x + 1, area.bottom() - buttons_high - 1, area.width - 2, buttons_high);
    let choices =
        [("Play again", Action::Again, theme.buttons[4]), ("More games", Action::Back, theme.buttons[1]), ("Dress up", Action::Dress, theme.buttons[0])];
    for (r, (words, action, bg)) in across(row, 3).into_iter().zip(choices) {
        worded(buf, app, r, Some(action), bg, words, 1);
    }
}

/// The dressing room: her gifts, to put on and take off.
fn dress(buf: &mut Buffer, app: &mut App, area: Rect) {
    let theme = app.theme();
    worded(buf, app, Rect::new(area.x, area.y, 12, 1), Some(Action::Back), theme.panel, "Esc Back", 0);
    centered(buf, area, area.y, "Dress up", theme.text);
    let stage = stage(Rect::new(area.x, area.y + 1, area.width, area.height - 1), 2);
    kitty(buf, app, stage.kitty, stage.size, 0.0);
    let high = bubble_rows(stage.side.height);
    bubble(buf, Rect::new(stage.side.x, stage.side.y, stage.side.width, high), &app.bubble, theme);
    let rest = stage.side.height - high - 1;
    let gift_high = (rest / 4).clamp(1, 5);
    for (i, gift) in GIFTS.iter().enumerate() {
        let row = Rect::new(stage.side.x, stage.side.y + high + 1 + (i as u16 / 3) * gift_high, stage.side.width, gift_high - u16::from(gift_high > 2));
        let r = across(row, 3)[i % 3];
        if !app.gifts.earned[i] {
            worded(buf, app, r, None, mix(theme.panel, theme.bg, 0.5), "?", 0);
        } else if app.gifts.worn[i] {
            // The tick has a place of its own, so that the longest name still fits.
            let bg = theme.buttons[i % theme.buttons.len()];
            worded(buf, app, r, Some(Action::Wear(i)), bg, gift.name, 0);
            put(buf, r.x + 1, r.y + r.height / 2, "✓", ink(bg), 1);
        } else {
            worded(buf, app, r, Some(Action::Wear(i)), theme.panel, gift.name, 0);
        }
    }
    let hearts = if app.gifts.hearts > 0 { format!("   Hearts: {}", app.gifts.hearts) } else { String::new() };
    centered(buf, stage.below, stage.below.y + 1, &format!("Gifts: {} of {}. Every game wins one.{hearts}", app.gifts.count(), GIFTS.len()), theme.dim);
}

/// The opening and the goodbye: she comes up from the bottom, or waves, under words.
fn greeting(buf: &mut Buffer, app: &mut App, area: Rect, words: &str, rise: f32) {
    // She is as big as the window allows; the words have what is left above her.
    let size = kitty_size(area.width, area.height.saturating_sub(7));
    let head_high = area.height - size.cells().1 - 1;
    let theme = app.theme();
    let head = Rect::new(area.x + 1, area.y, area.width - 2, head_high);
    if words == TITLE {
        title(buf, head, app, 3);
    } else {
        letters(buf, head, words, 3, &|i, c| (c, theme.buttons[i % theme.buttons.len()]));
    }
    let rest = Rect::new(area.x, area.y + head_high, area.width, area.height - head_high - 1);
    kitty(buf, app, rest, size, rise);
    if words == TITLE {
        centered(buf, area, area.bottom() - 1, "Press any key or click to start", theme.dim);
    }
    app.buttons.push((area, Action::Skip));
}

const HELP: [&str; 15] = [
    "Fun time with Pink Kitty",
    "",
    "Everything can be clicked. With the keyboard:",
    "",
    "Arrows or H J K L, then Enter    choose a button",
    "1 2 3 4    the games      5 dress up      6 surprise",
    "E    easy, medium or hard",
    "T theme      S sound      M motion      Q quit",
    "",
    "In a game, type the letters. A wrong key costs nothing.",
    "Tab    Pink Kitty says it again",
    "A B C  or  1 2 3    the words of story time (on Easy, the letter)",
    "Esc    back to the home screen",
    "",
    "Every game wins a gift for Pink Kitty to wear.",
];

fn help(buf: &mut Buffer, app: &mut App, area: Rect) {
    let theme = app.theme();
    let wide = (HELP.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16 + 6).min(area.width);
    let high = (HELP.len() as u16 + 2).min(area.height);
    let r = Rect::new(area.x + (area.width - wide) / 2, area.y + (area.height - high) / 2, wide, high);
    app.buttons.push((area, Action::Help));
    fill(buf, r, theme.panel);
    frame(buf, r, theme.text);
    for (i, line) in HELP.iter().enumerate().take(high as usize - 2) {
        centered(buf, Rect::new(r.x + 2, r.y, r.width - 4, r.height), r.y + 1 + i as u16, line, if i == 0 { theme.accent } else { theme.text });
    }
}

/// What has been thrown in the air: behind everything, or in front of it. Either way
/// only on empty cells with empty cells beside them, so that nothing covers a word or
/// a picture, or sits in the space between two words and reads as part of them.
fn particles(buf: &mut Buffer, app: &App, front: bool) {
    let dim = app.theme().dim;
    // As the screen is before any of them is drawn: they may sit beside each other.
    let taken: Vec<bool> = buf.content.iter().map(|cell| cell.symbol() != " ").collect();
    let wide = buf.area.width as usize;
    let free = |x: usize, y: usize| x < wide && taken.get(y * wide + x) == Some(&false);
    for p in app.particles.iter().filter(|p| p.front == front) {
        let rows: &[&str] = if p.symbol == BALLOON { &["▄███▄", "▀███▀", "  │  "] } else { std::slice::from_ref(&p.symbol) };
        for (j, row) in rows.iter().enumerate() {
            for (i, symbol) in row.chars().enumerate().filter(|&(_, c)| c != ' ') {
                let (x, y) = (p.x + i as f32, p.y + j as f32);
                if x < 0.0 || y < 0.0 {
                    continue;
                }
                let (cx, cy) = (x as usize, y as usize);
                if free(cx, cy)
                    && (cx == 0 || free(cx - 1, cy))
                    && (cx + 1 == wide || free(cx + 1, cy))
                    && let Some(cell) = buf.cell_mut((x as u16, y as u16))
                {
                    cell.set_char(symbol).set_fg(color(if symbol == '│' { dim } else { p.color }));
                }
            }
        }
    }
}

/// The nearest of the 256 colors every terminal has, for those without all of them.
fn indexed(c: Color) -> Color {
    let Color::Rgb(r, g, b) = c else { return c };
    let step = |v: u8| (v as u16 * 5 + 127) / 255;
    Color::Indexed((16 + 36 * step(r) + 6 * step(g) + step(b)) as u8)
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let buf = frame.buffer_mut();
    let theme = app.theme();
    app.buttons.clear();
    app.size = (area.width, area.height);
    fill(buf, area, theme.bg);
    if area.width < MIN.0 || area.height < MIN.1 {
        label(buf, area, &format!("Please make the window bigger: at least {} by {}.", MIN.0, MIN.1), theme.text, 0);
    } else {
        particles(buf, app, false);
        match app.screen {
            Screen::Intro => {
                // She comes up from below the window in the first moments.
                let rise = (1.0 - app.phase_time / 0.9).max(0.0).powi(2);
                greeting(buf, app, area, TITLE, rise);
            }
            Screen::Bye => greeting(buf, app, area, "Bye bye!", 0.0),
            Screen::Home => home(buf, app, area),
            Screen::Play => play(buf, app, area),
            Screen::Party => party(buf, app, area),
            Screen::Dress => dress(buf, app, area),
        }
        particles(buf, app, true);
        if app.help {
            help(buf, app, area);
        }
    }
    if !app.truecolor {
        for cell in &mut buf.content {
            cell.fg = indexed(cell.fg);
            cell.bg = indexed(cell.bg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Gifts;
    use crate::theme::{Settings, THEMES};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    fn app() -> App {
        App::new(true, Settings::default(), Gifts::default())
    }

    /// Draws the app in a window of this size and returns its lines of text.
    fn screen(app: &mut App, w: u16, h: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let buf = terminal.backend().buffer();
        (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect()).collect()
    }

    fn has(lines: &[String], text: &str) -> bool {
        lines.iter().any(|l| l.contains(text))
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    /// Clicks in the middle of the button that does this.
    fn click(app: &mut App, action: Action) {
        let &(rect, _) = app.buttons.iter().rev().find(|(_, a)| *a == action).unwrap_or_else(|| panic!("no button for {action:?}"));
        let (column, row) = (rect.x + rect.width / 2, rect.y + rect.height / 2);
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, modifiers: KeyModifiers::NONE });
        app.on_mouse(MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, modifiers: KeyModifiers::NONE });
    }

    /// Plays story time to its party, by trying every word of every story.
    fn to_the_party(app: &mut App) {
        app.start(Game::Story);
        for _ in 0..3 {
            for option in 0..3 {
                app.act(Action::Pick(option));
            }
            app.act(Action::Next);
        }
        assert_eq!(app.screen, Screen::Party);
    }

    /// Every state worth drawing: each screen, each game at each level at its start
    /// and part of the way through, each kind of party.
    fn states() -> Vec<(String, App)> {
        let mut states = vec![("home".to_string(), app())];
        let mut intro = app();
        intro.begin(true);
        intro.advance(1.5);
        states.push(("intro".into(), intro));
        let mut bye = app();
        bye.act(Action::Quit);
        states.push(("bye".into(), bye));
        let mut help = app();
        help.act(Action::Help);
        states.push(("help".into(), help));
        let mut dress = app();
        dress.act(Action::Dress);
        states.push(("dress, nothing yet".into(), dress));
        let mut dress = app();
        dress.gifts = Gifts { earned: [true; GIFTS.len()], worn: [true; GIFTS.len()], hearts: 3 };
        dress.act(Action::Dress);
        states.push(("dress, everything".into(), dress));
        for level in Level::ALL {
            for game in Game::ALL {
                let mut start = app();
                start.level = level;
                start.start(game);
                states.push((format!("{} at {}, start", game.name(), level.name()), start));
                let mut later = app();
                later.level = level;
                later.start(game);
                match &later.task.as_ref().unwrap().play {
                    Play::Typing(typing) => {
                        // A wrong key, then all of it, and the treat on its way.
                        let item = typing.items[0].clone();
                        later.act(Action::Key(if item.starts_with('Z') { 'Q' } else { 'Z' }));
                        for c in item.chars() {
                            later.act(Action::Key(c));
                        }
                        later.advance(0.2);
                    }
                    Play::Yarn(_) => {
                        for _ in 0..30 {
                            later.advance(0.3);
                        }
                    }
                    Play::Quiz(quiz) => {
                        let right = quiz.items[0].right;
                        later.act(Action::Pick((right + 1) % 3));
                        later.act(Action::Pick(right));
                        later.advance(0.2);
                    }
                }
                states.push((format!("{} at {}, later", game.name(), level.name()), later));
            }
        }
        for fun in Fun::ALL {
            let mut party = app();
            to_the_party(&mut party);
            party.party.as_mut().unwrap().fun = fun;
            for _ in 0..20 {
                party.advance(0.1);
            }
            states.push((format!("party with {fun:?}"), party));
        }
        // The party when there are no gifts left to win.
        let mut hearts = app();
        hearts.gifts.earned = [true; GIFTS.len()];
        to_the_party(&mut hearts);
        states.push(("party for a heart".into(), hearts));
        states
    }

    const SIZES: [(u16, u16); 10] = [(80, 24), (81, 25), (100, 30), (120, 40), (140, 42), (160, 50), (230, 70), (300, 90), (80, 90), (300, 24)];

    #[test]
    fn every_screen_draws_at_every_size_in_every_theme() {
        for (name, mut app) in states() {
            for (i, (w, h)) in SIZES.into_iter().enumerate() {
                // Every theme is met at some size, and the first at all of them.
                for theme in [0, 1 + i % (THEMES.len() - 1)] {
                    app.theme = theme;
                    let lines = screen(&mut app, w, h);
                    assert!(!has(&lines, "Please make the window bigger"), "{name} at {w}x{h}");
                    assert!(!app.buttons.is_empty(), "{name} at {w}x{h}");
                    for &(rect, action) in &app.buttons {
                        assert!(rect.width > 0 && rect.height > 0 && rect.right() <= w && rect.bottom() <= h, "{name} at {w}x{h}: {action:?} is at {rect:?}");
                    }
                    assert!(app.kitty.width > 0 && app.kitty.right() <= w && app.kitty.bottom() <= h, "{name} at {w}x{h}: she is at {:?}", app.kitty);
                    // No two things to click lie one on top of the other, the help and
                    // the opening aside, which cover everything.
                    let solid: Vec<Rect> = app.buttons.iter().filter(|(_, a)| !matches!(a, Action::Help | Action::Skip)).map(|&(r, _)| r).collect();
                    if !app.help && !name.contains("Yarn") {
                        for (i, a) in solid.iter().enumerate() {
                            assert!(solid[..i].iter().all(|b| !a.intersects(*b)), "{name} at {w}x{h}: buttons overlap at {a:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_home_screen_shows_everything_in_the_smallest_window() {
        let mut app = app();
        app.gifts.hearts = 2;
        let lines = screen(&mut app, MIN.0, MIN.1);
        for words in [TITLE, "What shall we do?", "1 Snack time", "2 Kitty says", "3 Yarn balls", "4 Story time", "5 Dress up", "6 Surprise"] {
            assert!(has(&lines, words), "no {words:?} in {lines:#?}");
        }
        for words in ["Easy (4-5)", "Medium (6-7)", "Hard (8-10)", "T candy", "S Sound on", "M Motion on", "? Help", "Q Quit", "Gifts: 0 of 12   Hearts: 2"] {
            assert!(has(&lines, words), "no {words:?} in {lines:#?}");
        }
        // She is there, in half blocks.
        assert!(lines.iter().filter(|l| l.contains('▀')).count() >= 12);
        // In a big window the title is in big letters, which are not text.
        let lines = screen(&mut app, 160, 50);
        assert!(!has(&lines, TITLE) && has(&lines, "T Theme: candy"));
    }

    #[test]
    fn the_arrows_reach_every_button_and_enter_presses_it() {
        for (w, h) in SIZES {
            let mut app = app();
            screen(&mut app, w, h);
            let all: Vec<Action> = app.buttons.iter().map(|&(_, action)| action).collect();
            assert_eq!(all.len(), 14, "{w}x{h}");
            let mut reached = vec![app.marker.unwrap()];
            let mut next = 0;
            while next < reached.len() {
                for code in
                    [KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down, KeyCode::Char('h'), KeyCode::Char('j'), KeyCode::Char('k'), KeyCode::Char('l')]
                {
                    app.marker = Some(reached[next]);
                    press(&mut app, code);
                    let marker = app.marker.unwrap();
                    if !reached.contains(&marker) {
                        reached.push(marker);
                    }
                }
                next += 1;
            }
            for action in &all {
                assert!(reached.contains(action), "{w}x{h}: the arrows never reach {action:?}");
            }
            // Left of "Snack time" is nothing; right of it is "Kitty says", and under
            // it "Yarn balls".
            app.marker = Some(Action::Play(Game::Snack));
            press(&mut app, KeyCode::Left);
            assert_eq!(app.marker, Some(Action::Play(Game::Snack)), "{w}x{h}");
            press(&mut app, KeyCode::Right);
            assert_eq!(app.marker, Some(Action::Play(Game::Says)), "{w}x{h}");
            press(&mut app, KeyCode::Left);
            press(&mut app, KeyCode::Down);
            assert_eq!(app.marker, Some(Action::Play(Game::Yarn)), "{w}x{h}");
            press(&mut app, KeyCode::Enter);
            assert_eq!(app.task.as_ref().map(|task| task.game), Some(Game::Yarn));
        }
    }

    #[test]
    fn the_mouse_does_everything_the_keys_do() {
        let mut app = app();
        screen(&mut app, 100, 30);
        // The marker follows the pointer.
        let &(rect, _) = app.buttons.iter().find(|(_, a)| *a == Action::Dress).unwrap();
        app.on_mouse(MouseEvent { kind: MouseEventKind::Moved, column: rect.x + 1, row: rect.y, modifiers: KeyModifiers::NONE });
        assert_eq!(app.marker, Some(Action::Dress));
        click(&mut app, Action::Level(Level::Medium));
        assert_eq!(app.level, Level::Medium);
        click(&mut app, Action::Theme);
        assert_eq!(app.theme, 1);

        // Snack time, on the keyboard that is drawn on the screen.
        click(&mut app, Action::Play(Game::Snack));
        for _ in 0..5 {
            screen(&mut app, 100, 30);
            let Some(Play::Typing(typing)) = app.task.as_ref().map(|task| &task.play) else { panic!() };
            let word = typing.items[typing.at].clone();
            for c in word.chars() {
                click(&mut app, Action::Key(c));
            }
            app.advance(2.0);
        }
        assert_eq!(app.screen, Screen::Party);
        screen(&mut app, 100, 30);
        click(&mut app, Action::Dress);
        assert_eq!(app.screen, Screen::Dress);
        screen(&mut app, 100, 30);
        let gift = app.gifts.earned.iter().position(|&e| e).unwrap();
        click(&mut app, Action::Wear(gift));
        assert!(!app.gifts.worn[gift]);
        click(&mut app, Action::Back);
        assert_eq!(app.screen, Screen::Home);

        // A ball of yarn can be clicked itself.
        app.level = Level::Easy;
        screen(&mut app, 100, 30);
        click(&mut app, Action::Play(Game::Yarn));
        app.advance(0.5);
        app.advance(3.0);
        screen(&mut app, 100, 30);
        let Some(Play::Yarn(yarn)) = app.task.as_ref().map(|task| &task.play) else { panic!() };
        let letter = yarn.balls[0].letter;
        // The ball is the first button with its letter; the key is the last.
        let (ball, key) = (
            app.buttons.iter().find(|(_, a)| *a == Action::Key(letter)).unwrap().0,
            app.buttons.iter().rev().find(|(_, a)| *a == Action::Key(letter)).unwrap().0,
        );
        assert!(ball != key && ball.height >= 4 && key.height == 1);
        let (column, row) = (ball.x + ball.width / 2, ball.y + ball.height / 2);
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, modifiers: KeyModifiers::NONE });
        assert_eq!(app.task.as_ref().unwrap().progress().0, 1);
        assert!(!app.particles.is_empty());

        // Story time, and on with "Next".
        screen(&mut app, 100, 30);
        click(&mut app, Action::Back);
        screen(&mut app, 100, 30);
        click(&mut app, Action::Play(Game::Story));
        for round in 0..3 {
            for option in 0..3 {
                screen(&mut app, 100, 30);
                if app.buttons.iter().any(|(_, a)| *a == Action::Pick(option)) {
                    click(&mut app, Action::Pick(option));
                }
            }
            screen(&mut app, 100, 30);
            assert_eq!(app.task.as_ref().unwrap().progress().0, round + 1);
            // A wrong word that was tried can no longer be clicked.
            assert!(app.buttons.iter().all(|(_, a)| !matches!(a, Action::Pick(_))));
            click(&mut app, Action::Next);
        }
        assert_eq!(app.screen, Screen::Party);
        screen(&mut app, 100, 30);
        click(&mut app, Action::Again);
        assert_eq!(app.screen, Screen::Play);

        // The help goes away at a click anywhere, and the opening too.
        let mut app = self::app();
        screen(&mut app, 100, 30);
        click(&mut app, Action::Help);
        screen(&mut app, 100, 30);
        assert!(app.help);
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 0, row: 0, modifiers: KeyModifiers::NONE });
        assert!(!app.help && app.screen == Screen::Home);
        let mut app = self::app();
        app.begin(true);
        screen(&mut app, 100, 30);
        // A terminal that only reports the button coming up still clicks.
        app.on_mouse(MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column: 5, row: 5, modifiers: KeyModifiers::NONE });
        assert_eq!(app.screen, Screen::Home);
    }

    #[test]
    fn enter_presses_next_and_the_party_buttons() {
        let mut app = app();
        app.level = Level::Medium;
        app.start(Game::Story);
        for _ in 0..3 {
            screen(&mut app, 120, 40);
            assert_eq!(app.marker, Some(Action::Pick(0)));
            // Along the three words with the arrows, trying each.
            for _ in 0..3 {
                press(&mut app, KeyCode::Enter);
                screen(&mut app, 120, 40);
                if app.marker == Some(Action::Next) {
                    break;
                }
            }
            assert_eq!(app.marker, Some(Action::Next));
            press(&mut app, KeyCode::Char(' '));
        }
        assert_eq!((app.screen, app.marker), (Screen::Party, Some(Action::Again)));
        let lines = screen(&mut app, 120, 40);
        assert!(has(&lines, "Play again") && has(&lines, "More games") && has(&lines, "Dress up") && has(&lines, "A gift for Pink Kitty: "));
        press(&mut app, KeyCode::Right);
        assert_eq!(app.marker, Some(Action::Back));
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Dress);
        let lines = screen(&mut app, 120, 40);
        assert!(has(&lines, "Gifts: 1 of 12") && has(&lines, "✓") && has(&lines, "?"));
        // Every name can be read whole in the smallest window, marked or not.
        app.gifts.earned = [true; GIFTS.len()];
        for (i, gift) in GIFTS.iter().enumerate() {
            app.marker = Some(Action::Wear(i));
            let lines = screen(&mut app, MIN.0, MIN.1);
            assert!(GIFTS.iter().all(|other| has(&lines, other.name)), "{} marked: {lines:#?}", gift.name);
        }
        // Enter does nothing with a marker that is not on this screen.
        app.marker = Some(Action::Quit);
        press(&mut app, KeyCode::Enter);
        assert!(!app.quit && app.screen == Screen::Dress);
    }

    #[test]
    fn what_is_to_be_typed_is_big_when_there_is_room_and_plain_when_not() {
        let mut app = app();
        app.level = Level::Hard;
        app.start(Game::Snack);
        let Some(Play::Typing(typing)) = app.task.as_ref().map(|task| &task.play) else { panic!() };
        let sentence = typing.items[0].clone();
        let lines = screen(&mut app, 160, 50);
        assert!(!has(&lines, &sentence) && has(&lines, "space") && has(&lines, "█"));

        // A hidden word is drawn as blanks, in big letters or plain.
        let dir = std::env::temp_dir().join(format!("funkitty-ui-test-{}", std::process::id()));
        app.speaker = crate::sound::Speaker::through(dir.join("no-such-player"), &[], dir.join("sounds"));
        app.start(Game::Says);
        assert!(app.concealed());
        let Some(Play::Typing(typing)) = app.task.as_ref().map(|task| &task.play) else { panic!() };
        let word = typing.items[0].clone();
        let spread = |text: &str| text.chars().map(String::from).collect::<Vec<_>>().join(" ");
        // Narrow enough that the word does not fit in big letters beside her.
        let lines = screen(&mut app, 80, 24);
        let blanks = "_".repeat(word.len());
        assert!(has(&lines, &spread(&blanks)) || has(&lines, "▀▀▀▀▀ ▀▀▀▀▀"), "{lines:#?}");
        assert!(!has(&lines, &spread(&word)) && !has(&lines, &word));
        assert!(has(&lines, "Tab Say again") && has(&lines, "Listen, and type what I say."));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_wanted_key_is_lit_on_the_keyboard() {
        let mut app = app();
        app.start(Game::Says);
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let wanted = app.wanted().unwrap();
        let buf = terminal.backend().buffer();
        let accent = color(app.theme().accent);
        for &(rect, action) in &app.buttons {
            if let Action::Key(key) = action {
                assert_eq!(buf[(rect.x, rect.y)].bg == accent, key == wanted, "{key}");
            }
        }
        // 26 letters; no space bar when there is no space to type.
        assert_eq!(app.buttons.iter().filter(|(_, a)| matches!(a, Action::Key(_))).count(), 26);
    }

    #[test]
    fn too_small_a_window_says_so_and_fewer_colors_are_used_when_asked() {
        let mut app = app();
        let lines = screen(&mut app, MIN.0 - 1, MIN.1);
        assert!(has(&lines, "Please make the window bigger") && app.buttons.is_empty());
        let lines = screen(&mut app, MIN.0, MIN.1 - 1);
        assert!(has(&lines, "at least 80 by 24"));
        screen(&mut app, 3, 2);

        app.truecolor = false;
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        assert!(terminal.backend().buffer().content.iter().all(|cell| !matches!(cell.fg, Color::Rgb(..)) && !matches!(cell.bg, Color::Rgb(..))));
        assert_eq!(indexed(Color::Rgb(255, 255, 255)), Color::Indexed(231));
    }

    #[test]
    fn balloons_and_fish_are_drawn_whole_and_nothing_covers_a_word() {
        let mut app = app();
        to_the_party(&mut app);
        app.party.as_mut().unwrap().fun = Fun::Balloons;
        for _ in 0..30 {
            app.advance(0.1);
        }
        let lines = screen(&mut app, 120, 40);
        assert!(has(&lines, "▄███▄") && has(&lines, "Play again"));
        app.particles.clear();
        app.party.as_mut().unwrap().fun = Fun::Fish;
        for _ in 0..30 {
            app.advance(0.1);
        }
        let lines = screen(&mut app, 120, 40);
        assert!(has(&lines, "><>") && has(&lines, "More games"));

        // Not even in the space between two words.
        let mut app = self::app();
        app.start(Game::Says);
        screen(&mut app, 100, 30);
        let &(back, _) = app.buttons.iter().find(|(_, a)| *a == Action::Back).unwrap();
        for kind in 0..40 {
            app.particles.clear();
            for _ in 0..60 {
                app.throw_at(back.x as f32 + (kind % 12) as f32, back.y as f32);
            }
            let lines = screen(&mut app, 100, 30);
            assert!(has(&lines, "Esc Back") && has(&lines, "Tab Say again"), "{}", lines[0]);
        }
    }
}
