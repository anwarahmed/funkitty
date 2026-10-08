//! Everything that is heard. A sound effect is made here out of a few notes; what
//! Pink Kitty says is put together from the clips of `voice`. Either is written as a
//! WAV file and played by handing that file to a program the system already has
//! (`pw-play`, `paplay` or `aplay` on Linux, `afplay` on macOS). Nothing is linked
//! against an audio library, and where no such program exists the game is silent.

use std::collections::VecDeque;
use std::f32::consts::TAU;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use crate::voice;

/// Samples a second, of one channel of 16 bits. The voice clips are the same.
const RATE: u32 = voice::RATE;

/// How many tunes a finished game can end with.
pub const FANFARES: u8 = 5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sound {
    /// The tune the game opens with.
    Intro,
    /// A button that is neither right nor wrong.
    Click,
    /// The right key.
    Tick,
    /// Another key. Soft: nobody is told off.
    Oops,
    /// Pink Kitty eating.
    Munch,
    /// A ball of yarn caught, a firework going off.
    Pop,
    /// A ball of yarn she missed, rolling round again.
    Boing,
    /// A word finished, a right answer.
    Chime,
    /// A gift being unwrapped.
    Gift,
    /// Something put on or taken off in the dressing room.
    Star,
    /// A game finished: one of `FANFARES` tunes.
    Fanfare(u8),
    Bye,
}

#[derive(Clone, Copy)]
enum Wave {
    /// A pure tone: a bell.
    Sine,
    /// A brighter one.
    Triangle,
    /// A knock: a tone that drops as it dies away.
    Knock,
    /// The other way: a tone that rises, for something springy.
    Rise,
    /// A hiss.
    Noise,
}

/// One note: what it sounds like, its pitch in Hz, when it starts and how long it
/// lasts in seconds, and how loud it is, from 0 to 1.
type Note = (Wave, f32, f32, f32, f32);

// The notes of the tunes, in Hz.
const C4: f32 = 261.6;
const E4: f32 = 329.6;
const G4: f32 = 392.0;
const C5: f32 = 523.3;
const D5: f32 = 587.3;
const E5: f32 = 659.3;
const F5: f32 = 698.5;
const G5: f32 = 784.0;
const A5: f32 = 880.0;
const B5: f32 = 987.8;
const C6: f32 = 1046.5;
const D6: f32 = 1174.7;
const E6: f32 = 1318.5;
const G6: f32 = 1568.0;
const C7: f32 = 2093.0;

impl Sound {
    #[cfg(test)]
    pub const ALL: [Sound; 16] = [
        Sound::Intro,
        Sound::Click,
        Sound::Tick,
        Sound::Oops,
        Sound::Munch,
        Sound::Pop,
        Sound::Boing,
        Sound::Chime,
        Sound::Gift,
        Sound::Star,
        Sound::Fanfare(0),
        Sound::Fanfare(1),
        Sound::Fanfare(2),
        Sound::Fanfare(3),
        Sound::Fanfare(4),
        Sound::Bye,
    ];

    fn name(self) -> String {
        match self {
            Sound::Intro => "intro".into(),
            Sound::Click => "click".into(),
            Sound::Tick => "tick".into(),
            Sound::Oops => "oops".into(),
            Sound::Munch => "munch".into(),
            Sound::Pop => "pop".into(),
            Sound::Boing => "boing".into(),
            Sound::Chime => "chime".into(),
            Sound::Gift => "gift".into(),
            Sound::Star => "star".into(),
            Sound::Fanfare(n) => format!("fanfare-{}", n % FANFARES),
            Sound::Bye => "bye".into(),
        }
    }

