//! Color themes, and the settings that are remembered between runs.

use std::path::PathBuf;

use crate::words::Level;

/// Red, green and blue.
pub type Rgb = (u8, u8, u8);

pub struct Theme {
    pub name: &'static str,
    /// The window, and the panels drawn on it.
    pub bg: Rgb,
    pub panel: Rgb,
    pub text: Rgb,
    pub dim: Rgb,
    /// The marker, the letter to type next, the chosen level.
    pub accent: Rgb,
    /// The six big buttons of the home screen, and the colors of every celebration.
    pub buttons: [Rgb; 6],
    /// What has been typed, and the right word.
    pub good: Rgb,
    /// A key of the keyboard that is drawn on the screen.
    pub key: Rgb,
}

pub const THEMES: [Theme; 5] = [
    Theme {
        name: "candy",
        bg: (255, 240, 246),
        panel: (255, 214, 231),
        text: (90, 24, 74),
        dim: (173, 104, 148),
        accent: (255, 64, 129),
        buttons: [(255, 105, 180), (149, 117, 255), (38, 198, 218), (255, 171, 64), (102, 187, 106), (240, 98, 146)],
        good: (38, 166, 91),
        key: (255, 255, 255),
    },
    Theme {
        name: "sunny",
        bg: (255, 248, 225),
        panel: (255, 236, 179),
        text: (62, 39, 35),
        dim: (141, 110, 99),
        accent: (255, 112, 67),
        buttons: [(239, 83, 80), (66, 165, 245), (255, 202, 40), (102, 187, 106), (171, 71, 188), (255, 138, 101)],
        good: (46, 160, 67),
        key: (255, 255, 255),
    },
    Theme {
        name: "sky",
        bg: (225, 242, 255),
        panel: (187, 222, 251),
        text: (20, 50, 90),
        dim: (90, 125, 165),
        accent: (30, 136, 229),
        buttons: [(255, 112, 140), (92, 107, 192), (38, 166, 154), (255, 167, 38), (126, 87, 194), (41, 182, 246)],
        good: (46, 160, 67),
        key: (255, 255, 255),
    },
    Theme {
        name: "mint",
        bg: (228, 247, 236),
        panel: (190, 232, 205),
        text: (25, 65, 45),
        dim: (95, 140, 115),
        accent: (0, 150, 110),
        buttons: [(255, 112, 150), (100, 120, 230), (0, 172, 193), (255, 160, 60), (140, 100, 210), (120, 180, 70)],
        good: (30, 150, 80),
        key: (255, 255, 255),
    },
    Theme {
        name: "night",
        bg: (22, 20, 46),
        panel: (42, 38, 84),
        text: (240, 240, 255),
        dim: (150, 150, 195),
        accent: (255, 150, 200),
        buttons: [(255, 105, 180), (149, 117, 255), (38, 198, 218), (255, 171, 64), (102, 210, 120), (255, 120, 120)],
        good: (105, 230, 150),
        key: (62, 58, 110),
    },
];

/// The names, for messages: "candy, sunny, ...".
pub fn theme_names() -> String {
    THEMES.iter().map(|t| t.name).collect::<Vec<_>>().join(", ")
}

pub fn find_theme(name: &str) -> Option<usize> {
    THEMES.iter().position(|t| t.name.eq_ignore_ascii_case(name))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    pub theme: usize,
    /// Whether to look for a newer release when the game starts.
    pub update: bool,
    /// Whether things move, or simply appear where they belong.
    pub animations: bool,
    /// Whether anything is heard: the tunes and Pink Kitty's voice alike.
    pub sound: bool,
    pub level: Level,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { theme: 0, update: true, animations: true, sound: true, level: Level::Easy }
    }
}

impl Settings {
    /// `settings` in the state directory.
    pub fn path() -> PathBuf {
        crate::update::state_dir().join("settings")
    }

    /// Lines of `name=value`. Anything missing or not understood keeps its default.
    pub fn parse(text: &str) -> Settings {
        let mut settings = Settings::default();
        for line in text.lines() {
            match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                Some(("theme", v)) => settings.theme = find_theme(v).unwrap_or(settings.theme),
                Some(("update", v)) => settings.update = v != "0",
                Some(("animations", v)) => settings.animations = v != "0",
                Some(("sound", v)) => settings.sound = v != "0",
                Some(("level", v)) => settings.level = Level::find(v).unwrap_or(settings.level),
                _ => {}
            }
        }
        settings
    }

    pub fn format(&self) -> String {
        format!(
            "theme={}\nupdate={}\nanimations={}\nsound={}\nlevel={}\n",
            THEMES[self.theme].name,
            u8::from(self.update),
            u8::from(self.animations),
            u8::from(self.sound),
            self.level.name().to_lowercase()
        )
    }

    pub fn load(path: &std::path::Path) -> Settings {
        Settings::parse(&std::fs::read_to_string(path).unwrap_or_default())
    }

    /// Failing to save is not worth interrupting a game for.
    pub fn save(&self, path: &std::path::Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, self.format());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_survive_a_round_trip_and_bad_input() {
        let settings = Settings { theme: find_theme("sky").unwrap(), update: false, animations: false, sound: false, level: Level::Hard };
        assert_eq!(Settings::parse(&settings.format()), settings);
        assert_eq!(Settings::parse("theme=nope\nsound\n=\nanimations=0\nlevel=huge\nextra=1"), Settings { animations: false, ..Settings::default() });

        let dir = std::env::temp_dir().join(format!("funkitty-test-{}", std::process::id()));
        let path = dir.join("deeper/settings");
        assert_eq!(Settings::load(&path), Settings::default());
        settings.save(&path);
        assert_eq!(Settings::load(&path), settings);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn names_are_unique_and_found() {
        for (i, theme) in THEMES.iter().enumerate() {
            assert_eq!(find_theme(theme.name), Some(i));
        }
    }
}
