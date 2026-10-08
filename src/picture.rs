//! Pictures made of square pixels, two to a terminal cell (`▀` with a color above and
//! a color below). A pixel can be empty, and then whatever is on the screen under it
//! stays, so a picture has any outline and can be put over another.

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

use crate::theme::Rgb;

#[derive(Clone, PartialEq, Debug)]
pub struct Picture {
    pub w: i32,
    pub h: i32,
    px: Vec<Option<Rgb>>,
}

impl Picture {
    pub fn new(w: i32, h: i32) -> Picture {
        Picture { w, h, px: vec![None; (w * h).max(0) as usize] }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        if x >= 0 && y >= 0 && x < self.w && y < self.h { self.px[(y * self.w + x) as usize] } else { None }
    }

    pub fn set(&mut self, x: i32, y: i32, c: Rgb) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = Some(c);
        }
    }

    /// Draws a line around everything: each empty pixel beside a filled one.
    pub fn outline(&mut self, line: Rgb) {
        let filled = self.px.clone();
        let is = |x: i32, y: i32| x >= 0 && y >= 0 && x < self.w && y < self.h && filled[(y * self.w + x) as usize].is_some();
        for y in 0..self.h {
            for x in 0..self.w {
                if !is(x, y) && (is(x + 1, y) || is(x - 1, y) || is(x, y + 1) || is(x, y - 1)) {
                    self.px[(y * self.w + x) as usize] = Some(line);
                }
            }
        }
    }

    /// A picture from rows of characters, each a color of `palette` or nothing, with
    /// a pixel of room all round for an outline.
    pub fn sprite(rows: &[&str], palette: &[(u8, Rgb)]) -> Picture {
        let wide = rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32;
        let mut picture = Picture::new(wide + 2, rows.len() as i32 + 2);
        for (j, row) in rows.iter().enumerate() {
            for (i, key) in row.bytes().enumerate() {
                if let Some(&(_, c)) = palette.iter().find(|(k, _)| *k == key) {
                    picture.set(i as i32 + 1, j as i32 + 1, c);
                }
            }
        }
        picture
    }

    /// The same picture with every pixel `times` pixels across.
    pub fn scaled(&self, times: i32) -> Picture {
        let mut big = Picture::new(self.w * times, self.h * times);
        for y in 0..big.h {
            for x in 0..big.w {
                big.px[(y * big.w + x) as usize] = self.get(x / times, y / times);
            }
        }
        big
    }

    /// Cells it is wide, and cells it is tall.
    pub fn cells(&self) -> (u16, u16) {
        (self.w as u16, (self.h as u16).div_ceil(2))
    }

    /// Puts the picture on the screen with its left edge at column `x` and its top at
    /// pixel row `y` (two to a cell row, counted from the top of the screen). Parts
    /// that are off the screen are left out.
    pub fn blit(&self, buf: &mut Buffer, x: i32, y: i32) {
        for row in y.div_euclid(2)..=(y + self.h - 1).div_euclid(2) {
            for px in 0..self.w {
                let (top, bottom) = (self.get(px, row * 2 - y), self.get(px, row * 2 + 1 - y));
                if top.is_none() && bottom.is_none() {
                    continue;
                }
                let (Ok(cx), Ok(cy)) = (u16::try_from(x + px), u16::try_from(row)) else { continue };
                let Some(cell) = buf.cell_mut((cx, cy)) else { continue };
                // What is there now, as two pixels, for the half this picture leaves empty.
                let (under_top, under_bottom) = match cell.symbol() {
                    "▀" => (cell.fg, cell.bg),
                    "▄" => (cell.bg, cell.fg),
                    "█" => (cell.fg, cell.fg),
                    _ => (cell.bg, cell.bg),
                };
                let color = |c: Rgb| Color::Rgb(c.0, c.1, c.2);
                cell.set_symbol("▀");
                cell.fg = top.map_or(under_top, color);
                cell.bg = bottom.map_or(under_bottom, color);
                cell.modifier = Modifier::empty();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    const RED: Rgb = (255, 0, 0);
    const BLUE: Rgb = (0, 0, 255);

    #[test]
    fn a_sprite_has_room_for_its_outline_and_can_be_made_bigger() {
        let mut picture = Picture::sprite(&["R.", ".R"], &[(b'R', RED)]);
        assert_eq!((picture.w, picture.h, picture.cells()), (4, 4, (4, 2)));
        assert_eq!((picture.get(1, 1), picture.get(2, 1), picture.get(2, 2), picture.get(9, 9)), (Some(RED), None, Some(RED), None));
        picture.outline(BLUE);
        assert_eq!((picture.get(1, 0), picture.get(2, 1), picture.get(0, 0), picture.get(1, 1)), (Some(BLUE), Some(BLUE), None, Some(RED)));
        let big = picture.scaled(2);
        assert_eq!((big.w, big.h, big.get(2, 2), big.get(3, 3), big.get(1, 1)), (8, 8, Some(RED), Some(RED), None));
    }

    #[test]
    fn empty_pixels_keep_what_is_under_them_at_any_height() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 3));
        for cell in &mut buf.content {
            cell.bg = Color::Rgb(9, 9, 9);
        }
        let mut dot = Picture::new(1, 1);
        dot.set(0, 0, RED);
        // On an even pixel row it is the top half of a cell, on an odd one the bottom.
        dot.blit(&mut buf, 1, 0);
        dot.blit(&mut buf, 2, 3);
        assert_eq!((buf[(1, 0)].symbol(), buf[(1, 0)].fg, buf[(1, 0)].bg), ("▀", Color::Rgb(255, 0, 0), Color::Rgb(9, 9, 9)));
        assert_eq!((buf[(2, 1)].symbol(), buf[(2, 1)].fg, buf[(2, 1)].bg), ("▀", Color::Rgb(9, 9, 9), Color::Rgb(255, 0, 0)));
        // Over the other half of the same cell, the first half stays.
        let mut other = Picture::new(1, 1);
        other.set(0, 0, BLUE);
        other.blit(&mut buf, 1, 1);
        assert_eq!((buf[(1, 0)].fg, buf[(1, 0)].bg), (Color::Rgb(255, 0, 0), Color::Rgb(0, 0, 255)));
        assert_eq!(buf[(0, 0)].symbol(), " ");
        // Off every edge: nothing panics.
        let big = Picture::sprite(&["RRRRRR"; 9], &[(b'R', RED)]);
        big.blit(&mut buf, -3, -5);
        big.blit(&mut buf, 2, 3);
    }
}