    fn notes(self) -> &'static [Note] {
        use Wave::{Knock, Noise, Rise, Sine, Triangle};
        match self {
            // A little skipping tune, up and down and up to a chord.
            Sound::Intro => &[
                (Triangle, C5, 0.0, 0.22, 0.3),
                (Triangle, E5, 0.18, 0.22, 0.3),
                (Triangle, G5, 0.36, 0.22, 0.3),
                (Triangle, E5, 0.54, 0.22, 0.3),
                (Triangle, G5, 0.72, 0.22, 0.3),
                (Triangle, C6, 0.9, 0.3, 0.3),
                (Triangle, C5, 1.3, 0.9, 0.2),
                (Triangle, E5, 1.3, 0.9, 0.2),
                (Triangle, G5, 1.3, 0.9, 0.2),
                (Sine, C6, 1.3, 0.9, 0.25),
                (Sine, E6, 1.45, 0.75, 0.12),
            ],
            Sound::Click => &[(Triangle, G5, 0.0, 0.07, 0.25)],
            Sound::Tick => &[(Sine, E6, 0.0, 0.08, 0.22), (Sine, G6, 0.02, 0.07, 0.1)],
            Sound::Oops => &[(Sine, E4, 0.0, 0.14, 0.2), (Sine, C4, 0.1, 0.2, 0.2)],
            // Three bites.
            Sound::Munch => &[
                (Knock, 190.0, 0.0, 0.09, 0.45),
                (Noise, 0.0, 0.0, 0.05, 0.22),
                (Knock, 170.0, 0.2, 0.09, 0.45),
                (Noise, 0.0, 0.2, 0.05, 0.22),
                (Knock, 200.0, 0.4, 0.09, 0.45),
                (Noise, 0.0, 0.4, 0.05, 0.22),
            ],
            Sound::Pop => &[(Knock, 720.0, 0.0, 0.07, 0.5), (Noise, 0.0, 0.0, 0.02, 0.2), (Sine, C6, 0.04, 0.16, 0.2)],
            Sound::Boing => &[(Rise, 180.0, 0.0, 0.28, 0.4)],
            Sound::Chime => &[(Sine, E5, 0.0, 0.16, 0.3), (Sine, G5, 0.09, 0.16, 0.3), (Sine, C6, 0.18, 0.4, 0.3)],
            Sound::Gift => &[
                (Sine, C6, 0.0, 0.16, 0.25),
                (Sine, E6, 0.07, 0.16, 0.25),
                (Sine, G6, 0.14, 0.16, 0.25),
                (Sine, C7, 0.21, 0.5, 0.25),
                (Sine, G6, 0.21, 0.5, 0.12),
            ],
            Sound::Star => &[(Sine, C6, 0.0, 0.25, 0.3), (Sine, G6, 0.03, 0.22, 0.15)],
            Sound::Bye => &[(Triangle, G5, 0.0, 0.2, 0.25), (Triangle, E5, 0.18, 0.2, 0.25), (Triangle, C5, 0.36, 0.5, 0.25)],
            Sound::Fanfare(n) => match n % FANFARES {
                // Up a chord and hold it.
                0 => &[
                    (Triangle, C5, 0.0, 0.14, 0.3),
                    (Triangle, E5, 0.12, 0.14, 0.3),
                    (Triangle, G5, 0.24, 0.14, 0.3),
                    (Triangle, C6, 0.36, 0.6, 0.3),
                    (Sine, E6, 0.36, 0.6, 0.15),
                    (Sine, G6, 0.36, 0.6, 0.12),
                ],
                // Ta-da!
                1 => &[
                    (Triangle, C5, 0.0, 0.16, 0.22),
                    (Triangle, E5, 0.0, 0.16, 0.22),
                    (Triangle, G5, 0.0, 0.16, 0.22),
                    (Triangle, C6, 0.22, 0.8, 0.22),
                    (Triangle, E6, 0.22, 0.8, 0.2),
                    (Sine, G6, 0.22, 0.8, 0.2),
                ],
                // All the way up the scale.
                2 => &[
                    (Sine, C5, 0.0, 0.12, 0.3),
                    (Sine, D5, 0.08, 0.12, 0.3),
                    (Sine, E5, 0.16, 0.12, 0.3),
                    (Sine, F5, 0.24, 0.12, 0.3),
                    (Sine, G5, 0.32, 0.12, 0.3),
                    (Sine, A5, 0.4, 0.12, 0.3),
                    (Sine, B5, 0.48, 0.12, 0.3),
                    (Sine, C6, 0.56, 0.6, 0.35),
                    (Sine, E6, 0.56, 0.6, 0.15),
                ],
                // Bouncing between two notes, then off the top.
                3 => &[
                    (Triangle, E5, 0.0, 0.12, 0.3),
                    (Triangle, G5, 0.12, 0.12, 0.3),
                    (Triangle, E5, 0.24, 0.12, 0.3),
                    (Triangle, G5, 0.36, 0.12, 0.3),
                    (Triangle, C6, 0.48, 0.2, 0.3),
                    (Triangle, D6, 0.66, 0.14, 0.3),
                    (Triangle, E6, 0.8, 0.6, 0.3),
                    (Sine, C6, 0.8, 0.6, 0.15),
                ],
                // A march: low, low, low, high.
                _ => &[
                    (Triangle, G4, 0.0, 0.12, 0.3),
                    (Triangle, C5, 0.14, 0.12, 0.3),
                    (Triangle, E5, 0.28, 0.12, 0.3),
                    (Triangle, G5, 0.42, 0.24, 0.3),
                    (Triangle, E5, 0.64, 0.12, 0.3),
                    (Triangle, G5, 0.78, 0.7, 0.3),
                    (Sine, C6, 0.78, 0.7, 0.2),
                    (Sine, E6, 0.78, 0.7, 0.12),
                ],
            },
        }
    }

    /// The sound itself.
    fn samples(self) -> Vec<i16> {
        let notes = self.notes();
        let length = notes.iter().map(|&(_, _, start, lasts, _)| start + lasts).fold(0.0, f32::max);
        let mut mix = vec![0.0f32; (length * RATE as f32) as usize];
        let mut seed = 0x2545_F491u32;
        for &(wave, pitch, start, lasts, loud) in notes {
            let first = (start * RATE as f32) as usize;
            for i in 0..(lasts * RATE as f32) as usize {
                let t = i as f32 / RATE as f32;
                let turn = (t * pitch).fract();
                let value = match wave {
                    Wave::Sine => (turn * TAU).sin(),
                    Wave::Triangle => 4.0 * (turn - 0.5).abs() - 1.0,
                    Wave::Knock => (TAU * pitch * (t - 2.0 * t * t)).sin(),
                    Wave::Rise => (TAU * pitch * (t + 4.0 * t * t)).sin(),
                    Wave::Noise => {
                        seed ^= seed << 13;
                        seed ^= seed >> 17;
                        seed ^= seed << 5;
                        seed as f32 / u32::MAX as f32 * 2.0 - 1.0
                    }
                };
                // In quickly, so that it does not click, and then dying away.
                let shape = (t / 0.004).min(1.0) * (-5.0 * t / lasts).exp() * (1.0 - t / lasts);
                if let Some(sample) = mix.get_mut(first + i) {
                    *sample += value * shape * loud;
                }
            }
        }
        mix.into_iter().map(|v| (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).collect()
    }
}

