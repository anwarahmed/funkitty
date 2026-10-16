//! What the newer games ask, at each level: counting, picture words, big and little
//! letters, "What's next?", and the cards of pairs. Each question is one of
//! `app::Question`, three answers and one of them right, so they are all played the
//! way story time is. Knows nothing about the screen.

use crate::app::{Card, Face, Question, Show, Thing};
use crate::art;
use crate::words::{self, BLANK, Level, Rng};

/// Questions in a game of counting, picture words, big and little, or "What's next?".
pub const QUESTIONS: usize = 5;

/// A question whose answers are these, shuffled, the first of them right.
fn question(rng: &mut Rng, text: String, answers: [String; 3], show: Show) -> Question {
    let mut order = [0, 1, 2];
    rng.shuffle(&mut order);
    Question {
        text,
        options: order.map(|i| answers[i].clone()),
        right: order.iter().position(|&i| i == 0).unwrap_or(0),
        ask: Vec::new(),
        read: Vec::new(),
        show,
        pictures: false,
        keys: None,
        bubble: String::new(),
    }
}

/// Two numbers from `low` to `high`, neither of them `right` nor each other, as near
/// to it as can be: a wrong answer that is miles off teaches nothing.
fn near(rng: &mut Rng, right: u32, low: u32, high: u32) -> [String; 2] {
    let mut others: Vec<u32> = (low..=high).filter(|&n| n != right).collect();
    rng.shuffle(&mut others);
    others.sort_by_key(|&n| n.abs_diff(right));
    others.truncate(3);
    rng.shuffle(&mut others);
    [others[0].to_string(), others[1].to_string()]
}

/// The key that answers a number: its digit, and 0 for ten.
fn digit(answer: &str) -> char {
    if answer == "10" { '0' } else { answer.chars().next().unwrap_or('0') }
}

/// Counting: how many things there are. On Hard, how many there are when some more
/// come, or when some go away.
pub fn counting(rng: &mut Rng, level: Level) -> Vec<Question> {
    let things: Vec<&str> = art::things().iter().map(|art| art.name).collect();
    let mut questions: Vec<Question> = Vec::new();
    for round in 0..QUESTIONS {
        // Not the same thing twice running.
        let fresh: Vec<&str> = things.iter().copied().filter(|&t| !questions.last().is_some_and(|last| shows(last, t))).collect();
        let thing = rng.pick(&fresh);
        let mut q = match level {
            Level::Easy | Level::Medium => {
                let n = if level == Level::Easy { 1 + rng.below(5) as u32 } else { 3 + rng.below(8) as u32 };
                let wrong = if level == Level::Easy { near(rng, n, 1, 5) } else { near(rng, n, 1, 10) };
                let mut q = question(
                    rng,
                    BLANK.to_string(),
                    [n.to_string(), wrong[0].clone(), wrong[1].clone()],
                    Show::Things(vec![Thing::Picture(thing); n as usize]),
                );
                let ask = format!("count-{thing}");
                q.bubble = words::shown(&ask).to_string();
                q.ask = vec![ask];
                q.read = vec![words::number_voice(n)];
                q
            }
            Level::Hard => {
                // Adding and taking away by turns, which comes first by chance.
                let adding = (round + rng.below(2)).is_multiple_of(2);
                let (a, b, answer, shown, sign, said) = if adding {
                    let a = 1 + rng.below(7) as u32;
                    let b = 1 + rng.below((10 - a).min(5) as usize) as u32;
                    let mut shown = vec![Thing::Picture(thing); a as usize];
                    shown.push(Thing::Sign('+'));
                    shown.extend(vec![Thing::Picture(thing); b as usize]);
                    (a, b, a + b, shown, '+', "plus")
                } else {
                    let a = 3 + rng.below(8) as u32;
                    let b = 1 + rng.below((a - 1).min(5) as usize) as u32;
                    let mut shown = vec![Thing::Picture(thing); (a - b) as usize];
                    shown.extend(vec![Thing::Faded(thing); b as usize]);
                    (a, b, a - b, shown, '-', "take-away")
                };
                let wrong = near(rng, answer, 1, 10);
                let mut q = question(rng, format!("{a} {sign} {b} = {BLANK}"), [answer.to_string(), wrong[0].clone(), wrong[1].clone()], Show::Things(shown));
                let ask = if adding { "in-all" } else { "left" };
                q.bubble = words::shown(ask).to_string();
                q.ask = vec![ask.to_string()];
                q.read = vec![words::number_voice(a), said.to_string(), words::number_voice(b), "makes".to_string(), words::number_voice(answer)];
                q
            }
        };
        q.keys = Some(q.options.clone().map(|answer| digit(&answer)));
        questions.push(q);
    }
    questions
}

