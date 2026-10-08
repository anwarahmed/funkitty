//! What the games are made of: the levels, the words, the sentences, the stories and
//! everything Pink Kitty says, all read from the text files in `data/`. Knows nothing
//! about the screen. The small `Rng` lives here too.

use std::sync::OnceLock;

/// Xorshift: all the chance the games need, and the same on every machine.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub fn from_clock() -> Rng {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
        Rng::new(now ^ (std::process::id() as u64) << 32)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A whole number from 0 up to, not including, `n`.
    pub fn below(&mut self, n: usize) -> usize {
        ((self.next() >> 33) as usize) % n.max(1)
    }

    /// From 0 up to 1.
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// From `low` up to `high`.
    pub fn between(&mut self, low: f32, high: f32) -> f32 {
        low + self.unit() * (high - low)
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i + 1));
        }
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

/// How much reading and typing a game asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    /// One letter at a time: for a child who is just meeting the keyboard.
    Easy,
    /// Short words.
    Medium,
    /// Longer words and whole sentences.
    Hard,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Easy, Level::Medium, Level::Hard];

    pub fn name(self) -> &'static str {
        match self {
            Level::Easy => "Easy",
            Level::Medium => "Medium",
            Level::Hard => "Hard",
        }
    }

    /// The ages it is meant for.
    pub fn ages(self) -> &'static str {
        match self {
            Level::Easy => "4-5",
            Level::Medium => "6-7",
            Level::Hard => "8-10",
        }
    }

    pub fn find(name: &str) -> Option<Level> {
        Level::ALL.into_iter().find(|l| l.name().eq_ignore_ascii_case(name))
    }
}

/// The lines of a data file that are not comments or empty.
fn rows(text: &'static str) -> impl Iterator<Item = &'static str> {
    text.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// Words of three and four letters.
pub fn short_words() -> &'static [&'static str] {
    static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| rows(include_str!("../data/short-words.txt")).flat_map(str::split_whitespace).collect())
}

/// Words of five letters and more.
pub fn long_words() -> &'static [&'static str] {
    static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| rows(include_str!("../data/long-words.txt")).flat_map(str::split_whitespace).collect())
}

pub fn sentences() -> &'static [&'static str] {
    static SENTENCES: OnceLock<Vec<&'static str>> = OnceLock::new();
    SENTENCES.get_or_init(|| rows(include_str!("../data/sentences.txt")).collect())
}

/// Where the missing word of a story goes.
pub const BLANK: &str = "___";

/// A sentence with a word missing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Story {
    pub level: Level,
    /// The sentence, with `BLANK` for the missing word.
    pub text: &'static str,
    /// The right word and two wrong ones.
    pub options: [&'static str; 3],
}

impl Story {
    /// The sentence with its word in place.
    pub fn whole(&self) -> String {
        self.text.replace(BLANK, self.options[0])
    }

    /// The name of the clip in which Pink Kitty reads it. It is made of the words, so
    /// that a story that is changed asks for a new clip instead of keeping the old one.
    pub fn voice(&self) -> String {
        format!("story-{}", slug(&self.whole()))
    }
}

fn story(line: &'static str) -> Option<Story> {
    let mut parts = line.split('|').map(str::trim);
    let level = match parts.next()? {
        "2" => Level::Medium,
        "3" => Level::Hard,
        _ => return None,
    };
    let text = parts.next().filter(|text| text.matches(BLANK).count() == 1)?;
    let options = [parts.next()?, parts.next()?, parts.next()?];
    (parts.next().is_none() && options.iter().all(|o| !o.is_empty())).then_some(Story { level, text, options })
}

pub fn stories() -> &'static [Story] {
    static STORIES: OnceLock<Vec<Story>> = OnceLock::new();
    STORIES.get_or_init(|| rows(include_str!("../data/stories.txt")).filter_map(story).collect())
}

/// Something Pink Kitty says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Phrase {
    pub id: &'static str,
    /// What her speech bubble shows.
    pub shown: &'static str,
    /// What is spoken: the same, unless the spelling would be read out wrongly.
    pub spoken: &'static str,
}

fn phrase(line: &'static str) -> Option<Phrase> {
    let mut parts = line.split('|').map(str::trim);
    let (id, shown) = (parts.next()?, parts.next()?);
    let spoken = parts.next().unwrap_or(shown);
    (parts.next().is_none() && !id.is_empty() && !shown.is_empty()).then_some(Phrase { id, shown, spoken })
}

pub fn phrases() -> &'static [Phrase] {
    static PHRASES: OnceLock<Vec<Phrase>> = OnceLock::new();
    PHRASES.get_or_init(|| rows(include_str!("../data/phrases.txt")).filter_map(phrase).collect())
}

/// What the speech bubble shows for a phrase. Nothing, for one there is not.
pub fn shown(id: &str) -> &'static str {
    phrases().iter().find(|p| p.id == id).map_or("", |p| p.shown)
}

/// The phrases of a group: `praise` is `praise-1`, `praise-2` and so on.
pub fn group(name: &str) -> Vec<&'static str> {
    let numbered = |id: &str| id.strip_prefix(name).and_then(|rest| rest.strip_prefix('-')).is_some_and(|n| n.parse::<u32>().is_ok());
    phrases().iter().map(|p| p.id).filter(|id| numbered(id)).collect()
}

/// Letters and digits in lower case, with a dash wherever anything else was.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() && c != '\'' {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// The name of the clip that says a letter's name.
pub fn letter_voice(letter: char) -> String {
    format!("letter-{}", letter.to_ascii_lowercase())
}