/// Samples as a WAV file.
fn wav(samples: &[i16]) -> Vec<u8> {
    let bytes = samples.len() as u32 * 2;
    let mut out = Vec::with_capacity(44 + bytes as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + bytes).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    // 16 bytes of format: plain samples, one channel, the rate, bytes a second,
    // bytes a sample and bits a sample.
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&bytes.to_le_bytes());
    out.extend(samples.iter().flat_map(|s| s.to_le_bytes()));
    out
}

/// The pause between two clips said one after the other, in seconds.
const BREATH: f32 = 0.1;
/// How many things can be waiting to be said. Pink Kitty does not fall behind: when
/// more than this pile up, the oldest is never said.
const WAITING: usize = 2;
/// The files what she says is written to, used in turn, so that the one being played
/// is never the one being written.
const FILES: usize = 4;

/// What plays the sounds, or does not.
pub struct Speaker {
    /// The program that plays a WAV file given as its last argument, and where the
    /// files are kept. Nothing when the game is silent.
    player: Option<(PathBuf, &'static [&'static str], PathBuf)>,
    written: Vec<Sound>,
    /// The players still playing, so that none is left behind as a zombie.
    playing: Vec<Child>,
    /// The player through which Pink Kitty is speaking now.
    talking: Option<Child>,
    /// What she will say when she has finished that.
    waiting: VecDeque<Vec<i16>>,
    said_files: usize,
    /// Everything asked for, in order.
    #[cfg(test)]
    pub heard: Vec<Sound>,
    /// Every clip she was asked to say, in order.
    #[cfg(test)]
    pub said: Vec<String>,
}

impl Speaker {
    pub fn silent() -> Speaker {
        Speaker {
            player: None,
            written: Vec::new(),
            playing: Vec::new(),
            talking: None,
            waiting: VecDeque::new(),
            said_files: 0,
            #[cfg(test)]
            heard: Vec::new(),
            #[cfg(test)]
            said: Vec::new(),
        }
    }