/// Whether a question shows this thing.
fn shows(question: &Question, thing: &str) -> bool {
    matches!(&question.show, Show::Things(things) if things.iter().any(|t| matches!(t, Thing::Picture(name) | Thing::Faded(name) if *name == thing)))
}

/// `count` different pictures, by name.
fn some_pictures(rng: &mut Rng, count: usize) -> Vec<&'static str> {
    let mut names = art::names();
    rng.shuffle(&mut names);
    names.truncate(count);
    names
}

/// Picture words: on Easy, which of three pictures is the word she says; on Medium,
/// which of three words is the picture. (Hard types the word: see `App::typing`.)
pub fn pictures(rng: &mut Rng, level: Level) -> Vec<Question> {
    let mut questions = Vec::new();
    let mut met: Vec<&str> = Vec::new();
    while questions.len() < QUESTIONS {
        let three = some_pictures(rng, 3);
        if met.contains(&three[0]) {
            continue;
        }
        met.push(three[0]);
        let answers = [three[0], three[1], three[2]].map(String::from);
        let q = if level == Level::Easy {
            let mut q = question(rng, three[0].to_string(), answers, Show::Text);
            q.pictures = true;
            q.bubble = format!("{} {}?", words::shown("where"), three[0]);
            q.ask = vec!["where".to_string(), words::word_voice(three[0])];
            q
        } else {
            let mut q = question(rng, String::new(), answers, Show::Things(vec![Thing::Picture(three[0])]));
            q.bubble = words::shown("what-is").to_string();
            q.ask = vec!["what-is".to_string()];
            q
        };
        questions.push(Question { read: vec![words::word_voice(three[0])], ..q });
    }
    questions
}

/// The little letters that look like another one turned over.
const MIRRORS: [(char, char); 8] = [('b', 'd'), ('d', 'b'), ('p', 'q'), ('q', 'p'), ('n', 'u'), ('u', 'n'), ('m', 'w'), ('w', 'm')];

fn mirror(c: char) -> Option<char> {
    MIRRORS.iter().find(|&&(a, _)| a == c).map(|&(_, b)| b)
}

/// Big and little: which little letter goes with a big one (Easy), which big one with
/// a little one (Medium), and which word in little letters is the same as one in big
/// letters, when the others have a letter turned over (Hard).
pub fn letters(rng: &mut Rng, level: Level) -> Vec<Question> {
    let mut questions = Vec::new();
    let mut met: Vec<char> = Vec::new();
    if level == Level::Hard {
        let mut pool: Vec<&str> =
            words::short_words().iter().chain(words::long_words()).copied().filter(|w| w.len() <= 6 && w.chars().any(|c| mirror(c).is_some())).collect();
        rng.shuffle(&mut pool);
        for word in pool.into_iter().take(QUESTIONS) {
            let [a, b] = turned(rng, word);
            let mut q = question(rng, word.to_uppercase(), [word.to_string(), a, b], Show::Text);
            q.bubble = words::shown("same").to_string();
            q.ask = vec!["same".to_string(), words::word_voice(word)];
            q.read = vec![words::word_voice(word)];
            questions.push(q);
        }
        return questions;
    }
    while questions.len() < QUESTIONS {
        let little = (b'a' + rng.below(26) as u8) as char;
        if met.contains(&little) {
            continue;
        }
        met.push(little);
        // The letter it is most often taken for is one of the wrong answers.
        let mut wrong: Vec<char> = mirror(little).into_iter().collect();
        while wrong.len() < 2 {
            let other = (b'a' + rng.below(26) as u8) as char;
            if other != little && !wrong.contains(&other) {
                wrong.push(other);
            }
        }
        let big = little.to_ascii_uppercase();
        let (shown, answers, phrase) = if level == Level::Easy {
            (big, [little, wrong[0], wrong[1]], "little")
        } else {
            (little, [big, wrong[0].to_ascii_uppercase(), wrong[1].to_ascii_uppercase()], "big")
        };
        let mut q = question(rng, shown.to_string(), answers.map(String::from), Show::Text);
        q.keys = Some(q.options.clone().map(|answer| answer.chars().next().unwrap_or(' ')));
        q.bubble = format!("{} {shown}!", words::shown(phrase));
        q.ask = vec![phrase.to_string(), words::letter_voice(big)];
        q.read = vec![words::letter_voice(big)];
        questions.push(q);
    }
    questions
}