/// The name of the clip that says a word.
pub fn word_voice(word: &str) -> String {
    format!("word-{}", word.to_ascii_lowercase())
}

/// Every clip the game can ask for: its name and the text to be spoken. This is what
/// `funkitty voice-lines` prints and `tools/voice.py` makes the clips from.
pub fn spoken() -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = Vec::new();
    all.extend(('A'..='Z').map(|c| (letter_voice(c), format!("{c}."))));
    all.extend(short_words().iter().chain(long_words()).map(|w| (word_voice(w), format!("{w}."))));
    all.extend(phrases().iter().map(|p| (p.id.to_string(), p.spoken.to_string())));
    all.extend(stories().iter().map(|s| (s.voice(), s.whole())));
    all.sort();
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_words_are_plain_and_the_right_length_and_none_is_there_twice() {
        let (short, long) = (short_words(), long_words());
        assert!(short.len() >= 100 && long.len() >= 100, "{} short, {} long", short.len(), long.len());
        for word in short {
            assert!((3..=4).contains(&word.len()) && word.bytes().all(|b| b.is_ascii_lowercase()), "{word}");
        }
        for word in long {
            assert!((5..=9).contains(&word.len()) && word.bytes().all(|b| b.is_ascii_lowercase()), "{word}");
        }
        let all: HashSet<_> = short.iter().chain(long).collect();
        assert_eq!(all.len(), short.len() + long.len());
        // Every letter starts some short word but X, so Easy's story time can ask for any other.
        let firsts: HashSet<u8> = short.iter().map(|w| w.as_bytes()[0]).collect();
        assert!(firsts.len() >= 20, "{}", firsts.len());
    }

    #[test]
    fn the_sentences_are_letters_and_single_spaces() {
        let sentences = sentences();
        assert!(sentences.len() >= 30);
        for sentence in sentences {
            assert!(sentence.bytes().all(|b| b.is_ascii_lowercase() || b == b' '), "{sentence}");
            assert!(!sentence.contains("  ") && (2..=5).contains(&sentence.split(' ').count()) && sentence.len() <= 22, "{sentence}");
            assert!(crate::font::supported(sentence));
        }
        assert_eq!(sentences.iter().collect::<HashSet<_>>().len(), sentences.len());
    }

    #[test]
    fn every_story_line_is_a_story_with_three_different_words() {
        let lines = rows(include_str!("../data/stories.txt")).count();
        let stories = stories();
        assert_eq!(stories.len(), lines, "a line of data/stories.txt is not understood");
        for level in [Level::Medium, Level::Hard] {
            assert!(stories.iter().filter(|s| s.level == level).count() >= 20);
        }
        for story in stories {
            let [a, b, c] = story.options;
            assert!(a != b && a != c && b != c, "{story:?}");
            assert!(crate::font::supported(story.text) && story.options.iter().all(|o| crate::font::supported(o)), "{story:?}");
        }
        let voices: HashSet<_> = stories.iter().map(Story::voice).collect();
        assert_eq!(voices.len(), stories.len());
        assert_eq!(story("2|No blank.|a|b|c"), None);
        assert_eq!(story("4|A ___.|a|b|c"), None);
        assert_eq!(story("2|A ___.|a|b"), None);
        assert_eq!(story("2|A ___.|a|b|c").unwrap().whole(), "A a.");
    }

    #[test]
    fn every_phrase_line_is_a_phrase_and_groups_are_found() {
        let lines = rows(include_str!("../data/phrases.txt")).count();
        assert_eq!(phrases().len(), lines, "a line of data/phrases.txt is not understood");
        let ids: HashSet<_> = phrases().iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), lines);
        assert_eq!(shown("praise-1"), "Good job!");
        assert_eq!(shown("nothing"), "");
        assert_eq!(phrases().iter().find(|p| p.id == "praise-8").unwrap().spoken, "Perfect!");
        assert!(group("praise").len() >= 10 && group("party").len() >= 8 && group("yum").len() >= 4);
        // "dress" is a group of its own, apart from "dress-intro" and "dress-empty".
        assert_eq!(group("dress").len(), 4);
        assert!(phrases().iter().all(|p| crate::font::supported(p.shown)));
    }

    #[test]
    fn slugs_and_clip_names() {
        assert_eq!(slug("Hi! I'm Pink Kitty!"), "hi-im-pink-kitty");
        assert_eq!(slug("  A  cat, says: meow. "), "a-cat-says-meow");
        assert_eq!((letter_voice('Q'), word_voice("Cat")), ("letter-q".to_string(), "word-cat".to_string()));
        let all = spoken();
        let names: HashSet<_> = all.iter().map(|(name, _)| name).collect();
        assert_eq!(names.len(), all.len());
        assert!(all.iter().all(|(name, text)| !text.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')));
    }

    #[test]
    fn the_rng_shuffles_and_stays_in_range() {
        let mut rng = Rng::new(7);
        let mut items: Vec<usize> = (0..20).collect();
        rng.shuffle(&mut items);
        assert_ne!(items, (0..20).collect::<Vec<_>>());
        items.sort();
        assert_eq!(items, (0..20).collect::<Vec<_>>());
        for _ in 0..200 {
            assert!(rng.below(5) < 5 && (0.0..1.0).contains(&rng.unit()) && (2.0..3.0).contains(&rng.between(2.0, 3.0)));
        }
    }
}
