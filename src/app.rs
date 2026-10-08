//! `App`: where the player is, what every key, click and tick of the clock does, the
//! four games, the celebrations, and the gifts that are kept between runs. Draws
//! nothing.

use std::f32::consts::{PI, TAU};
use std::path::PathBuf;
use std::time::Instant;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

use crate::kitty::{Arms, Eyes, GIFTS, Look, Mouth};
use crate::sound::{FANFARES, Sound, Speaker};
use crate::theme::{Rgb, Settings, THEMES, Theme};
use crate::words::{self, BLANK, Level, Rng};

/// Seconds the little celebration after each word or answer lasts.
pub const CHEER: f32 = 1.4;
/// Seconds a treat takes to fly to her mouth; then she eats it.
pub const FLY: f32 = 0.45;
/// Seconds the opening lasts when nobody skips it.
pub const INTRO: f32 = 4.6;
/// Seconds she waves goodbye for.
pub const BYE: f32 = 1.8;
/// Seconds a party keeps throwing things in the air.
pub const PARTY: f32 = 7.0;
/// Seconds a caught ball of yarn takes to burst.
pub const BURST: f32 = 0.35;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Game {
    /// Type what is written on the treat, and she eats it.
    Snack,
    /// Type what she says.
    Says,
    /// Press the letter on a ball of yarn before it rolls past.
    Yarn,
    /// Pick the word that is missing from the sentence.
    Story,
}

impl Game {
    pub const ALL: [Game; 4] = [Game::Snack, Game::Says, Game::Yarn, Game::Story];

    pub fn name(self) -> &'static str {
        match self {
            Game::Snack => "Snack time",
            Game::Says => "Kitty says",
            Game::Yarn => "Yarn balls",
            Game::Story => "Story time",
        }
    }

    /// The phrase she opens the game with.
    fn intro(self, level: Level) -> &'static str {
        match (self, level) {
            (Game::Snack, _) => "snack-intro",
            (Game::Says, _) => "says-intro",
            (Game::Yarn, _) => "yarn-intro",
            (Game::Story, Level::Easy) => "starts-intro",
            (Game::Story, _) => "story-intro",
        }
    }
}

/// Everything a button can do. Keys do the same things through `App::act`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Play(Game),
    /// Any of the games.
    Surprise,
    /// To the dressing room.
    Dress,
    Level(Level),
    Theme,
    Sound,
    Motion,
    Help,
    Quit,
    /// Back to the home screen.
    Back,
    /// The same game once more.
    Again,
    /// A key of the keyboard on the screen, or a ball of yarn.
    Key(char),
    /// One of the three words of a story.
    Pick(usize),
    /// On from a finished story to the next.
    Next,
    /// Say it again.
    Repeat,
    /// Put a gift on, or take it off.
    Wear(usize),
    /// Past the opening, or the goodbye.
    Skip,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Intro,
    Home,
    Play,
    /// A game is finished.
    Party,
    /// The dressing room.
    Dress,
    Bye,
}

/// Typing: snack time and "Kitty says".
#[derive(Debug)]
pub struct Typing {
    /// What is to be typed, in capitals.
    pub items: Vec<String>,
    pub at: usize,
    /// How many letters of this one have been typed.
    pub typed: usize,
    /// Wrong keys since this one began.
    pub misses: u32,
    /// The words are heard and not shown, unless it turns out to be too hard.
    pub hidden: bool,
    /// Seconds since this one was finished, while that is being celebrated.
    pub cheer: Option<f32>,
}

#[derive(Debug)]
pub struct Ball {
    pub letter: char,
    /// From 1 at the far edge to 0 at Pink Kitty.
    pub x: f32,
    /// Which of the theme's colors it is.
    pub color: usize,
    /// Seconds since it was caught, while it bursts.
    pub popped: Option<f32>,
}

#[derive(Debug)]
pub struct Yarn {
    pub balls: Vec<Ball>,
    pub caught: usize,
    pub goal: usize,
    next_in: f32,
    /// Seconds since the last was caught.
    done: Option<f32>,
}

#[derive(Debug)]
pub struct Question {
    /// The sentence, with `BLANK` for the missing word.
    pub text: String,
    pub options: [String; 3],
    pub right: usize,
    /// What she says before it, and what she reads out once it is right.
    ask: Vec<String>,
    read: Vec<String>,
}

#[derive(Debug)]
pub struct Quiz {
    pub items: Vec<Question>,
    pub at: usize,
    /// The wrong words already tried.
    pub tried: [bool; 3],
    /// Seconds since the right word was found.
    pub solved: Option<f32>,
    /// How long to wait then, so that she can read the sentence out.
    wait: f32,
}

#[derive(Debug)]
pub enum Play {
    Typing(Typing),
    Yarn(Yarn),
    Quiz(Quiz),
}

#[derive(Debug)]
pub struct Task {
    pub game: Game,
    pub level: Level,
    pub play: Play,
}

impl Task {
    /// How many things are done, and how many there are.
    pub fn progress(&self) -> (usize, usize) {
        match &self.play {
            Play::Typing(typing) => (typing.at + usize::from(typing.cheer.is_some()), typing.items.len()),
            Play::Yarn(yarn) => (yarn.caught, yarn.goal),
            Play::Quiz(quiz) => (quiz.at + usize::from(quiz.solved.is_some()), quiz.items.len()),
        }
    }
}

/// The ways a finished game is celebrated. One is picked each time, never the same
/// twice running: the user asked for many, so that it does not get boring.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fun {
    Confetti,
    Fireworks,
    Balloons,
    Hearts,
    /// Shooting stars.
    Stars,
    Bubbles,
    Rainbow,
    /// Music, and she sways to it.
    Dance,
    /// A garden grows along the bottom.
    Flowers,
    /// Fish swim by, which she likes best of all.
    Fish,
    Spiral,
    Glitter,
}

impl Fun {
    pub const ALL: [Fun; 12] = [
        Fun::Confetti,
        Fun::Fireworks,
        Fun::Balloons,
        Fun::Hearts,
        Fun::Stars,
        Fun::Bubbles,
        Fun::Rainbow,
        Fun::Dance,
        Fun::Flowers,
        Fun::Fish,
        Fun::Spiral,
        Fun::Glitter,
    ];

    /// Seconds between one thing thrown in the air and the next.
    fn every(self) -> f32 {
        match self {
            Fun::Confetti => 0.03,
            Fun::Fireworks => 0.5,
            Fun::Balloons => 0.3,
            Fun::Hearts => 0.06,
            Fun::Stars => 0.05,
            Fun::Bubbles => 0.07,
            Fun::Rainbow => 0.05,
            Fun::Dance => 0.12,
            Fun::Flowers => 0.12,
            Fun::Fish => 0.35,
            Fun::Spiral => 0.02,
            Fun::Glitter => 0.015,
        }
    }
}

/// What is written across a party.
const HEADLINES: [&str; 10] = ["You did it!", "Hooray!", "Well done!", "Super!", "Amazing!", "Wow!", "Purr-fect!", "Superstar!", "Fantastic!", "Yay!"];

#[derive(Debug)]
pub struct Party {
    pub fun: Fun,
    pub headline: &'static str,
    /// The gift this game won, one of `GIFTS`. A heart, when she has them all.
    pub gift: Option<usize>,
    pub age: f32,
    emit: f32,
}

/// The little celebrations after each word or answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cheer {
    Sparkle,
    Hearts,
    Fountain,
    Notes,
    Cannons,
    Ring,
}

impl Cheer {
    pub const ALL: [Cheer; 6] = [Cheer::Sparkle, Cheer::Hearts, Cheer::Fountain, Cheer::Notes, Cheer::Cannons, Cheer::Ring];
}

/// How Pink Kitty feels, which is how she is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {
    Calm,
    /// Paws up, jumping.
    Cheer,
    Munch,
    /// A wrong key: wide eyes for a moment. Never a frown.
    Oops,
    /// Hearts for eyes.
    Love,
    Wave,
    /// Leaping at a ball of yarn.
    Pounce,
}

/// A bit of confetti, a heart, a bubble: placed in cells.
pub struct Particle {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    gravity: f32,
    /// How far it swings from side to side as it goes.
    sway: f32,
    pub age: f32,
    pub life: f32,
    pub color: Rgb,
    /// What it is drawn as. `BALLOON` is a picture, not a character.
    pub symbol: &'static str,
    /// Drawn over the screen, as the little celebrations are, or behind it.
    pub front: bool,
}

pub const BALLOON: &str = "balloon";

/// The gifts she has been given and the ones she has on. They are kept between runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Gifts {
    pub earned: [bool; GIFTS.len()],
    pub worn: [bool; GIFTS.len()],
    /// What a game wins once she has every gift.
    pub hearts: u32,
}

impl Gifts {
    /// `gifts` in the state directory.
    pub fn path() -> PathBuf {
        crate::update::state_dir().join("gifts")
    }

    pub fn count(&self) -> usize {
        self.earned.iter().filter(|&&e| e).count()
    }

    /// Lines of `name=value`. Anything missing or not understood is left out.
    pub fn parse(text: &str) -> Gifts {
        let mut gifts = Gifts::default();
        let listed = |value: &str| {
            let mut set = [false; GIFTS.len()];
            for name in value.split(',') {
                if let Some(i) = GIFTS.iter().position(|gift| words::slug(gift.name) == name.trim()) {
                    set[i] = true;
                }
            }
            set
        };
        for line in text.lines() {
            match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                Some(("earned", v)) => gifts.earned = listed(v),
                Some(("worn", v)) => gifts.worn = listed(v),
                Some(("hearts", v)) => gifts.hearts = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        // She cannot wear what she was never given.
        for i in 0..GIFTS.len() {
            gifts.worn[i] &= gifts.earned[i];
        }
        gifts
    }

    pub fn format(&self) -> String {
        let listed = |set: &[bool]| GIFTS.iter().zip(set).filter(|&(_, &on)| on).map(|(gift, _)| words::slug(gift.name)).collect::<Vec<_>>().join(",");
        format!("earned={}\nworn={}\nhearts={}\n", listed(&self.earned), listed(&self.worn), self.hearts)
    }

    pub fn load(path: &std::path::Path) -> Gifts {
        Gifts::parse(&std::fs::read_to_string(path).unwrap_or_default())
    }

    /// Failing to save is not worth interrupting a game for.
    pub fn save(&self, path: &std::path::Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, self.format());
    }