/// Two ways of getting a word wrong by turning a letter over, or by changing two
/// round when it has only one letter that turns.
fn turned(rng: &mut Rng, word: &str) -> [String; 2] {
    let letters: Vec<char> = word.chars().collect();
    let mut ways: Vec<String> = Vec::new();
    for (i, &c) in letters.iter().enumerate() {
        if let Some(m) = mirror(c) {
            let mut other = letters.clone();
            other[i] = m;
            ways.push(other.into_iter().collect());
        }
    }
    let all: String = letters.iter().map(|&c| mirror(c).unwrap_or(c)).collect();
    ways.push(all);
    let mut swapped = letters.clone();
    swapped.swap(0, 1);
    ways.push(swapped.into_iter().collect());
    let mut last = letters.clone();
    last.swap(letters.len() - 2, letters.len() - 1);
    ways.push(last.into_iter().collect());
    ways.retain(|way| way != word);
    ways.dedup();
    let mut kept: Vec<String> = Vec::new();
    rng.shuffle(&mut ways);
    for way in ways {
        if !kept.contains(&way) {
            kept.push(way);
        }
    }
    [kept[0].clone(), kept.get(1).cloned().unwrap_or_else(|| format!("{word}{}", letters[0]))]
}

/// "What's next?": a pattern of pictures (Easy, and some of Medium), counting up or
/// down, the alphabet (Medium), and counting in twos, fives and tens (Hard).
pub fn next(rng: &mut Rng, level: Level) -> Vec<Question> {
    let mut questions = Vec::new();
    for round in 0..QUESTIONS {
        let kind = match level {
            Level::Easy => 0,
            Level::Medium => [1, 2, 3, 0][(round + rng.below(4)) % 4],
            Level::Hard => [4, 4, 5, 3][(round + rng.below(4)) % 4],
        };
        let mut q = match kind {
            0 => pattern(rng, level),
            3 => {
                // The alphabet: in order on Medium, every other letter on Hard.
                let step = if level == Level::Hard { 2 } else { 1 };
                let start = rng.below(26 - 4 * step) as u8;
                let shown: Vec<String> = (0..4).map(|i| ((b'A' + start + i * step as u8) as char).to_string()).collect();
                let right = (b'A' + start + 4 * step as u8) as char;
                let wrong = [(right as u8 - 1) as char, ((right as u8 - b'A' + 1) % 26 + b'A') as char];
                let mut q = question(rng, format!("{} {BLANK}", shown.join(" ")), [right, wrong[0], wrong[1]].map(String::from), Show::Text);
                q.read = vec![words::letter_voice(right)];
                q
            }
            _ => {
                let (step, start): (i32, u32) = match kind {
                    1 => (1, 1 + rng.below(12) as u32),
                    2 => (-1, 6 + rng.below(15) as u32),
                    // Twos, fives or tens, now and then down.
                    _ => match rng.below(5) {
                        0 => (2, 2 * rng.below(6) as u32),
                        1 => (5, 5 * rng.below(6) as u32),
                        2 => (10, 10 * rng.below(6) as u32),
                        3 => (-2, 20 - 2 * rng.below(3) as u32),
                        _ => (-10, 100 - 10 * rng.below(3) as u32),
                    },
                };
                let at = |i: i32| (start as i32 + step * i) as u32;
                let shown: Vec<String> = (0..4).map(|i| at(i).to_string()).collect();
                let right = at(4);
                let wrong = if step.abs() > 1 { [(right as i32 + step).max(0) as u32, right + 1] } else { [right + 2, right.abs_diff(1).max(1)] };
                let wrong = if wrong[1] == right || wrong[0] == wrong[1] { [right + 3, right + 2] } else { wrong };
                let mut q = question(rng, format!("{} {BLANK}", shown.join(" ")), [right.to_string(), wrong[0].to_string(), wrong[1].to_string()], Show::Text);
                q.read = vec![words::number_voice(right)];
                q
            }
        };
        q.bubble = words::shown("next").to_string();
        q.ask = vec!["next".to_string()];
        questions.push(q);
    }
    questions
}