    /// Plays through `program`, keeping the sound files in `dir`.
    pub fn through(program: PathBuf, arguments: &'static [&'static str], dir: PathBuf) -> Speaker {
        Speaker { player: Some((program, arguments, dir)), ..Speaker::silent() }
    }

    /// Plays through whichever of the system's own players is installed, looked for
    /// along `PATH`. Silent when there is none.
    pub fn find(dir: PathBuf) -> Speaker {
        let players: &[(&str, &[&str])] = if cfg!(target_os = "macos") { &[("afplay", &[])] } else { &[("pw-play", &[]), ("paplay", &[]), ("aplay", &["-q"])] };
        let path = std::env::var_os("PATH").unwrap_or_default();
        for &(name, arguments) in players {
            if let Some(program) = std::env::split_paths(&path).map(|dir| dir.join(name)).find(|file| file.is_file()) {
                return Speaker::through(program, arguments, dir);
            }
        }
        Speaker::silent()
    }

    /// Whether there is anything to play a sound with.
    pub fn is_on(&self) -> bool {
        self.player.is_some()
    }

    fn start(&mut self, file: &std::path::Path) -> Option<Child> {
        let (program, arguments, _) = self.player.as_ref()?;
        // It must not write on the screen the game is drawn on.
        Command::new(program).args(*arguments).arg(file).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok()
    }

    /// Starts the sound and returns at once. A sound that cannot be played is not
    /// worth interrupting a game for.
    pub fn play(&mut self, sound: Sound) {
        #[cfg(test)]
        self.heard.push(sound);
        let Some((_, _, dir)) = &self.player else { return };
        self.playing.retain_mut(|child| !matches!(child.try_wait(), Ok(Some(_)) | Err(_)));
        let file = dir.join(format!("{}.wav", sound.name()));
        if !self.written.contains(&sound) {
            if std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&file, wav(&sound.samples()))).is_err() {
                return;
            }
            self.written.push(sound);
        }
        let child = self.start(&file);
        self.playing.extend(child);
    }

    /// Has Pink Kitty say these clips one after the other, once she has finished what
    /// she is saying now. Returns how many seconds they take her.
    pub fn say(&mut self, clips: &[String]) -> f32 {
        #[cfg(test)]
        self.said.extend(clips.iter().cloned());
        let mut samples: Vec<i16> = Vec::new();
        for clip in clips.iter().filter_map(|name| voice::clip(name)) {
            if !samples.is_empty() {
                samples.extend(std::iter::repeat_n(0, (BREATH * RATE as f32) as usize));
            }
            samples.extend(clip);
        }
        let seconds = samples.len() as f32 / RATE as f32;
        if self.player.is_some() && !samples.is_empty() {
            if self.waiting.len() >= WAITING {
                self.waiting.pop_front();
            }
            self.waiting.push_back(samples);
            self.poll();
        }
        seconds
    }

    /// Stops her in the middle of a word, and forgets what she was going to say.
    pub fn hush(&mut self) {
        self.waiting.clear();
        if let Some(mut child) = self.talking.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Whether she is speaking, or about to.
    pub fn talking(&mut self) -> bool {
        if self.talking.as_mut().is_some_and(|child| matches!(child.try_wait(), Ok(Some(_)) | Err(_))) {
            self.talking = None;
        }
        self.talking.is_some() || !self.waiting.is_empty()
    }

    /// Starts the next thing she has to say, if she has finished the last. The game
    /// calls this many times a second.
    pub fn poll(&mut self) {
        if self.talking() && self.talking.is_none() {
            let Some(samples) = self.waiting.pop_front() else { return };
            let Some((_, _, dir)) = &self.player else { return };
            let file = dir.join(format!("say-{}.wav", self.said_files % FILES));
            self.said_files += 1;
            if std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&file, wav(&samples))).is_ok() {
                self.talking = self.start(&file);
            }
        }
    }

    /// Where a sound's file is kept, once it has been played.
    #[cfg(test)]
    fn file(&self, sound: Sound) -> Option<PathBuf> {
        self.player.as_ref().map(|(_, _, dir)| dir.join(format!("{}.wav", sound.name())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_is_short_heard_and_never_too_loud() {
        for sound in Sound::ALL {
            let samples = sound.samples();
            let seconds = samples.len() as f32 / RATE as f32;
            assert!((0.05..3.0).contains(&seconds), "{sound:?} lasts {seconds}");
            let loudest = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
            assert!((3000..30000).contains(&loudest), "{sound:?} peaks at {loudest}");
            // It starts and ends in silence, so nothing clicks.
            assert!(samples[0].abs() < 600 && samples[samples.len() - 1].abs() < 600, "{sound:?}");

            let wav = wav(&samples);
            assert_eq!((&wav[..4], &wav[8..16], &wav[36..40]), (&b"RIFF"[..], &b"WAVEfmt "[..], &b"data"[..]));
            assert_eq!(u32::from_le_bytes(wav[4..8].try_into().unwrap()) as usize, wav.len() - 8);
            assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize, samples.len() * 2);
        }
        let names: std::collections::HashSet<_> = Sound::ALL.into_iter().map(Sound::name).collect();
        assert_eq!(names.len(), Sound::ALL.len());
        assert_eq!(Sound::Fanfare(FANFARES + 1).name(), Sound::Fanfare(1).name());
    }

    /// A stand-in for the system's player, which notes what it was asked to play and
    /// then takes as long as it is told to.
    fn stand_in(name: &str, takes: &str) -> (Speaker, PathBuf, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("funkitty-sound-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (player, log) = (dir.join("player"), dir.join("log"));
        std::fs::write(&player, format!("#!/bin/sh\necho \"$@\" >> '{}'\nsleep {takes}\n", log.display())).unwrap();
        std::fs::set_permissions(&player, std::fs::Permissions::from_mode(0o755)).unwrap();
        (Speaker::through(player, &["-q"], dir.join("sounds")), dir, log)
    }

    #[test]
    fn a_sound_is_a_file_handed_to_the_systems_player() {
        let (mut speaker, dir, log) = stand_in("play", "0");
        assert!(speaker.is_on());
        speaker.play(Sound::Click);
        speaker.play(Sound::Fanfare(2));
        speaker.play(Sound::Click);
        for child in &mut speaker.playing {
            child.wait().unwrap();
        }
        let (click, fanfare) = (speaker.file(Sound::Click).unwrap(), speaker.file(Sound::Fanfare(2)).unwrap());
        let mut asked: Vec<String> = std::fs::read_to_string(&log).unwrap().lines().map(String::from).collect();
        asked.sort();
        assert_eq!(asked, [format!("-q {}", click.display()), format!("-q {}", click.display()), format!("-q {}", fanfare.display())]);
        assert_eq!(std::fs::read(&click).unwrap(), wav(&Sound::Click.samples()));
        assert_eq!(speaker.heard, [Sound::Click, Sound::Fanfare(2), Sound::Click]);

        // With no player there is nothing to hear and nothing is written.
        let mut speaker = Speaker::silent();
        speaker.play(Sound::Star);
        assert!(speaker.say(&["praise-1".to_string()]) > 0.2);
        assert!(!speaker.is_on() && speaker.playing.is_empty() && speaker.written.is_empty() && !speaker.talking());
        // A player that is not there is no reason to stop.
        let mut speaker = Speaker::through(dir.join("missing"), &[], dir.join("sounds"));
        speaker.play(Sound::Star);
        speaker.say(&["praise-1".to_string()]);
        assert!(speaker.playing.is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn she_says_one_thing_at_a_time_and_does_not_fall_behind() {
        let (mut speaker, dir, log) = stand_in("say", "0.3");
        let one = speaker.say(&["praise-1".to_string()]);
        let two = speaker.say(&["type".to_string(), "word-cat".to_string()]);
        // Two clips and the breath between them.
        let apart = voice::clip("type").unwrap().len() + voice::clip("word-cat").unwrap().len();
        assert!((two - apart as f32 / RATE as f32 - BREATH).abs() < 0.01 && one > 0.2);
        speaker.say(&["praise-2".to_string()]);
        speaker.say(&["praise-3".to_string()]);
        // The first is being said, the second was dropped, and the last two wait.
        assert!(speaker.talking() && speaker.waiting.len() == WAITING);
        let started = std::time::Instant::now();
        while speaker.talking() && started.elapsed().as_secs() < 10 {
            speaker.poll();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let sounds = dir.join("sounds");
        let asked: Vec<String> = std::fs::read_to_string(&log).unwrap().lines().map(String::from).collect();
        assert_eq!(asked, [0, 1, 2].map(|n| format!("-q {}", sounds.join(format!("say-{n}.wav")).display())));
        assert_eq!(std::fs::read(sounds.join("say-2.wav")).unwrap(), wav(&voice::clip("praise-3").unwrap()));
        assert_eq!(speaker.said, ["praise-1", "type", "word-cat", "praise-2", "praise-3"]);

        // Hushed, she stops at once and says no more.
        speaker.say(&["praise-1".to_string()]);
        speaker.say(&["praise-2".to_string()]);
        speaker.hush();
        assert!(!speaker.talking());
        // A clip there is not is left out.
        assert_eq!(speaker.say(&["no-such-clip".to_string()]), 0.0);
        assert!(!speaker.talking());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