    /// Puts a gift on, or takes it off. She wears one scarf pattern at a time.
    fn toggle(&mut self, gift: usize) {
        if !self.earned[gift] {
            return;
        }
        self.worn[gift] = !self.worn[gift];
        if self.worn[gift] && GIFTS[gift].scarf {
            for other in (0..GIFTS.len()).filter(|&other| other != gift && GIFTS[other].scarf) {
                self.worn[other] = false;
            }
        }
    }
}

pub struct App {
    pub screen: Screen,
    /// The help is shown over whatever screen this is.
    pub help: bool,
    pub level: Level,
    /// The game being played, or the one just finished.
    pub task: Option<Task>,
    pub party: Option<Party>,
    pub gifts: Gifts,
    pub gifts_path: Option<PathBuf>,
    pub settings_path: Option<PathBuf>,
    /// The button Enter presses. It follows the arrow keys and the mouse.
    pub marker: Option<Action>,
    /// What Pink Kitty is saying, for her speech bubble.
    pub bubble: String,
    /// Seconds she will go on talking for, so that her mouth moves.
    pub talk: f32,
    pub mood: Mood,
    mood_left: f32,
    /// Seconds since the screen began: the opening, the goodbye.
    pub phase_time: f32,
    /// Seconds things have been moving for; it stands still while animations are off.
    pub time: f32,
    pub particles: Vec<Particle>,
    /// What can be clicked, as of the last time the screen was drawn; later ones are
    /// on top of earlier ones.
    pub buttons: Vec<(Rect, Action)>,
    /// Where she was drawn, for what flies out of her.
    pub kitty: Rect,
    /// The window in cells.
    pub size: (u16, u16),
    pub theme: usize,
    pub sound: bool,
    pub animations: bool,
    auto_update: bool,
    pub truecolor: bool,
    pub speaker: Speaker,
    pub quit: bool,
    rng: Rng,
    /// Words, sentences and stories already met since the game was started, so that
    /// the next game brings others.
    seen: Vec<String>,
    last_phrase: &'static str,
    last_cheer: Option<Cheer>,
    last_fun: Option<Fun>,
    last_game: Game,
    last_tick: Option<Instant>,
    /// The mouse button went down and has not come up yet.
    button_down: bool,
    intro_said: bool,
}

impl App {
    pub fn new(truecolor: bool, settings: Settings, gifts: Gifts) -> App {
        let mut app = App {
            screen: Screen::Home,
            help: false,
            level: settings.level,
            task: None,
            party: None,
            gifts,
            gifts_path: None,
            settings_path: None,
            marker: None,
            bubble: String::new(),
            talk: 0.0,
            mood: Mood::Calm,
            mood_left: 0.0,
            phase_time: 0.0,
            time: 0.0,
            particles: Vec::new(),
            buttons: Vec::new(),
            kitty: Rect::new(0, 0, 30, 14),
            size: (80, 24),
            theme: settings.theme,
            sound: settings.sound,
            animations: settings.animations,
            auto_update: settings.update,
            truecolor,
            speaker: Speaker::silent(),
            quit: false,
            rng: Rng::from_clock(),
            seen: Vec::new(),
            last_phrase: "",
            last_cheer: None,
            last_fun: None,
            last_game: Game::Snack,
            last_tick: None,
            button_down: false,
            intro_said: false,
        };
        app.home();
        app
    }