/// A row of pictures that goes on in a pattern, and what comes after it.
fn pattern(rng: &mut Rng, level: Level) -> Question {
    let mut names: Vec<&str> = art::things().iter().map(|art| art.name).collect();
    rng.shuffle(&mut names);
    let (a, b, c) = (names[0], names[1], names[2]);
    let units: &[&[&str]] = if level == Level::Easy { &[&[a, b], &[a, a, b], &[a, b, b]] } else { &[&[a, b, c], &[a, a, b, b], &[a, b, c, c]] };
    let unit = units[rng.below(units.len())];
    let length = if unit.len() == 2 { 5 } else { 6 };
    let mut shown: Vec<Thing> = (0..length).map(|i| Thing::Picture(unit[i % unit.len()])).collect();
    shown.push(Thing::Sign('?'));
    let right = unit[length % unit.len()];
    let others: Vec<&str> = [a, b, c, names[3]].into_iter().filter(|&n| n != right).collect();
    let mut q = question(rng, String::new(), [right, others[0], others[1]].map(String::from), Show::Things(shown));
    q.pictures = true;
    q.read = vec![words::word_voice(right)];
    q
}

/// The cards of pairs, shuffled: pictures (Easy), big and little letters (Medium),
/// and words that rhyme (Hard).
pub fn pairs(rng: &mut Rng, level: Level) -> Vec<Card> {
    let mut cards = Vec::new();
    match level {
        Level::Easy => {
            for (pair, name) in some_pictures(rng, 3).into_iter().enumerate() {
                cards.extend([0, 1].map(|_| Card::new(Face::Picture(name), pair)));
            }
        }
        Level::Medium => {
            let mut all: Vec<char> = ('a'..='z').collect();
            rng.shuffle(&mut all);
            for (pair, little) in all.into_iter().take(4).enumerate() {
                cards.push(Card::new(Face::Text(little.to_ascii_uppercase().to_string()), pair));
                cards.push(Card::new(Face::Text(little.to_string()), pair));
            }
        }
        Level::Hard => {
            let mut families: Vec<&Vec<&str>> = words::rhymes().iter().collect();
            rng.shuffle(&mut families);
            for (pair, family) in families.into_iter().take(5).enumerate() {
                let mut two = family.clone();
                rng.shuffle(&mut two);
                cards.extend(two[..2].iter().map(|w| Card::new(Face::Text(w.to_string()), pair)));
            }
        }
    }
    rng.shuffle(&mut cards);
    cards
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn check(q: &Question) {
        let answers: HashSet<&String> = q.options.iter().collect();
        assert_eq!(answers.len(), 3, "{q:?}");
        assert!(q.right < 3 && !q.read.is_empty() && !q.ask.is_empty() && !q.bubble.is_empty(), "{q:?}");
        if let Some(keys) = q.keys {
            assert_eq!(keys.iter().map(|k| k.to_ascii_lowercase()).collect::<HashSet<_>>().len(), 3, "{q:?}");
        }
        if q.pictures {
            assert!(q.options.iter().all(|o| art::find(o).is_some()), "{q:?}");
        }
        // Everything she says has a clip to say it with.
        let clips: HashSet<String> = words::spoken().into_iter().map(|(name, _)| name).collect();
        for clip in q.ask.iter().chain(&q.read) {
            assert!(clips.contains(clip), "no clip {clip} for {q:?}");
        }
        assert!(crate::font::supported(&q.text) && q.options.iter().all(|o| crate::font::supported(o)), "{q:?}");
    }

    #[test]
    fn every_question_has_three_different_answers_and_clips_for_what_she_says() {
        let mut rng = Rng::new(3);
        for _ in 0..40 {
            for level in Level::ALL {
                for q in counting(&mut rng, level).iter().chain(&pictures(&mut rng, level)).chain(&letters(&mut rng, level)).chain(&next(&mut rng, level)) {
                    check(q);
                }
            }
        }
    }

    #[test]
    fn counting_counts_right_and_answers_by_digit() {
        let mut rng = Rng::new(5);
        for _ in 0..50 {
            for level in Level::ALL {
                for q in counting(&mut rng, level) {
                    let Show::Things(things) = &q.show else { panic!() };
                    let here = things.iter().filter(|t| matches!(t, Thing::Picture(_))).count() as u32;
                    let right: u32 = q.options[q.right].parse().unwrap();
                    let most = if level == Level::Easy { 5 } else { 10 };
                    assert!(q.options.iter().all(|o| (1..=most).contains(&o.parse::<u32>().unwrap())), "{q:?}");
                    assert_eq!(q.keys.unwrap()[q.right], if right == 10 { '0' } else { char::from_digit(right, 10).unwrap() });
                    // What is there to count is the answer, whether some were added or went away.
                    assert_eq!(here, right, "{q:?}");
                    if level == Level::Hard {
                        assert!(q.text.ends_with(BLANK) && (q.text.contains('+') || q.text.contains('-')), "{q:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn big_and_little_offers_the_turned_over_letter() {
        let mut rng = Rng::new(9);
        let mut mirrored = 0;
        for _ in 0..30 {
            for q in letters(&mut rng, Level::Easy) {
                let big = q.text.chars().next().unwrap();
                assert_eq!(q.options[q.right], big.to_ascii_lowercase().to_string());
                if let Some(m) = mirror(big.to_ascii_lowercase()) {
                    assert!(q.options.contains(&m.to_string()));
                    mirrored += 1;
                }
            }
            for q in letters(&mut rng, Level::Hard) {
                assert_eq!(q.options[q.right], q.text.to_lowercase());
                assert!(q.options.iter().all(|o| o.len() == q.text.len() || o.len() == q.text.len() + 1));
            }
        }
        assert!(mirrored > 0);
    }

    #[test]
    fn what_comes_next_is_what_comes_next() {
        let mut rng = Rng::new(11);
        let said: HashSet<String> = words::spoken().into_iter().map(|(name, _)| name).collect();
        for _ in 0..60 {
            for level in Level::ALL {
                for q in next(&mut rng, level) {
                    let right = &q.options[q.right];
                    if let Show::Things(things) = &q.show {
                        assert_eq!(things.last(), Some(&Thing::Sign('?')));
                        assert!(q.pictures);
                    } else {
                        let shown: Vec<&str> = q.text.split(' ').filter(|s| *s != BLANK).collect();
                        if let Ok(numbers) = shown.iter().map(|s| s.parse::<i32>()).collect::<Result<Vec<_>, _>>() {
                            let step = numbers[1] - numbers[0];
                            assert!(numbers.windows(2).all(|w| w[1] - w[0] == step), "{q:?}");
                            assert_eq!(right.parse::<i32>().unwrap(), numbers[3] + step);
                            assert!(said.contains(&words::number_voice((numbers[3] + step) as u32)));
                        } else {
                            let letters: Vec<u8> = shown.iter().map(|s| s.as_bytes()[0]).collect();
                            assert_eq!(right.as_bytes()[0] as i32, letters[3] as i32 + letters[1] as i32 - letters[0] as i32, "{q:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn pairs_come_in_pairs() {
        let mut rng = Rng::new(13);
        for (level, count) in [(Level::Easy, 6), (Level::Medium, 8), (Level::Hard, 10)] {
            let cards = pairs(&mut rng, level);
            assert_eq!(cards.len(), count);
            for pair in 0..count / 2 {
                let two: Vec<&Card> = cards.iter().filter(|card| card.pair == pair).collect();
                assert_eq!(two.len(), 2);
                match (&two[0].face, &two[1].face) {
                    (Face::Picture(a), Face::Picture(b)) => assert_eq!(a, b),
                    (Face::Text(a), Face::Text(b)) if level == Level::Medium => assert!(a != b && a.eq_ignore_ascii_case(b)),
                    (Face::Text(a), Face::Text(b)) => assert!(a != b && words::family(a) == words::family(b)),
                    faces => panic!("{faces:?}"),
                }
            }
        }
    }
}
