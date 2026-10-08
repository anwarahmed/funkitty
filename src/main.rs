mod app;
mod font;
mod kitty;
mod picture;
mod sound;
mod theme;
mod ui;
mod update;
mod voice;
mod words;

use std::io::{self, IsTerminal, Write, stdout};
use std::process::ExitCode;
use std::time::Duration;

use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use ratatui::crossterm::execute;

use app::{App, Gifts};
use theme::Settings;
use words::Level;

const USAGE: &str = "\
funkitty - fun time with Pink Kitty: little reading and typing games for young children

Usage:
  funkitty                    play
  funkitty reset              give all of Pink Kitty's gifts back, to win them again
  funkitty update             check for a newer release now and install it
  funkitty update off | on    stop, or resume, checking when the game starts
  funkitty --help | --version

Options:
  -l, --level NAME            easy (ages 4-5), medium (6-7) or hard (8-10)
  -t, --theme NAME            candy, sunny, sky, mint or night
  -a, --animations on|off     whether things move (on by default)
  -s, --sound on|off          whether anything is heard (on by default)
      --no-intro              start at the home screen, without the opening

The level, the theme, the animations and the sound are remembered for next time, and
so are the gifts Pink Kitty has won, in $XDG_STATE_HOME/funkitty
(~/.local/state/funkitty).

In the game everything can be clicked. With the keyboard:
  arrows or H J K L, Enter    move the marker and choose
  1 2 3 4                     the games: snack time, Kitty says, yarn balls, story time
  5 dress up   6 surprise     E easy, medium or hard
  T theme   S sound   M motion (animations)   ? help   Q quit
  Tab                         in a game: Pink Kitty says it again
  Esc                         in a game: back to the home screen

Environment:
  FUNKITTY_NO_UPDATE          set to skip the update check for one run
  FUNKITTY_NO_SOUND           set to play no sound for one run
  FUNKITTY_LOG                a file to record every key and mouse event in
";

fn fail(msg: &str) -> ExitCode {
    eprintln!("funkitty: {msg}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let truecolor = matches!(std::env::var("COLORTERM").as_deref(), Ok("truecolor" | "24bit"));
    let settings_path = Settings::path();
    let mut settings = Settings::load(&settings_path);
    let mut intro = true;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-V" | "-v" | "--version" => {
                println!("funkitty {} ({})", update::VERSION, if update::COMMIT.is_empty() { "unknown commit" } else { update::COMMIT });
                return ExitCode::SUCCESS;
            }
            "update" => {
                return match args.next().as_deref() {
                    None => update::command().map_or_else(|e| fail(&e), |()| ExitCode::SUCCESS),
                    Some(switch @ ("on" | "off")) => {
                        settings.update = switch == "on";
                        settings.save(&settings_path);
                        println!("The update check at startup is {switch}.");
                        ExitCode::SUCCESS
                    }
                    Some(_) => fail("'update' takes on, off or nothing"),
                };
            }
            "reset" => {
                Gifts::default().save(&Gifts::path());
                println!("Pink Kitty has given all her gifts back. Every game wins one again.");
                return ExitCode::SUCCESS;
            }
            // For tools/voice.py: every line Pink Kitty can say, as a clip's name, a
            // tab and the text.
            "voice-lines" => {
                let mut out = stdout().lock();
                for (name, text) in words::spoken() {
                    let _ = writeln!(out, "{name}\t{text}");
                }
                return ExitCode::SUCCESS;
            }
            "-l" | "--level" => match args.next().as_deref().and_then(Level::find) {
                Some(level) => settings.level = level,
                None => return fail("--level needs easy, medium or hard"),
            },
            "-t" | "--theme" => match args.next().as_deref().and_then(theme::find_theme) {
                Some(theme) => settings.theme = theme,
                None => return fail(&format!("--theme needs one of: {}", theme::theme_names())),
            },
            "-a" | "--animations" => match args.next().as_deref() {
                Some(switch @ ("on" | "off")) => settings.animations = switch == "on",
                _ => return fail("--animations needs on or off"),
            },
            "-s" | "--sound" => match args.next().as_deref() {
                Some(switch @ ("on" | "off")) => settings.sound = switch == "on",
                _ => return fail("--sound needs on or off"),
            },
            "--no-intro" => intro = false,
            _ => return fail(&format!("unknown argument '{arg}'")),
        }
    }
    // What the options chose is remembered even when the game then cannot start.
    settings.save(&settings_path);
    if !io::stdin().is_terminal() || !stdout().is_terminal() {
        return fail("this is an interactive game and needs a terminal");
    }
    update::before_start(settings.update);

    let gifts_path = Gifts::path();
    let mut app = App::new(truecolor, settings, Gifts::load(&gifts_path));
    app.settings_path = Some(settings_path);
    app.gifts_path = Some(gifts_path);
    // Found even while sound is off, so that S has something to switch on.
    if std::env::var_os("FUNKITTY_NO_SOUND").is_none() {
        app.speaker = sound::Speaker::find(update::state_dir().join("sounds"));
    }
    app.begin(intro);

    // Installed before ratatui's hook, which restores the terminal and then calls this one.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // ratatui's hook does not know the mouse was captured.
        let _ = execute!(stdout(), DisableMouseCapture);
        default_hook(info)
    }));

    match run(&mut app) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(&e.to_string()),
    }
}

/// How long the loop waits for a key before drawing her tail a little further on.
const POLL: Duration = Duration::from_millis(80);
/// The same while something is flying, rolling or jumping, so that it moves smoothly.
const FRAME: Duration = Duration::from_millis(16);

fn run(app: &mut App) -> io::Result<()> {
    // Raw mode, the alternate screen, and a panic hook that undoes both.
    let mut terminal = ratatui::init();
    // For clicking. While captured, selecting text needs shift (option on macOS).
    execute!(stdout(), EnableMouseCapture)?;
    // FUNKITTY_LOG=file records every key and mouse event as the program receives it,
    // for working out why a terminal's input is not doing what it should.
    let mut log = std::env::var_os("FUNKITTY_LOG").and_then(|path| std::fs::File::create(path).ok());
    let mut redraw = true;
    let result = loop {
        redraw |= app.tick();
        if redraw && let Err(e) = terminal.draw(|f| ui::draw(f, app)) {
            break Err(e);
        }
        redraw = false;
        if app.quit {
            break Ok(());
        }
        match event::poll(if app.animating() { FRAME } else { POLL }) {
            Ok(false) => {}
            Ok(true) => match event::read().inspect(|event| {
                if let Some(log) = &mut log {
                    let _ = writeln!(log, "{event:?}");
                }
            }) {
                Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                    app.on_key(key);
                    redraw = true;
                }
                Ok(Event::Mouse(mouse)) => {
                    app.on_mouse(mouse);
                    redraw = true;
                }
                Ok(_) => redraw = true,
                Err(e) => break Err(e),
            },
            Err(e) => break Err(e),
        }
    };
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}
