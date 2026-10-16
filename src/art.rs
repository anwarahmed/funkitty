//! Pictures of things, read from `data/pictures.txt` (a cat, a boat, a cake: what
//! picture words, pairs and "What's next?" show) and `data/things.txt` (little ones,
//! for counting). Each is drawn with a dark line round it.

use std::sync::OnceLock;

use crate::picture::Picture;
use crate::theme::Rgb;

/// The dark line round every picture, as round the treats.
pub const LINE: Rgb = (96, 72, 96);

pub struct Art {
    pub name: &'static str,
    rows: Vec<&'static str>,
    colors: Vec<(u8, Rgb)>,
}

impl Art {
    /// The picture, `times` its size, with its line round it.
    pub fn picture(&self, times: i32) -> Picture {
        let mut picture = Picture::sprite(&self.rows, &self.colors);
        picture.outline(LINE);
        picture.scaled(times.max(1))
    }

    /// The same, pale and grey: one that has gone away, in a sum that takes away.
    pub fn faded(&self, times: i32) -> Picture {
        let pale = |(r, g, b): Rgb| {
            let v = ((r as u16 + g as u16 + b as u16) / 3 + 2 * 235) / 3;
            (v as u8, v as u8, v as u8)
        };
        let colors: Vec<(u8, Rgb)> = self.colors.iter().map(|&(key, c)| (key, pale(c))).collect();
        let mut picture = Picture::sprite(&self.rows, &colors);
        picture.outline((175, 168, 180));
        picture.scaled(times.max(1))
    }
}

fn parse(text: &'static str) -> Vec<Art> {
    let mut colors: Vec<(u8, Rgb)> = Vec::new();
    let mut all: Vec<Art> = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')) {
        if let Some(color) = line.strip_prefix(':') {
            let parts: Vec<&str> = color.split_whitespace().collect();
            if let [key, r, g, b] = parts[..]
                && let (Ok(r), Ok(g), Ok(b)) = (r.parse(), g.parse(), b.parse())
            {
                colors.push((key.as_bytes()[0], (r, g, b)));
            }
        } else if let Some(name) = line.strip_prefix('=') {
            all.push(Art { name: name.trim(), rows: Vec::new(), colors: colors.clone() });
        } else if let Some(art) = all.last_mut() {
            art.rows.push(line);
        }
    }
    all
}

/// The pictures of picture words.
pub fn pictures() -> &'static [Art] {
    static ALL: OnceLock<Vec<Art>> = OnceLock::new();
    ALL.get_or_init(|| parse(include_str!("../data/pictures.txt")))
}

/// The little things that are counted.
pub fn things() -> &'static [Art] {
    static ALL: OnceLock<Vec<Art>> = OnceLock::new();
    ALL.get_or_init(|| parse(include_str!("../data/things.txt")))
}

/// The picture of this name, big or little.
pub fn find(name: &str) -> Option<&'static Art> {
    pictures().iter().chain(things()).find(|art| art.name == name)
}

/// The names of the pictures.
pub fn names() -> Vec<&'static str> {
    pictures().iter().map(|art| art.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_picture_is_whole_named_once_and_small_enough() {
        for (all, most, least) in [(pictures(), (15, 13), 18), (things(), (10, 10), 6)] {
            assert!(all.len() >= least);
            let names: HashSet<&str> = all.iter().map(|art| art.name).collect();
            assert_eq!(names.len(), all.len());
            for art in all {
                assert!(art.name.bytes().all(|b| b.is_ascii_lowercase()) && !art.rows.is_empty(), "{}", art.name);
                // Every character is a color or nothing.
                assert!(art.rows.iter().all(|row| row.bytes().all(|b| b == b'.' || art.colors.iter().any(|&(key, _)| key == b))), "{}", art.name);
                // Three across beside her in the smallest window, and the line round them.
                let picture = art.picture(1);
                assert!(picture.w <= most.0 && picture.h <= most.1, "{} is {}x{}", art.name, picture.w, picture.h);
                assert_eq!(art.faded(2).w, picture.w * 2);
            }
        }
        assert!(find("cat").is_some() && find("balloon").is_some() && find("dragon").is_none());
    }
}