    pub fn theme(&self) -> &'static Theme {
        &THEMES[self.theme]
    }

    /// Opens with the intro, or, when nothing is to move, with a hello.
    pub fn begin(&mut self, intro: bool) {
        if intro && self.animations {
            self.screen = Screen::Intro;
            self.phase_time = 0.0;
            self.intro_said = false;
            self.bubble = words::shown("hello").to_string();
            self.feel(Mood::Wave, INTRO);
            self.play(Sound::Intro);
        } else {
            self.say(&["hello".to_string()]);
        }
    }

    fn play(&mut self, sound: Sound) {
        if self.sound {
            self.speaker.play(sound);
        }
    }

    /// Has her say these clips, and keeps her mouth moving for as long as they take.
    /// Returns how many seconds that is.
    fn say(&mut self, clips: &[String]) -> f32 {
        if !self.sound {
            return 0.0;
        }
        let seconds = self.speaker.say(clips);
        self.talk = (self.talk + seconds).min(9.0);
        seconds
    }

    /// Has her say a phrase, and shows it in her speech bubble.
    fn speak(&mut self, phrase: &'static str) -> f32 {
        self.bubble = words::shown(phrase).to_string();
        self.say(&[phrase.to_string()])
    }

    /// One of a group of phrases, and not the one she said last.
    fn one_of(&mut self, group: &str) -> &'static str {
        let mut choices = words::group(group);
        if choices.len() > 1 {
            choices.retain(|&id| id != self.last_phrase);
        }
        self.last_phrase = if choices.is_empty() { "" } else { self.rng.pick(&choices) };
        self.last_phrase
    }

    fn feel(&mut self, mood: Mood, seconds: f32) {
        self.mood = mood;
        self.mood_left = seconds;
    }

    /// Whether she can be heard speaking.
    pub fn voiced(&self) -> bool {
        self.sound && self.speaker.is_on()
    }

    fn save_settings(&self) {
        if let Some(path) = &self.settings_path {
            Settings { theme: self.theme, update: self.auto_update, animations: self.animations, sound: self.sound, level: self.level }.save(path);
        }
    }

    fn save_gifts(&self) {
        if let Some(path) = &self.gifts_path {
            self.gifts.save(path);
        }
    }

    /// How she is to be drawn now.
    pub fn look(&self) -> Look {
        let chewing = self.animations && (self.mood == Mood::Munch || self.talk > 0.0);
        Look {
            eyes: match self.mood {
                Mood::Oops => Eyes::Open,
                Mood::Love => Eyes::Hearts,
                _ => Eyes::Smile,
            },
            mouth: if chewing && (self.time * 8.0) as i32 % 2 == 0 { Mouth::Open } else { Mouth::Smile },
            arms: match self.mood {
                Mood::Cheer => Arms::Up,
                Mood::Wave => Arms::Wave,
                _ => Arms::Down,
            },
            time: self.time,
            worn: self.gifts.worn,
        }
    }

    /// How far she is from where she stands, right and down, in the units of her
    /// drawing: jumping for joy, leaping at a ball, swaying to the music.
    pub fn hop(&self) -> (f32, f32) {
        if !self.animations {
            return (0.0, 0.0);
        }
        let dancing = self.screen == Screen::Party && self.party.as_ref().is_some_and(|party| party.fun == Fun::Dance && party.age < PARTY);
        match self.mood {
            _ if dancing => ((self.time * 5.0).sin() * 4.0, -(self.time * 10.0).sin().abs() * 2.0),
            Mood::Cheer => (0.0, -(self.time * 9.0).sin().abs() * 5.0),
            Mood::Pounce => {
                let leap = (self.mood_left / BURST).clamp(0.0, 1.0) * PI;
                (leap.sin() * 7.0, -leap.sin() * 3.0)
            }
            Mood::Love => (0.0, -(self.time * 6.0).sin().abs() * 2.0),
            _ => (0.0, 0.0),
        }
    }

    /// `count` of `pool`, none twice, and as few as can be that were met before.
    fn draw(&mut self, pool: &[&str], count: usize) -> Vec<String> {
        let mut fresh: Vec<&str> = pool.iter().copied().filter(|item| !self.seen.iter().any(|seen| seen == item)).collect();
        if fresh.len() < count {
            // All met: start over with the whole pool.
            self.seen.retain(|seen| !pool.contains(&seen.as_str()));
            fresh = pool.to_vec();
        }
        self.rng.shuffle(&mut fresh);
        fresh.truncate(count);
        self.seen.extend(fresh.iter().map(|item| item.to_string()));
        fresh.into_iter().map(String::from).collect()
    }

    fn letters(&mut self, count: usize) -> Vec<String> {
        let mut all: Vec<char> = ('A'..='Z').collect();
        self.rng.shuffle(&mut all);
        all.into_iter().take(count).map(String::from).collect()
    }

    fn typing(&mut self, game: Game) -> Typing {
        let capitals = |items: Vec<String>| items.into_iter().map(|item| item.to_uppercase()).collect();
        let items = match (game, self.level) {
            (_, Level::Easy) => self.letters(5),
            (_, Level::Medium) => capitals(self.draw(words::short_words(), 5)),
            (Game::Snack, Level::Hard) => capitals(self.draw(words::sentences(), 3)),
            (_, Level::Hard) => capitals(self.draw(words::long_words(), 5)),
        };
        Typing { items, at: 0, typed: 0, misses: 0, hidden: game == Game::Says && self.level == Level::Hard, cheer: None }
    }

    fn quiz(&mut self) -> Quiz {
        let mut items = Vec::new();
        if self.level == Level::Easy {
            for word in self.draw(words::short_words(), 3) {
                let first = word.chars().next().unwrap_or('a').to_ascii_uppercase();
                let mut others: Vec<char> = ('A'..='Z').filter(|&c| c != first).collect();
                self.rng.shuffle(&mut others);
                let mut options = [first, others[0], others[1]];
                self.rng.shuffle(&mut options);
                items.push(Question {
                    text: format!("{} starts with {BLANK}", word.to_uppercase()),
                    options: options.map(String::from),
                    right: options.iter().position(|&c| c == first).unwrap_or(0),
                    ask: vec!["starts".to_string(), words::word_voice(&word)],
                    read: vec![words::letter_voice(first), words::word_voice(&word)],
                });
            }
        } else {
            let pool: Vec<&'static str> = words::stories().iter().filter(|s| s.level == self.level).map(|s| s.text).collect();
            for text in self.draw(&pool, 3) {
                let Some(story) = words::stories().iter().find(|s| s.text == text) else { continue };
                let mut options = story.options;
                self.rng.shuffle(&mut options);
                items.push(Question {
                    text,
                    options: options.map(String::from),
                    right: options.iter().position(|&o| o == story.options[0]).unwrap_or(0),
                    ask: Vec::new(),
                    read: vec![story.voice()],
                });
            }
        }
        Quiz { items, at: 0, tried: [false; 3], solved: None, wait: 0.0 }
    }

    /// Back to the home screen.
    fn home(&mut self) {
        self.screen = Screen::Home;
        self.task = None;
        self.party = None;
        self.particles.clear();
        self.marker = Some(Action::Play(self.last_game));
        self.bubble = words::shown("what-do").to_string();
        self.feel(Mood::Calm, 0.0);
    }

    pub fn start(&mut self, game: Game) {
        let play = match game {
            Game::Snack | Game::Says => Play::Typing(self.typing(game)),
            Game::Yarn => Play::Yarn(Yarn { balls: Vec::new(), caught: 0, goal: [6, 8, 10][self.level as usize], next_in: 0.4, done: None }),
            Game::Story => Play::Quiz(self.quiz()),
        };
        self.task = Some(Task { game, level: self.level, play });
        self.party = None;
        self.last_game = game;
        self.screen = Screen::Play;
        self.particles.clear();
        self.speaker.hush();
        self.talk = 0.0;
        self.feel(Mood::Calm, 0.0);
        self.marker = (game == Game::Story).then_some(Action::Pick(0));
        self.play(Sound::Click);
        self.speak(game.intro(self.level));
        self.ask(false);
        if !self.animations {
            self.roll(0.0);
        }
    }

    /// Puts the next thing to her: says what is to be typed, or reads the question.
    /// `again` is for when the player asks to hear it once more.
    fn ask(&mut self, again: bool) {
        let Some(task) = &self.task else { return };
        match &task.play {
            Play::Typing(typing) if task.game == Game::Says => {
                let Some(item) = typing.items.get(typing.at) else { return };
                let (item, letter) = (item.clone(), task.level == Level::Easy);
                self.bubble = match (letter, typing.hidden) {
                    (true, _) => format!("{} {item}?", words::shown("find")),
                    (false, false) => format!("{} {item}?", words::shown("type")),
                    (false, true) => words::shown("says-intro").to_string(),
                };
                let clips = if letter {
                    ["find".to_string(), words::letter_voice(item.chars().next().unwrap_or('A'))]
                } else {
                    ["type".to_string(), words::word_voice(&item)]
                };
                self.say(&clips);
            }
            Play::Quiz(quiz) => {
                let Some(question) = quiz.items.get(quiz.at) else { return };
                let clips = if question.ask.is_empty() && again { vec!["fits".to_string()] } else { question.ask.clone() };
                if task.level == Level::Easy {
                    self.bubble = words::shown("starts").to_string();
                } else if again || quiz.at > 0 {
                    self.bubble = words::shown("fits").to_string();
                }
                self.say(&clips);
            }
            _ => {}
        }
    }

    /// Whether the word to type is being kept out of sight: only while she can be
    /// heard saying it, and until two wrong keys show that it is too hard.
    pub fn concealed(&self) -> bool {
        matches!(&self.task, Some(Task { play: Play::Typing(typing), .. }) if typing.hidden && typing.misses < 2) && self.voiced()
    }

    /// The key that would be right now, for the keyboard on the screen to light up.
    /// Easy always shows it; the others only after a wrong key.
    pub fn wanted(&self) -> Option<char> {
        let task = self.task.as_ref()?;
        match &task.play {
            Play::Typing(typing) if typing.cheer.is_none() && (task.level == Level::Easy || typing.misses > 0) && !self.concealed() => {
                typing.items.get(typing.at)?.chars().nth(typing.typed)
            }
            Play::Yarn(yarn) if task.level == Level::Easy => {
                yarn.balls.iter().filter(|ball| ball.popped.is_none()).min_by(|a, b| a.x.total_cmp(&b.x)).map(|ball| ball.letter)
            }
            _ => None,
        }
    }

    fn on_char(&mut self, c: char) {
        let c = c.to_ascii_uppercase();
        let Some(task) = &mut self.task else { return };
        let game = task.game;
        match &mut task.play {
            Play::Typing(typing) => {
                if typing.cheer.is_some() || !(c.is_ascii_alphabetic() || c == ' ') {
                    return;
                }
                let Some(item) = typing.items.get(typing.at) else { return };
                if item.chars().nth(typing.typed) == Some(c) {
                    typing.typed += 1;
                    if typing.typed == item.chars().count() {
                        typing.cheer = Some(0.0);
                        self.finished_one(game);
                    } else {
                        self.play(Sound::Tick);
                    }
                } else {
                    typing.misses += 1;
                    let stuck = typing.misses % 4 == 0;
                    self.play(Sound::Oops);
                    self.feel(Mood::Oops, 0.5);
                    if stuck {
                        let comfort = self.one_of("comfort");
                        self.speak(comfort);
                    }
                }
            }
            Play::Yarn(yarn) => {
                if yarn.done.is_some() || !c.is_ascii_alphabetic() {
                    return;
                }
                let nearest = yarn.balls.iter_mut().filter(|ball| ball.popped.is_none() && ball.letter == c).min_by(|a, b| a.x.total_cmp(&b.x));
                let Some(ball) = nearest else {
                    self.play(Sound::Oops);
                    return;
                };
                ball.popped = Some(0.0);
                yarn.caught += 1;
                // The next one is not long in coming.
                yarn.next_in = yarn.next_in.min(0.5);
                let (caught, goal) = (yarn.caught, yarn.goal);
                if caught == goal {
                    yarn.done = Some(0.0);
                }
                self.play(Sound::Pop);
                self.feel(Mood::Pounce, BURST);
                if let Some(&(at, _)) = self.buttons.iter().rev().find(|(_, action)| *action == Action::Key(c)) {
                    self.burst(at.x as f32 + at.width as f32 / 2.0, at.y as f32 + at.height as f32 / 2.0, 14, &["★", "✦"], 12.0, 14.0);
                }
                if caught % 3 == 0 && caught < goal {
                    let cry = self.one_of("pounce");
                    self.speak(cry);
                }
                if !self.animations {
                    self.roll(0.0);
                }
            }
            Play::Quiz(_) => {}
        }
    }

    /// A word typed: she is pleased, and in snack time she eats.
    fn finished_one(&mut self, game: Game) {
        let praise = self.one_of(if game == Game::Snack { "yum" } else { "praise" });
        self.speak(praise);
        if game == Game::Snack {
            self.play(Sound::Tick);
        } else {
            self.play(Sound::Chime);
            self.feel(Mood::Cheer, CHEER);
            self.cheer();
        }
        if !self.animations {
            if game == Game::Snack {
                self.play(Sound::Munch);
            }
            self.next_one();
        }
    }

    /// On to the next thing to type, or to the party.
    fn next_one(&mut self) {
        let Some(Task { play: Play::Typing(typing), .. }) = &mut self.task else { return };
        typing.at += 1;
        typing.typed = 0;
        typing.misses = 0;
        typing.cheer = None;
        if typing.at >= typing.items.len() {
            self.celebrate();
        } else {
            self.ask(false);
        }
    }

    fn pick(&mut self, option: usize) {
        let Some(Task { play: Play::Quiz(quiz), .. }) = &mut self.task else { return };
        let Some(question) = quiz.items.get(quiz.at) else { return };
        if quiz.solved.is_some() || option >= 3 || quiz.tried[option] {
            return;
        }
        if option == question.right {
            quiz.solved = Some(0.0);
            let read = question.read.clone();
            let praise = self.one_of("praise");
            // Long enough for her to read the sentence out, and a moment to look at it.
            let wait = self.speak(praise) + self.say(&read) + 1.2;
            if let Some(Task { play: Play::Quiz(quiz), .. }) = &mut self.task {
                quiz.wait = wait.max(2.5);
            }
            self.marker = Some(Action::Next);
            self.play(Sound::Chime);
            self.feel(Mood::Cheer, CHEER);
            self.cheer();
        } else {
            quiz.tried[option] = true;
            let other = (0..3).find(|&i| !quiz.tried[i]).unwrap_or(question.right);
            self.marker = Some(Action::Pick(other));
            self.play(Sound::Oops);
            self.feel(Mood::Oops, 0.5);
            let comfort = self.one_of("comfort");
            self.speak(comfort);
        }
    }

    /// On from a finished story to the next, or to the party.
    fn next_question(&mut self) {
        let Some(Task { play: Play::Quiz(quiz), .. }) = &mut self.task else { return };
        if quiz.solved.is_none() {
            return;
        }
        quiz.at += 1;
        quiz.tried = [false; 3];
        quiz.solved = None;
        if quiz.at >= quiz.items.len() {
            self.celebrate();
        } else {
            self.marker = Some(Action::Pick(0));
            self.ask(false);
        }
    }

    /// The balls of yarn: rolls them on by `dt` seconds, sends in new ones, and ends
    /// the game when the last is caught.
    fn roll(&mut self, dt: f32) {
        let Some(Task { play: Play::Yarn(yarn), level, .. }) = &mut self.task else { return };
        let (crossing, most) = [(11.0, 1), (8.0, 2), (6.0, 3)][*level as usize];
        let mut missed = false;
        for ball in &mut yarn.balls {
            match &mut ball.popped {
                Some(age) => *age += dt,
                None => {
                    ball.x -= dt / crossing;
                    if ball.x <= 0.0 {
                        // Nothing is lost: it rolls round and comes in again.
                        ball.x = 1.0;
                        missed = true;
                    }
                }
            }
        }
        yarn.balls.retain(|ball| ball.popped.is_none_or(|age| age < BURST));
        yarn.next_in -= dt;
        loop {
            let rolling = yarn.balls.iter().filter(|ball| ball.popped.is_none()).count();
            // With nothing moving, the balls simply appear, part of the way along.
            if rolling >= most || yarn.caught + rolling >= yarn.goal || (yarn.next_in > 0.0 && self.animations) {
                break;
            }
            let mut letters: Vec<char> = ('A'..='Z').filter(|c| yarn.balls.iter().all(|ball| ball.letter != *c)).collect();
            self.rng.shuffle(&mut letters);
            let x = if self.animations { 1.0 } else { 0.3 + 0.3 * rolling as f32 };
            yarn.balls.push(Ball { letter: letters[0], x, color: self.rng.below(6), popped: None });
            yarn.next_in = crossing / most as f32;
        }
        let ended = match &mut yarn.done {
            Some(age) => {
                *age += dt;
                *age >= 0.9 || !self.animations
            }
            None => false,
        };
        if missed {
            self.play(Sound::Boing);
        }
        if ended {
            self.celebrate();
        }
    }

    /// A game is finished: a party, a gift, and a different party from the last one.
    fn celebrate(&mut self) {
        let mut funs: Vec<Fun> = Fun::ALL.into_iter().filter(|&fun| Some(fun) != self.last_fun).collect();
        self.rng.shuffle(&mut funs);
        let fun = funs[0];
        self.last_fun = Some(fun);
        let missing: Vec<usize> = (0..GIFTS.len()).filter(|&i| !self.gifts.earned[i]).collect();
        let gift = (!missing.is_empty()).then(|| self.rng.pick(&missing));
        match gift {
            Some(gift) => {
                self.gifts.earned[gift] = true;
                self.gifts.toggle(gift);
            }
            None => self.gifts.hearts += 1,
        }
        self.save_gifts();
        let headline = self.rng.pick(&HEADLINES);
        self.party = Some(Party { fun, headline, gift, age: 0.0, emit: 0.0 });
        self.screen = Screen::Party;
        self.phase_time = 0.0;
        self.particles.clear();
        self.marker = Some(Action::Again);
        let fanfare = self.rng.below(FANFARES as usize) as u8;
        self.play(Sound::Fanfare(fanfare));
        let cheer = self.one_of("party");
        self.speak(cheer);
        let present = if gift.is_some() { self.one_of("gift") } else { "heart" };
        self.say(&[present.to_string()]);
        self.feel(Mood::Cheer, PARTY);
    }

    fn dress(&mut self) {
        self.screen = Screen::Dress;
        self.task = None;
        self.party = None;
        self.particles.clear();
        self.marker = (0..GIFTS.len()).find(|&i| self.gifts.earned[i]).map(Action::Wear).or(Some(Action::Back));
        self.play(Sound::Click);
        self.speak(if self.gifts.count() == 0 { "dress-empty" } else { "dress-intro" });
    }

    fn wear(&mut self, gift: usize) {
        if gift >= GIFTS.len() || !self.gifts.earned[gift] {
            return;
        }
        self.gifts.toggle(gift);
        self.save_gifts();
        self.play(Sound::Star);
        if self.gifts.worn[gift] {
            self.feel(Mood::Love, 1.2);
            let pleased = self.one_of("dress");
            self.speak(pleased);
        }
    }

    pub fn act(&mut self, action: Action) {
        match action {
            Action::Play(game) => self.start(game),
            Action::Surprise => {
                let game = self.rng.pick(&Game::ALL);
                self.start(game);
            }
            Action::Dress => self.dress(),
            Action::Level(level) => {
                self.level = level;
                self.save_settings();
                self.play(Sound::Click);
            }
            Action::Theme => {
                self.theme = (self.theme + 1) % THEMES.len();
                self.save_settings();
                self.play(Sound::Click);
            }
            Action::Sound => {
                self.sound = !self.sound;
                self.save_settings();
                if !self.sound {
                    self.speaker.hush();
                    self.talk = 0.0;
                }
                self.play(Sound::Click);
            }
            Action::Motion => {
                self.animations = !self.animations;
                self.save_settings();
                self.particles.clear();
                self.play(Sound::Click);
            }
            Action::Help => {
                self.help = !self.help;
                self.play(Sound::Click);
            }
            Action::Quit => {
                if self.animations && self.screen != Screen::Bye {
                    self.screen = Screen::Bye;
                    self.phase_time = 0.0;
                    self.particles.clear();
                    self.speaker.hush();
                    self.talk = 0.0;
                    self.play(Sound::Bye);
                    self.speak("bye");
                    self.feel(Mood::Wave, BYE);
                } else {
                    self.quit = true;
                }
            }
            Action::Back => {
                self.speaker.hush();
                self.talk = 0.0;
                self.play(Sound::Click);
                self.home();
            }
            Action::Again => match &self.task {
                Some(task) => self.start(task.game),
                None => self.home(),
            },
            Action::Key(c) => self.on_char(c),
            Action::Pick(option) => self.pick(option),
            Action::Next => self.next_question(),
            Action::Repeat => self.ask(true),
            Action::Wear(gift) => self.wear(gift),
            Action::Skip => match self.screen {
                Screen::Intro => {
                    self.home();
                    // She has said hello, or says it now; then she asks.
                    let greeting = if self.intro_said { "what-do" } else { "hello" };
                    self.say(&[greeting.to_string()]);
                }
                Screen::Bye => self.quit = true,
                _ => {}
            },
        }
    }

    /// Whether the arrow keys can put the marker on a button.
    fn reachable(action: Action) -> bool {
        !matches!(action, Action::Key(_) | Action::Skip)
    }

    /// Moves the marker to the nearest button in a direction.
    fn step(&mut self, dx: i32, dy: i32) {
        let center = |r: Rect| (r.x as f32 + r.width as f32 / 2.0, (r.y as f32 + r.height as f32 / 2.0) * 2.0);
        let choices: Vec<(Rect, Action)> = self.buttons.iter().copied().filter(|&(_, action)| App::reachable(action)).collect();
        let Some(&(from, _)) = choices.iter().find(|&&(_, action)| Some(action) == self.marker) else {
            self.marker = choices.first().map(|&(_, action)| action);
            return;
        };
        let (fx, fy) = center(from);
        // Sideways, only along the row the marker is in: Left from the first game
        // must not land on the settings far below it. Up and down, what is more
        // ahead than aside, and when nothing is, whatever is ahead at all, so that
        // rows of different numbers of buttons can all be reached.
        let cones: &[f32] = if dx != 0 { &[f32::INFINITY] } else { &[0.5, 2.0] };
        for cone in cones {
            let mut best: Option<(f32, Action)> = None;
            for &(rect, action) in &choices {
                let (x, y) = center(rect);
                let (along, across) = if dx != 0 { ((x - fx) * dx as f32, (y - fy).abs()) } else { ((y - fy) * dy as f32, (x - fx).abs()) };
                let beside = dx == 0 || (rect.top() < from.bottom() && from.top() < rect.bottom());
                if beside && along > 0.5 && across <= along * cone + 1.0 {
                    let far = along + across * 2.5;
                    if best.is_none_or(|(nearest, _)| far < nearest) {
                        best = Some((far, action));
                    }
                }
            }
            if let Some((_, action)) = best {
                self.marker = Some(action);
                return;
            }
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if matches!(key.code, KeyCode::Char('c' | 'C')) {
                self.quit = true;
            }
            return;
        }
        if self.help {
            self.help = false;
            return;
        }
        if matches!(self.screen, Screen::Intro | Screen::Bye) {
            return self.act(Action::Skip);
        }
        // In the games that are typed, a letter is a letter and nothing else.
        if matches!(&self.task, Some(Task { play: Play::Typing(_) | Play::Yarn(_), .. }) if self.screen == Screen::Play) {
            match key.code {
                KeyCode::Esc => self.act(Action::Back),
                KeyCode::Tab => self.act(Action::Repeat),
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::ALT) => self.on_char(c),
                _ => {}
            }
            return;
        }
        // Easy story time asks for a letter, and the letter's own key answers. So
        // there too a letter is only a letter: not an arrow, not the theme.
        if self.screen == Screen::Play
            && let Some(Task { play: Play::Quiz(quiz), level: Level::Easy, .. }) = &self.task
            && let KeyCode::Char(c) = key.code
            && c.is_ascii_alphabetic()
        {
            let option = quiz.items.get(quiz.at).and_then(|question| question.options.iter().position(|o| o.eq_ignore_ascii_case(&c.to_string())));
            if let Some(option) = option {
                self.act(Action::Pick(option));
            }
            return;
        }
        // Everywhere else, vim's H J K L are the arrows.
        let code = match key.code {
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'h' => KeyCode::Left,
                'j' => KeyCode::Down,
                'k' => KeyCode::Up,
                'l' => KeyCode::Right,
                c => KeyCode::Char(c),
            },
            other => other,
        };
        match code {
            KeyCode::Left => return self.step(-1, 0),
            KeyCode::Right => return self.step(1, 0),
            KeyCode::Up => return self.step(0, -1),
            KeyCode::Down => return self.step(0, 1),
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Only a button that is on the screen: the marker may be from before.
                if let Some(action) = self.marker.filter(|marker| self.buttons.iter().any(|(_, action)| action == marker)) {
                    self.act(action);
                }
                return;
            }
            KeyCode::Char('t') => return self.act(Action::Theme),
            KeyCode::Char('s') => return self.act(Action::Sound),
            KeyCode::Char('m') => return self.act(Action::Motion),
            KeyCode::Char('?') | KeyCode::F(1) => return self.act(Action::Help),
            _ => {}
        }
        match (self.screen, code) {
            (Screen::Home, KeyCode::Char(c @ '1'..='4')) => self.act(Action::Play(Game::ALL[c as usize - '1' as usize])),
            (Screen::Home, KeyCode::Char('5' | 'd')) => self.act(Action::Dress),
            (Screen::Home, KeyCode::Char('6')) => self.act(Action::Surprise),
            (Screen::Home, KeyCode::Char('e')) => {
                let next = Level::ALL[(self.level as usize + 1) % Level::ALL.len()];
                self.act(Action::Level(next));
            }
            // Esc does not leave the game: on the home screen only Q does.
            (Screen::Home, KeyCode::Char('q')) => self.act(Action::Quit),
            (Screen::Play, KeyCode::Char(c @ ('a'..='c' | '1'..='3'))) => {
                self.act(Action::Pick(if c.is_ascii_digit() { c as usize - '1' as usize } else { c as usize - 'a' as usize }))
            }
            (Screen::Play, KeyCode::Tab) => self.act(Action::Repeat),
            (Screen::Play | Screen::Party | Screen::Dress, KeyCode::Esc) => self.act(Action::Back),
            _ => {}
        }
    }

    pub fn on_mouse(&mut self, mouse: MouseEvent) {
        let at = Position::new(mouse.column, mouse.row);
        let under = self.buttons.iter().rev().find(|(rect, _)| rect.contains(at)).map(|&(_, action)| action);
        // A click counts when the button goes down. Should a terminal only ever report
        // it coming up, that counts instead. (The sister projects do the same.)
        let click = match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.button_down = true;
                true
            }
            MouseEventKind::Up(MouseButton::Left) => !std::mem::take(&mut self.button_down),
            _ => false,
        };
        if click {
            if self.help {
                self.help = false;
            } else if matches!(self.screen, Screen::Intro | Screen::Bye) {
                self.act(Action::Skip);
            } else if let Some(action) = under {
                self.act(action);
            }
        } else if mouse.kind == MouseEventKind::Moved {
            // The marker follows the pointer, so that there is one marker, not two.
            if let Some(action) = under.filter(|&action| App::reachable(action) && action != Action::Help) {
                self.marker = Some(action);
            }
        }
    }

    /// Whether something is moving that should be drawn smoothly. She always moves a
    /// little, but her tail does not need sixty pictures a second.
    pub fn animating(&self) -> bool {
        self.animations
            && (!self.particles.is_empty()
                || self.mood != Mood::Calm
                || matches!(self.screen, Screen::Intro | Screen::Bye)
                || self.party.as_ref().is_some_and(|party| party.age < PARTY)
                || matches!(&self.task, Some(Task { play: Play::Yarn(_), .. }) if self.screen == Screen::Play)
                || matches!(&self.task, Some(Task { play: Play::Typing(Typing { cheer: Some(_), .. }), .. })))
    }

    /// Lets time pass. Returns whether the screen should be drawn again.
    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let passed = self.last_tick.map_or(0.0, |last| now.duration_since(last).as_secs_f32()).min(0.1);
        self.last_tick = Some(now);
        self.speaker.poll();
        self.advance(passed)
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        self.phase_time += dt;
        self.talk = (self.talk - dt).max(0.0);
        if self.mood != Mood::Calm {
            self.mood_left -= dt;
            if self.mood_left <= 0.0 {
                self.mood = Mood::Calm;
            }
        }
        if !self.animations {
            // Nothing moves, so there is nothing new to draw until a key is pressed.
            return false;
        }
        self.time += dt;

        match self.screen {
            Screen::Intro => {
                if !self.intro_said && self.phase_time >= 1.2 {
                    self.intro_said = true;
                    self.say(&["hello".to_string(), "hello-fun".to_string()]);
                    self.confetti(60);
                }
                if self.phase_time >= INTRO {
                    self.act(Action::Skip);
                }
            }
            Screen::Bye if self.phase_time >= BYE => self.quit = true,
            Screen::Play => self.play_on(dt),
            Screen::Party => self.party_on(dt),
            _ => {}
        }

        let (wide, floor) = (self.size.0 as f32, self.size.1 as f32);
        for p in &mut self.particles {
            p.age += dt;
            p.vy += p.gravity * dt;
            p.x += (p.vx + p.sway * (p.age * 3.0).sin()) * dt;
            p.y += p.vy * dt;
        }
        self.particles.retain(|p| p.age < p.life && p.y < floor + 4.0 && p.y > -6.0 && p.x > -8.0 && p.x < wide + 8.0);
        true
    }

    /// A game in play, `dt` seconds on.
    fn play_on(&mut self, dt: f32) {
        let Some(task) = &mut self.task else { return };
        let game = task.game;
        match &mut task.play {
            Play::Typing(typing) => {
                let Some(age) = &mut typing.cheer else { return };
                let before = *age;
                *age += dt;
                let now = *age;
                // The treat has reached her mouth.
                if game == Game::Snack && before < FLY && now >= FLY {
                    self.play(Sound::Munch);
                    self.feel(Mood::Munch, CHEER - FLY);
                    self.cheer();
                }
                if now >= CHEER {
                    self.next_one();
                }
            }
            Play::Yarn(_) => self.roll(dt),
            Play::Quiz(quiz) => {
                if let Some(age) = &mut quiz.solved {
                    *age += dt;
                    if *age >= quiz.wait {
                        self.next_question();
                    }
                }
            }
        }
    }

    /// A party, `dt` seconds on: more of whatever it is throwing in the air.
    fn party_on(&mut self, dt: f32) {
        let Some(party) = &mut self.party else { return };
        let before = party.age;
        party.age += dt;
        if party.age >= PARTY {
            return;
        }
        // The gift is unwrapped a moment after the tune begins.
        if party.gift.is_some() && before < 0.9 && party.age >= 0.9 {
            self.play(Sound::Gift);
        }
        let Some(party) = &mut self.party else { return };
        party.emit -= dt;
        let (fun, age) = (party.fun, party.age);
        let mut due = 0;
        while party.emit <= 0.0 {
            party.emit += fun.every();
            due += 1;
        }
        for _ in 0..due {
            self.throw(fun, age);
        }
    }

    fn colors(&self) -> [Rgb; 6] {
        self.theme().buttons
    }

    fn add(&mut self, particle: Particle) {
        // Enough is as good as a feast, and a slow terminal has to draw them all.
        if self.particles.len() < 600 {
            self.particles.push(particle);
        }
    }

    /// One more of what a party throws.
    fn throw(&mut self, fun: Fun, age: f32) {
        let (w, h) = (self.size.0 as f32, self.size.1 as f32);
        let colors = self.colors();
        let color = self.rng.pick(&colors);
        let still = Particle { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, gravity: 0.0, sway: 0.0, age: 0.0, life: 1.0, color, symbol: "●", front: false };
        match fun {
            Fun::Confetti => {
                let symbol = self.rng.pick(&["■", "●", "▲", "◆", "★"]);
                let particle = Particle {
                    x: self.rng.unit() * w,
                    y: -1.0,
                    vx: self.rng.between(-3.0, 3.0),
                    vy: self.rng.between(5.0, 14.0),
                    gravity: 3.0,
                    life: 8.0,
                    symbol,
                    ..still
                };
                self.add(particle);
            }
            Fun::Fireworks => {
                let (x, y) = (self.rng.between(0.12, 0.88) * w, self.rng.between(0.1, 0.5) * h);
                for i in 0..24 {
                    let turn = i as f32 / 24.0 * TAU;
                    let speed = self.rng.between(7.0, 15.0);
                    let particle = Particle {
                        x,
                        y,
                        vx: turn.cos() * speed * 2.0,
                        vy: turn.sin() * speed,
                        gravity: 9.0,
                        life: self.rng.between(0.8, 1.4),
                        symbol: if i % 3 == 0 { "★" } else { "✦" },
                        ..still
                    };
                    self.add(particle);
                }
                self.play(Sound::Pop);
            }
            Fun::Balloons => {
                let particle = Particle {
                    x: self.rng.between(2.0, w - 6.0),
                    y: h + 1.0,
                    vy: -self.rng.between(5.0, 9.0),
                    sway: 2.5,
                    life: 10.0,
                    symbol: BALLOON,
                    ..still
                };
                self.add(particle);
            }
            Fun::Hearts => {
                let color = self.rng.pick(&[(255, 105, 180), (255, 64, 129), (240, 98, 146), (255, 150, 195)]);
                let particle = Particle { x: self.rng.unit() * w, y: h, vy: -self.rng.between(4.0, 10.0), sway: 3.0, life: 8.0, symbol: "♥", color, ..still };
                self.add(particle);
            }
            Fun::Stars => {
                let symbol = self.rng.pick(&["★", "✦", "★", "·"]);
                let color = self.rng.pick(&[(255, 214, 0), (255, 235, 120), color]);
                let particle = Particle {
                    x: self.rng.between(-0.4, 1.0) * w,
                    y: -1.0,
                    vx: self.rng.between(14.0, 22.0),
                    vy: self.rng.between(7.0, 12.0),
                    life: 8.0,
                    symbol,
                    color,
                    ..still
                };
                self.add(particle);
            }
            Fun::Bubbles => {
                let color = self.rng.pick(&[(120, 195, 255), (160, 215, 255), (190, 170, 255), (140, 225, 225)]);
                let symbol = self.rng.pick(&["○", "o", "°", "O"]);
                let particle = Particle { x: self.rng.unit() * w, y: h, vy: -self.rng.between(2.5, 6.0), sway: 4.0, life: 10.0, symbol, color, ..still };
                self.add(particle);
            }
            // The rainbow itself is drawn by `ui`; these twinkle around it.
            Fun::Rainbow | Fun::Glitter => {
                let symbol = self.rng.pick(&["✦", "✧", "·", "*"]);
                let color = if fun == Fun::Glitter { self.rng.pick(&[(255, 214, 0), (255, 240, 170), color]) } else { color };
                let particle = Particle { x: self.rng.unit() * w, y: self.rng.unit() * h, life: self.rng.between(0.3, 0.8), symbol, color, ..still };
                self.add(particle);
            }
            Fun::Dance => {
                let (x, y) = (self.kitty.x as f32 + self.kitty.width as f32 / 2.0, self.kitty.y as f32 + 2.0);
                let symbol = self.rng.pick(&["♪", "♫"]);
                let particle = Particle { x, y, vx: self.rng.between(-16.0, 16.0), vy: -self.rng.between(2.0, 8.0), sway: 3.0, life: 3.0, symbol, ..still };
                self.add(particle);
            }
            Fun::Flowers => {
                // A flower on a stem, somewhere along the bottom, there to stay.
                let (x, tall) = (self.rng.unit() * w, 1 + self.rng.below(3));
                for i in 0..tall {
                    self.add(Particle { x, y: h - 1.0 - i as f32, life: 60.0, symbol: "|", color: (70, 170, 90), ..still });
                }
                let symbol = self.rng.pick(&["✿", "❀"]);
                self.add(Particle { x, y: h - 1.0 - tall as f32, life: 60.0, symbol, ..still });
            }
            Fun::Fish => {
                let color = self.rng.pick(&[(60, 150, 235), (255, 150, 60), (90, 200, 190), color]);
                let particle =
                    Particle { x: -3.0, y: self.rng.between(1.0, h - 2.0), vx: self.rng.between(10.0, 20.0), life: 12.0, symbol: "><>", color, ..still };
                self.add(particle);
            }
            Fun::Spiral => {
                let turn = age * 7.0;
                let particle = Particle { x: w / 2.0, y: h / 2.0, vx: turn.cos() * 26.0, vy: turn.sin() * 13.0, life: 2.5, symbol: "★", ..still };
                self.add(particle);
            }
        }
    }

    /// Something lying still at a place, in front of everything: for testing where
    /// such things may be drawn.
    #[cfg(test)]
    pub fn throw_at(&mut self, x: f32, y: f32) {
        let color = self.colors()[0];
        self.particles.push(Particle { x, y, vx: 0.0, vy: 0.0, gravity: 0.0, sway: 0.0, age: 0.0, life: 9.0, color, symbol: "★", front: true });
    }

    /// Confetti from the top of the window.
    fn confetti(&mut self, pieces: usize) {
        for _ in 0..pieces {
            self.throw(Fun::Confetti, 0.0);
            if let Some(particle) = self.particles.last_mut() {
                particle.y = -self.rng.unit() * self.size.1 as f32 * 0.6;
            }
        }
    }

    /// Things flying out from a place, in every direction.
    fn burst(&mut self, x: f32, y: f32, pieces: usize, symbols: &[&'static str], speed: f32, gravity: f32) {
        let colors = self.colors();
        for i in 0..pieces {
            let turn = i as f32 / pieces as f32 * TAU;
            let speed = speed * self.rng.between(0.5, 1.0);
            // A cell is twice as tall as it is wide, so half the speed downwards.
            let particle = Particle {
                x,
                y,
                vx: turn.cos() * speed * 2.0,
                vy: turn.sin() * speed,
                gravity,
                sway: 0.0,
                age: 0.0,
                life: self.rng.between(0.5, 0.9),
                color: colors[i % colors.len()],
                symbol: symbols[i % symbols.len()],
                front: true,
            };
            self.add(particle);
        }
    }

    /// One of the little celebrations, around Pink Kitty, and not the last one again.
    fn cheer(&mut self) {
        if !self.animations {
            return;
        }
        let mut kinds: Vec<Cheer> = Cheer::ALL.into_iter().filter(|&kind| Some(kind) != self.last_cheer).collect();
        self.rng.shuffle(&mut kinds);
        let kind = kinds[0];
        self.last_cheer = Some(kind);
        let (left, top, wide, high) = (self.kitty.x as f32, self.kitty.y as f32, self.kitty.width as f32, self.kitty.height as f32);
        let (x, y) = (left + wide / 2.0, top + high / 2.0);
        let colors = self.colors();
        let still = Particle { x, y: top, vx: 0.0, vy: 0.0, gravity: 0.0, sway: 0.0, age: 0.0, life: 1.2, color: colors[0], symbol: "★", front: true };
        match kind {
            Cheer::Sparkle => self.burst(x, y, 18, &["★", "✦"], 14.0, 14.0),
            Cheer::Ring => self.burst(x, y, 22, &["●"], 16.0, 0.0),
            Cheer::Hearts => {
                for _ in 0..12 {
                    let color = self.rng.pick(&[(255, 105, 180), (255, 64, 129), (255, 150, 195)]);
                    let particle = Particle {
                        x: left + self.rng.unit() * wide,
                        y: top + self.rng.unit() * high * 0.5,
                        vy: -self.rng.between(4.0, 9.0),
                        sway: 3.0,
                        symbol: "♥",
                        color,
                        ..still
                    };
                    self.add(particle);
                }
            }
            Cheer::Fountain => {
                for i in 0..20 {
                    let particle = Particle {
                        vx: self.rng.between(-14.0, 14.0),
                        vy: -self.rng.between(9.0, 17.0),
                        gravity: 24.0,
                        color: colors[i % 6],
                        symbol: if i % 2 == 0 { "★" } else { "✦" },
                        ..still
                    };
                    self.add(particle);
                }
            }
            Cheer::Notes => {
                for i in 0..10 {
                    let particle = Particle {
                        vx: self.rng.between(-12.0, 18.0),
                        vy: -self.rng.between(2.0, 7.0),
                        sway: 3.0,
                        color: colors[i % 6],
                        symbol: if i % 2 == 0 { "♪" } else { "♫" },
                        ..still
                    };
                    self.add(particle);
                }
            }
            Cheer::Cannons => {
                let (w, h) = (self.size.0 as f32, self.size.1 as f32);
                for i in 0..28 {
                    let from_left = i % 2 == 0;
                    let particle = Particle {
                        x: if from_left { 0.0 } else { w - 1.0 },
                        y: h - 1.0,
                        vx: self.rng.between(10.0, 30.0) * if from_left { 1.0 } else { -1.0 },
                        vy: -self.rng.between(10.0, 22.0),
                        gravity: 22.0,
                        life: 1.6,
                        color: colors[i % 6],
                        symbol: ["■", "●", "▲", "◆"][i % 4],
                        ..still
                    };
                    self.add(particle);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new(true, Settings::default(), Gifts::default());
        app.rng = Rng::new(7);
        app
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    fn typing(app: &App) -> &Typing {
        let Some(Task { play: Play::Typing(typing), .. }) = &app.task else { panic!("not typing") };
        typing
    }

    fn yarn(app: &App) -> &Yarn {
        let Some(Task { play: Play::Yarn(yarn), .. }) = &app.task else { panic!("not yarn") };
        yarn
    }

    fn quiz(app: &App) -> &Quiz {
        let Some(Task { play: Play::Quiz(quiz), .. }) = &app.task else { panic!("not a story") };
        quiz
    }

    /// A folder of its own for a test's files.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("funkitty-app-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn gifts_survive_a_round_trip_and_bad_input() {
        let mut gifts = Gifts::default();
        assert_eq!(Gifts::parse(&gifts.format()), gifts);
        gifts.earned = [true; GIFTS.len()];
        gifts.hearts = 4;
        // One scarf at a time: the second takes the first one's place.
        let scarves: Vec<usize> = (0..GIFTS.len()).filter(|&i| GIFTS[i].scarf).collect();
        gifts.toggle(scarves[0]);
        gifts.toggle(0);
        gifts.toggle(scarves[1]);
        assert!(!gifts.worn[scarves[0]] && gifts.worn[scarves[1]] && gifts.worn[0]);
        gifts.toggle(0);
        assert!(!gifts.worn[0]);
        assert_eq!(Gifts::parse(&gifts.format()), gifts);
        assert_eq!(gifts.count(), GIFTS.len());

        // Nonsense is left out, and nothing is worn that was not given.
        let odd = Gifts::parse("earned=crown,teapot,,bell\nworn=crown,flower\nhearts=many\nmore=1\n");
        assert_eq!((odd.count(), odd.hearts), (2, 0));
        assert_eq!(odd.worn.iter().filter(|&&w| w).count(), 1);
        let mut none = Gifts::default();
        none.toggle(3);
        assert_eq!(none, Gifts::default());

        let dir = scratch("gifts");
        let path = dir.join("deeper/gifts");
        assert_eq!(Gifts::load(&path), Gifts::default());
        gifts.save(&path);
        assert_eq!(Gifts::load(&path), gifts);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn snack_time_from_the_first_word_to_the_party() {
        let mut app = app();
        app.level = Level::Medium;
        press(&mut app, KeyCode::Char('1'));
        assert_eq!((app.screen, app.bubble.as_str()), (Screen::Play, "I'm hungry! Can you feed me?"));
        assert_eq!(app.speaker.said, ["snack-intro"]);
        for round in 0..5 {
            let word = typing(&app).items[round].clone();
            assert!((3..=4).contains(&word.len()) && word == word.to_uppercase(), "{word}");
            assert_eq!(app.task.as_ref().unwrap().progress(), (round, 5));
            // Medium does not give the key away until one goes wrong.
            assert_eq!(app.wanted(), None);
            let wrong = if word.starts_with('Z') { 'q' } else { 'z' };
            press(&mut app, KeyCode::Char(wrong));
            assert_eq!((typing(&app).typed, typing(&app).misses, app.mood), (0, 1, Mood::Oops));
            assert_eq!((app.speaker.heard.last(), app.wanted()), (Some(&Sound::Oops), word.chars().next()));
            // Digits and the like are not letters gone wrong.
            press(&mut app, KeyCode::Char('7'));
            assert_eq!(typing(&app).misses, 1);
            // Either case will do.
            type_text(&mut app, &word.to_lowercase());
            assert!(typing(&app).cheer.is_some());
            assert_eq!(app.task.as_ref().unwrap().progress(), (round + 1, 5));
            assert!(app.bubble == words::shown(app.last_phrase) && app.last_phrase.starts_with("yum-"));
            // Nothing is typed while she eats.
            press(&mut app, KeyCode::Char('a'));
            assert_eq!(typing(&app).typed, word.len());
            app.advance(FLY + 0.01);
            assert_eq!((app.speaker.heard.last(), app.mood), (Some(&Sound::Munch), Mood::Munch));
            assert!(!app.particles.is_empty() && app.animating());
            app.advance(CHEER);
        }
        assert_eq!(app.screen, Screen::Party);
        let party = app.party.as_ref().unwrap();
        let gift = party.gift.unwrap();
        assert!(app.gifts.earned[gift] && app.gifts.worn[gift] && app.gifts.count() == 1);
        assert!(HEADLINES.contains(&party.headline) && app.mood == Mood::Cheer);
        assert!(app.speaker.heard.iter().any(|s| matches!(s, Sound::Fanfare(_))));
        let said = &app.speaker.said;
        assert!(said[said.len() - 2].starts_with("party-") && said[said.len() - 1].starts_with("gift-"), "{said:?}");
        // The gift is unwrapped a moment later.
        app.advance(1.0);
        assert!(app.speaker.heard.contains(&Sound::Gift));
    }

    #[test]
    fn hard_snack_time_is_whole_sentences_with_spaces() {
        let mut app = app();
        app.level = Level::Hard;
        app.start(Game::Snack);
        assert_eq!(typing(&app).items.len(), 3);
        let sentence = typing(&app).items[0].clone();
        assert!(sentence.contains(' ') && words::sentences().contains(&sentence.to_lowercase().as_str()));
        type_text(&mut app, &sentence.to_lowercase());
        assert!(typing(&app).cheer.is_some());
    }

    #[test]
    fn kitty_says_what_to_type_and_says_it_again() {
        let mut app = app();
        app.start(Game::Says);
        let letter = typing(&app).items[0].clone();
        assert_eq!(letter.len(), 1);
        assert_eq!(app.bubble, format!("Can you find the letter {letter}?"));
        assert_eq!(app.speaker.said, ["says-intro".to_string(), "find".into(), words::letter_voice(letter.chars().next().unwrap())]);
        // Easy lights the key up from the start.
        assert_eq!(app.wanted(), letter.chars().next());
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.speaker.said.len(), 5);
        type_text(&mut app, &letter);
        assert_eq!((app.speaker.heard.last(), app.mood), (Some(&Sound::Chime), Mood::Cheer));
        assert!(app.last_phrase.starts_with("praise-") && !app.particles.is_empty());
        app.advance(CHEER + 0.01);
        assert_eq!(typing(&app).at, 1);
        let next = typing(&app).items[1].clone();
        assert_eq!(app.speaker.said.last(), Some(&words::letter_voice(next.chars().next().unwrap())));

        app.level = Level::Medium;
        app.start(Game::Says);
        let word = typing(&app).items[0].clone();
        assert_eq!(app.bubble, format!("Can you type {word}?"));
        assert_eq!(app.speaker.said.last(), Some(&words::word_voice(&word)));
        assert!(!typing(&app).hidden && !app.concealed());
    }

    #[test]
    fn hard_words_are_heard_and_not_shown_until_that_proves_too_hard() {
        let mut app = app();
        app.level = Level::Hard;
        app.start(Game::Says);
        let word = typing(&app).items[0].clone();
        assert!(typing(&app).hidden && word.len() >= 5);
        // With nothing to hear her through, the word has to be shown.
        assert!(!app.voiced() && !app.concealed());
        // A player that is never there is as good as one that is, for this.
        let dir = scratch("says");
        app.speaker = Speaker::through(dir.join("no-such-player"), &[], dir.join("sounds"));
        assert!(app.voiced() && app.concealed());
        assert_eq!(app.bubble, "Listen, and type what I say.");
        assert_eq!(app.wanted(), None);
        let wrong = if word.starts_with('Z') { 'q' } else { 'z' };
        press(&mut app, KeyCode::Char(wrong));
        assert!(app.concealed() && app.wanted().is_none());
        press(&mut app, KeyCode::Char(wrong));
        assert!(!app.concealed());
        assert_eq!(app.wanted(), word.chars().next());
        // With the sound switched off it is shown as well.
        app.start(Game::Says);
        assert!(app.concealed());
        app.sound = false;
        assert!(!app.concealed());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn yarn_balls_roll_round_again_until_they_are_caught() {
        let mut app = app();
        app.start(Game::Yarn);
        assert!(yarn(&app).balls.is_empty() && app.animating());
        app.advance(0.5);
        assert_eq!(yarn(&app).balls.len(), 1);
        let letter = yarn(&app).balls[0].letter;
        assert_eq!(app.wanted(), Some(letter));
        // Easy: one ball at a time.
        app.advance(3.0);
        assert_eq!(yarn(&app).balls.len(), 1);
        assert!(yarn(&app).balls[0].x < 0.8);
        // Past her it goes, and in again at the far side. Nothing is lost.
        app.advance(9.0);
        assert!(yarn(&app).balls[0].x > 0.9 && app.speaker.heard.contains(&Sound::Boing));
        let other = if letter == 'Q' { 'w' } else { 'q' };
        press(&mut app, KeyCode::Char(other));
        assert_eq!((yarn(&app).caught, app.speaker.heard.last()), (0, Some(&Sound::Oops)));
        press(&mut app, KeyCode::Char(letter.to_ascii_lowercase()));
        assert_eq!((yarn(&app).caught, app.speaker.heard.last(), app.mood), (1, Some(&Sound::Pop), Mood::Pounce));
        assert_ne!(app.hop(), (0.0, 0.0));
        app.advance(BURST + 0.01);
        assert!(yarn(&app).balls.iter().all(|ball| ball.popped.is_none()));
        let goal = yarn(&app).goal;
        for caught in 1..goal {
            // The next one comes soon after a catch.
            app.advance(0.6);
            let letter = yarn(&app).balls.iter().find(|ball| ball.popped.is_none()).expect("a ball").letter;
            press(&mut app, KeyCode::Char(letter));
            assert_eq!(yarn(&app).caught, caught + 1);
        }
        assert!(app.speaker.said.iter().any(|clip| clip.starts_with("pounce-")));
        assert_eq!(app.screen, Screen::Play);
        app.advance(1.0);
        assert_eq!(app.screen, Screen::Party);

        // Hard has three rolling at once, each with its own letter.
        app.level = Level::Hard;
        app.start(Game::Yarn);
        for _ in 0..40 {
            app.advance(0.25);
        }
        let letters: std::collections::HashSet<char> = yarn(&app).balls.iter().map(|ball| ball.letter).collect();
        assert_eq!((yarn(&app).balls.len(), letters.len(), app.wanted()), (3, 3, None));
    }

    #[test]
    fn story_time_wrong_words_cost_nothing_and_the_right_one_is_read_out() {
        let mut app = app();
        app.level = Level::Medium;
        press(&mut app, KeyCode::Char('4'));
        assert_eq!((app.marker, quiz(&app).items.len()), (Some(Action::Pick(0)), 3));
        let question = &quiz(&app).items[0];
        let (right, text) = (question.right, question.text.clone());
        let story = words::stories().iter().find(|s| s.text == text).unwrap();
        assert_eq!(question.options[right], story.options[0]);
        let wrong = (right + 1) % 3;
        press(&mut app, KeyCode::Char((b'a' + wrong as u8) as char));
        assert!(quiz(&app).tried[wrong] && quiz(&app).solved.is_none());
        assert_eq!((app.speaker.heard.last(), app.mood), (Some(&Sound::Oops), Mood::Oops));
        assert!(app.last_phrase.starts_with("comfort-") && app.marker != Some(Action::Pick(wrong)));
        // The same wrong word again does nothing more.
        let said = app.speaker.said.len();
        press(&mut app, KeyCode::Char((b'1' + wrong as u8) as char));
        assert_eq!(app.speaker.said.len(), said);
        app.act(Action::Next);
        assert_eq!(quiz(&app).at, 0);

        press(&mut app, KeyCode::Char((b'1' + right as u8) as char));
        assert!(quiz(&app).solved.is_some() && app.marker == Some(Action::Next));
        assert_eq!(app.speaker.said.last(), Some(&story.voice()));
        assert_eq!(app.task.as_ref().unwrap().progress(), (1, 3));
        // She has time to read it out before the next one comes.
        let wait = quiz(&app).wait;
        assert!(wait > 2.5, "{wait}");
        app.advance(wait - 0.2);
        assert_eq!(quiz(&app).at, 0);
        app.advance(0.3);
        assert_eq!((quiz(&app).at, app.marker, app.bubble.as_str()), (1, Some(Action::Pick(0)), "Which word fits?"));
        // Or the player goes on without waiting.
        app.act(Action::Pick(quiz(&app).items[1].right));
        app.act(Action::Next);
        assert_eq!(quiz(&app).at, 2);
        app.act(Action::Pick(quiz(&app).items[2].right));
        app.act(Action::Next);
        assert_eq!(app.screen, Screen::Party);
    }

    #[test]
    fn easy_story_time_asks_what_letter_a_word_starts_with() {
        let mut app = app();
        app.start(Game::Story);
        assert_eq!(app.bubble, "What does this word start with?");
        for round in 0..3 {
            let question = &quiz(&app).items[round];
            let word = question.text.split(' ').next().unwrap().to_string();
            assert!(words::short_words().contains(&word.to_lowercase().as_str()) && question.text.ends_with(BLANK));
            assert_eq!(question.options[question.right], word[..1]);
            let letters: std::collections::HashSet<&String> = question.options.iter().collect();
            assert!(letters.len() == 3 && question.options.iter().all(|o| o.len() == 1));
            // She says the word, and when it is right, the letter and the word.
            assert_eq!(app.speaker.said[app.speaker.said.len() - 2..], ["starts".to_string(), words::word_voice(&word)]);
            // The letter's own key answers, and no other letter does anything at all:
            // not T for the theme, not H for the arrow.
            let (right, theme, marker) = (question.right, app.theme, app.marker);
            let wrong = ('a'..='z').find(|c| !quiz(&app).items[round].options.contains(&c.to_ascii_uppercase().to_string())).unwrap();
            type_text(&mut app, &format!("{wrong}"));
            assert!(quiz(&app).solved.is_none() && quiz(&app).tried == [false; 3] && (app.theme, app.marker) == (theme, marker));
            type_text(&mut app, &word[..1].to_lowercase());
            assert!(quiz(&app).solved.is_some() && quiz(&app).items[round].right == right);
            assert_eq!(app.speaker.said[app.speaker.said.len() - 2..], [words::letter_voice(word.chars().next().unwrap()), words::word_voice(&word)]);
            app.act(Action::Next);
        }
        assert_eq!(app.screen, Screen::Party);
    }

    #[test]
    fn no_two_parties_running_are_the_same_and_every_game_wins_a_gift() {
        let mut app = app();
        let mut funs = Vec::new();
        for round in 0..200 {
            app.start(Game::Says);
            app.celebrate();
            let party = app.party.as_ref().unwrap();
            funs.push(party.fun);
            if round < GIFTS.len() {
                assert_eq!((app.gifts.count(), app.gifts.hearts), (round + 1, 0));
                assert!(app.gifts.worn[party.gift.unwrap()]);
            } else {
                // She has everything: a heart instead.
                assert_eq!((party.gift, app.gifts.hearts as usize), (None, round + 1 - GIFTS.len()));
                assert_eq!(app.speaker.said.last().map(String::as_str), Some("heart"));
            }
        }
        assert!(funs.windows(2).all(|pair| pair[0] != pair[1]));
        assert!(Fun::ALL.iter().all(|fun| funs.contains(fun)));
        assert_eq!(app.gifts.worn.iter().zip(&GIFTS).filter(|&(&worn, gift)| worn && gift.scarf).count(), 1);
    }

    #[test]
    fn every_kind_of_party_throws_something_and_then_stops() {
        for fun in Fun::ALL {
            let mut app = app();
            app.start(Game::Says);
            app.celebrate();
            app.party.as_mut().unwrap().fun = fun;
            app.advance(0.2);
            assert!(!app.particles.is_empty(), "{fun:?}");
            assert!(app.particles.iter().all(|p| !p.symbol.is_empty()) && app.animating());
            if fun == Fun::Fireworks {
                assert_eq!(app.speaker.heard.last(), Some(&Sound::Pop));
            }
            for _ in 0..100 {
                app.advance(0.1);
            }
            assert!(app.particles.len() <= 600, "{fun:?}: {}", app.particles.len());
            let after = app.party.as_ref().unwrap().age;
            assert!(after >= PARTY);
            // Whatever is still in the air comes down, and nothing more goes up.
            let left = app.particles.len();
            app.advance(0.1);
            assert!(app.particles.len() <= left, "{fun:?}");
            // "Play again" is the same game; "More games" is the home screen.
            app.act(Action::Again);
            assert!(app.screen == Screen::Play && app.task.as_ref().unwrap().game == Game::Says && app.particles.is_empty());
            app.celebrate();
            app.act(Action::Back);
            assert!(app.screen == Screen::Home && app.task.is_none() && app.party.is_none());
            assert_eq!(app.marker, Some(Action::Play(Game::Says)));
        }
    }

    #[test]
    fn the_little_celebrations_take_turns() {
        let mut app = app();
        let mut kinds = Vec::new();
        for _ in 0..60 {
            app.particles.clear();
            app.cheer();
            assert!(!app.particles.is_empty() && app.particles.iter().all(|p| p.front));
            kinds.push(app.last_cheer.unwrap());
        }
        assert!(kinds.windows(2).all(|pair| pair[0] != pair[1]));
        assert!(Cheer::ALL.iter().all(|kind| kinds.contains(kind)));
        // With nothing to move, nothing is thrown.
        app.animations = false;
        app.particles.clear();
        app.cheer();
        assert!(app.particles.is_empty());
    }

    #[test]
    fn the_dressing_room_puts_gifts_on_and_off_and_remembers() {
        let dir = scratch("dress");
        let mut app = app();
        app.gifts_path = Some(dir.join("gifts"));
        press(&mut app, KeyCode::Char('5'));
        assert_eq!((app.screen, app.marker, app.bubble.as_str()), (Screen::Dress, Some(Action::Back), "Play a game to win a gift!"));
        app.act(Action::Wear(2));
        assert_eq!(app.gifts, Gifts::default());
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.screen, Screen::Home);

        app.start(Game::Says);
        app.celebrate();
        let gift = app.party.as_ref().unwrap().gift.unwrap();
        assert_eq!(Gifts::load(&dir.join("gifts")), app.gifts);
        app.act(Action::Dress);
        assert_eq!((app.marker, app.bubble.as_str()), (Some(Action::Wear(gift)), "Let's dress up!"));
        app.act(Action::Wear(gift));
        assert!(!app.gifts.worn[gift] && app.speaker.heard.last() == Some(&Sound::Star));
        assert_eq!(Gifts::load(&dir.join("gifts")), app.gifts);
        app.act(Action::Wear(gift));
        assert!(app.gifts.worn[gift] && app.mood == Mood::Love && app.last_phrase.starts_with("dress-"));
        assert_eq!(app.look().eyes, Eyes::Hearts);
        assert!(app.look().worn[gift]);
        app.act(Action::Wear(99));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_keys_of_the_home_screen() {
        let dir = scratch("keys");
        let mut app = app();
        app.settings_path = Some(dir.join("settings"));
        // Esc does not leave the game.
        press(&mut app, KeyCode::Esc);
        assert!(!app.quit && app.screen == Screen::Home);
        for (key, game) in [('1', Game::Snack), ('2', Game::Says), ('3', Game::Yarn), ('4', Game::Story)] {
            press(&mut app, KeyCode::Char(key));
            assert_eq!(app.task.as_ref().map(|task| task.game), Some(game));
            press(&mut app, KeyCode::Esc);
            assert_eq!((app.screen, app.marker), (Screen::Home, Some(Action::Play(game))));
        }
        press(&mut app, KeyCode::Char('6'));
        assert_eq!(app.screen, Screen::Play);
        press(&mut app, KeyCode::Esc);

        press(&mut app, KeyCode::Char('e'));
        assert_eq!(app.level, Level::Medium);
        press(&mut app, KeyCode::Char('E'));
        press(&mut app, KeyCode::Char('e'));
        assert_eq!(app.level, Level::Easy);
        press(&mut app, KeyCode::Char('e'));
        press(&mut app, KeyCode::Char('t'));
        press(&mut app, KeyCode::Char('s'));
        press(&mut app, KeyCode::Char('m'));
        let saved = Settings::load(&dir.join("settings"));
        assert_eq!(saved, Settings { theme: 1, sound: false, animations: false, level: Level::Medium, ..Settings::default() });
        press(&mut app, KeyCode::Char('?'));
        assert!(app.help);
        // Any key puts the help away, and does nothing else.
        press(&mut app, KeyCode::Char('q'));
        assert!(!app.help && !app.quit && app.screen == Screen::Home);

        // With nothing moving, Q leaves at once. Otherwise she waves goodbye first.
        press(&mut app, KeyCode::Char('q'));
        assert!(app.quit);
        let mut app = self::app();
        press(&mut app, KeyCode::Char('Q'));
        assert_eq!((app.screen, app.quit, app.bubble.as_str(), app.mood), (Screen::Bye, false, "Bye bye! Come back soon!", Mood::Wave));
        assert_eq!(app.speaker.said.last().map(String::as_str), Some("bye"));
        app.advance(BYE + 0.01);
        assert!(app.quit);
        // Or any key cuts the goodbye short.
        let mut app = self::app();
        press(&mut app, KeyCode::Char('q'));
        press(&mut app, KeyCode::Char(' '));
        assert!(app.quit);
        let mut app = self::app();
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.quit);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn in_a_typing_game_a_letter_is_only_a_letter() {
        let mut app = app();
        app.level = Level::Medium;
        app.start(Game::Snack);
        let (theme, sound, animations) = (app.theme, app.sound, app.animations);
        type_text(&mut app, "qtsmhjkl?e123456");
        app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
        app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));
        press(&mut app, KeyCode::Enter);
        assert!(!app.quit && !app.help && app.screen == Screen::Play);
        assert_eq!((app.theme, app.sound, app.animations, app.level), (theme, sound, animations, Level::Medium));
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.screen, Screen::Home);
    }

    #[test]
    fn the_next_game_brings_new_words() {
        let mut app = app();
        app.level = Level::Medium;
        let mut met: Vec<String> = Vec::new();
        for _ in 0..10 {
            app.start(Game::Says);
            for word in &typing(&app).items {
                assert!(!met.contains(word), "{word} twice");
                met.push(word.clone());
            }
        }
        // And when every one has been met, they come round again.
        let all = words::short_words().len();
        for _ in 0..all / 5 + 2 {
            app.start(Game::Says);
            let items = &typing(&app).items;
            assert_eq!(items.iter().collect::<std::collections::HashSet<_>>().len(), 5);
        }
        app.level = Level::Easy;
        app.start(Game::Snack);
        assert_eq!(typing(&app).items.iter().collect::<std::collections::HashSet<_>>().len(), 5);
    }

    #[test]
    fn the_opening_ends_by_itself_or_at_any_key() {
        let mut app = app();
        app.begin(true);
        assert_eq!((app.screen, app.speaker.heard.as_slice(), app.mood), (Screen::Intro, &[Sound::Intro][..], Mood::Wave));
        app.advance(1.3);
        assert_eq!(app.speaker.said, ["hello", "hello-fun"]);
        assert!(!app.particles.is_empty() && app.talk > 0.5);
        assert_eq!(app.look().arms, Arms::Wave);
        app.advance(INTRO);
        assert_eq!((app.screen, app.bubble.as_str()), (Screen::Home, "What shall we do?"));
        assert!(app.particles.is_empty());

        // Skipped before she has said hello, she says it on the home screen.
        let mut app = self::app();
        app.begin(true);
        press(&mut app, KeyCode::Char('x'));
        assert_eq!((app.screen, app.speaker.said.as_slice()), (Screen::Home, &["hello".to_string()][..]));
        // With no opening asked for, or nothing moving, the same.
        let mut app = self::app();
        app.begin(false);
        assert_eq!((app.screen, app.speaker.said.as_slice()), (Screen::Home, &["hello".to_string()][..]));
        let mut app = self::app();
        app.animations = false;
        app.begin(true);
        assert_eq!(app.screen, Screen::Home);
    }

    #[test]
    fn without_animations_nothing_waits_and_nothing_moves() {
        let mut app = app();
        app.animations = false;
        app.level = Level::Medium;
        app.start(Game::Snack);
        for round in 0..5 {
            let word = typing(&app).items[round].clone();
            type_text(&mut app, &word);
        }
        assert_eq!(app.screen, Screen::Party);
        assert!(!app.advance(1.0) && app.particles.is_empty() && !app.animating());
        assert_eq!(app.hop(), (0.0, 0.0));

        // A ball of yarn is simply there, and the next one when it is caught.
        app.start(Game::Yarn);
        let goal = yarn(&app).goal;
        for caught in 0..goal {
            assert_eq!(yarn(&app).balls.iter().filter(|ball| ball.popped.is_none()).count(), 2.min(goal - caught));
            let ball = yarn(&app).balls.iter().find(|ball| ball.popped.is_none()).unwrap();
            assert!((0.2..0.9).contains(&ball.x));
            let letter = ball.letter;
            press(&mut app, KeyCode::Char(letter));
        }
        assert_eq!(app.screen, Screen::Party);
        // A story waits for "Next".
        app.start(Game::Story);
        app.act(Action::Pick(quiz(&app).items[0].right));
        app.advance(60.0);
        assert_eq!(quiz(&app).at, 0);
        app.act(Action::Next);
        assert_eq!(quiz(&app).at, 1);
    }

    #[test]
    fn she_talks_with_her_mouth_and_stops_when_the_sound_is_off() {
        let mut app = app();
        app.speak("hello");
        assert!(app.talk > 0.3 && app.bubble == "Hi! I'm Pink Kitty!");
        let mouths: std::collections::HashSet<bool> = (0..20)
            .map(|_| {
                app.advance(0.02);
                app.look().mouth == Mouth::Open
            })
            .collect();
        assert_eq!(mouths.len(), 2);
        app.advance(10.0);
        assert_eq!((app.talk, app.look().mouth), (0.0, Mouth::Smile));
        press(&mut app, KeyCode::Char('s'));
        let said = app.speaker.said.len();
        app.speak("hello");
        app.start(Game::Says);
        assert_eq!((app.talk, app.speaker.said.len(), app.speaker.heard.len()), (0.0, said, 0));
        // The bubble still says it, for reading.
        assert!(app.bubble.starts_with("Can you find the letter"));
    }
}
