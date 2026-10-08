# funkitty

"Fun time with Pink Kitty": four little reading and typing games for young children in
the terminal (TUI), a dressing room for the gifts they win, and a celebration after
everything. Pink Kitty speaks. Keyboard and mouse both do everything. Rust + ratatui,
targets macOS and Linux. `README.md` is for players and parents; this file is for
whoever changes the code.

## Commands

```sh
cargo run --release                          # play (from a checkout it never updates itself)
cargo run --release -- --no-intro -l hard    # straight to the home screen, at a level
cargo test                                   # unit tests: words, voice, sound, her picture, the games, drawing at eight window sizes, settings, update
cargo clippy --all-targets -- -D warnings    # no warnings allowed
cargo fmt                                    # rustfmt.toml: max_width 160
cargo build --release && tests/e2e.sh        # the built program in tmux, install.sh, the updater
cargo run -q -- voice-lines | tools/voice.py # make the voice clips that are missing or out of date
tools/voice-check.py --retake 8              # list the clips a speech recognizer does not hear as written, and say those again
FUNKITTY_LOG=/tmp/events.log cargo run       # record every key and mouse event received
```

`build.rs` stamps the commit for `--version` and writes the list of `voice/*.wav` that
`voice.rs` includes.

`install.sh` (POSIX sh, macOS + Linux) downloads the latest release binary into
`~/.local/bin` (`FUNKITTY_BIN_DIR` overrides) and verifies its checksum. `--source`
builds instead (the checkout it is in, else a fresh clone), and it falls back to that
when no binary exists for the platform. `--link` symlinks to the checkout's build,
`--uninstall` removes it. It never installs Rust and never edits shell profiles.
`FUNKITTY_RELEASE_URL` points it, and the self-updater, at another download base (a
`file://` directory holding `VERSION`, `SHA256SUMS` and a binary; `tests/e2e.sh` makes
such directories).

## Workflow

- **`main` only accepts pull requests** (GitHub ruleset "Main"): no direct pushes, no
  force-pushes, no deletion, no bypass for anyone. A PR needs these checks to pass,
  matched by job name: `test (ubuntu-latest)`, `test (macos-latest)`, `msrv`,
  `release checklist`. Renaming a CI job means updating the ruleset or PRs wait forever.
  PRs are squash-merged.
- **Local layout.** The user keeps this repo as a bare clone with one worktree per
  branch: `~/Developer/GitHub/anwarahmed/funkitty/main` plus a sibling directory per
  feature branch (`git worktree add -b <branch> <branch> origin/main` from the bare
  repo). Remove the worktree and branch after the PR merges, then fast-forward `main`.
- **Releasing** is merging a version bump; a merge without one publishes nothing.
  **Follow [RELEASING.md](RELEASING.md) every time, every step.** The release PR's
  description must carry its checklist with every line ticked (`gh pr create --body`
  does not add it for you), or the `release checklist` check fails. Tick a line only
  after doing what it says. Then do its "After merging" steps and report each one.
- **Sibling repo:** https://github.com/anwarahmed/homebrew-tap holds the generated
  Homebrew formula (`Formula/funkitty.rb`, written by its
  `scripts/formulae/funkitty.sh`). It takes direct pushes, because its bot commits
  formulae to `main`.
- **Sister projects:** fungeo, funchess, wordl and typeshelf (same owner) share this
  release workflow, `install.sh` and `src/update.rs` design. A fix to any of those in
  one project belongs in the others in the same sitting.

## Architecture

Single binary crate, no async, no threads. One file per concern in `src/`:

| File         | Role |
|--------------|------|
| `main.rs`    | CLI, terminal setup/teardown, the event loop (80 ms poll, 16 ms while something moves) |
| `words.rs`   | The levels, and everything read from `data/`: words, sentences, stories, what she says. `spoken()` lists every clip the game can ask for. The small `Rng`. Knows nothing about the screen |
| `app.rs`     | `App` state; what every key, click and tick does; the four games (`Typing`, `Yarn`, `Quiz`); the parties and little celebrations; `Gifts`; particles |
| `ui.rs`      | All drawing: layouts, her place on each screen, treats, balls of yarn, the keyboard, and the clickable rectangles (`App::buttons`) |
| `kitty.rs`   | Pink Kitty: drawn out of ellipses and triangles each frame, with her face, paws and gifts |
| `picture.rs` | `Picture`: square pixels, two to a cell, with empty pixels that keep what is under them |
| `font.rs`    | The 5x7 bitmap letters, drawn with half-block characters |
| `voice.rs`   | The clips of `voice/`, built in, and the IMA ADPCM decoder for them |
| `sound.rs`   | Sound effects made out of notes; `Speaker`, which plays those and queues what she says, through the system's player |
| `theme.rs`   | The five themes and `Settings` |
| `update.rs`  | Self-update, the sister projects' design |

How a frame happens: `App::tick` advances time (`App::advance(dt)`, which tests call
directly) and lets the speaker start her next sentence, then `ui::draw` redraws
everything and rebuilds `App::buttons`. A click looks up the topmost button under it
and calls `App::act(Action)`; a key is turned into the same `Action`. So mouse and
keyboard cannot drift apart: anything new must be an `Action` with both a button and a
key. The marker (`App::marker`) is an `Action` too: the arrows move it to the nearest
button in that direction (`App::step`, by the rectangles of the last frame), and the
mouse moves it to whatever it is over.

## Decisions, and why

- **Who it is for.** The user's children, 4 to 10. The youngest have just started to
  read, type and use a computer; the oldest read and type well but are slow at the
  computer. They come for a minute, do one thing and leave, so there is no long goal:
  a game is 3 to 10 small things, then a party. Themes, animations and sounds are to
  be playful, pleasant and positive (the user's words).
- **Pink Kitty** is the user's character: a white kitty with a large, round, cute
  face and a light pink scarf "around her neck and head". The user chose her from a
  sheet of four scarves and three faces: the snug hood with her ears tucked inside it
  ("1"), and the smiling eyes ("C"). Do not redesign her without asking. The medium
  one is drawn from shapes (`kitty::picture`); the large is that with every pixel
  doubled; the small one (for an 80x24 window) is drawn by hand (`kitty::SMALL`),
  because shapes at half size came out rough. Her face and gifts are drawn on top of
  all three the same way, so a new gift or expression needs both sizes checked.
- **Nobody loses, and nothing is told off.** A wrong key is a soft sound and wide eyes
  for half a second. No lives, no timer, no score. A ball of yarn that gets past
  rolls round again. Every fourth wrong key she says something encouraging. Keep it
  so: a new game must have no way to fail.
- **Many celebrations** (the user asked for "many types of celebrations, so that it
  doesn't get boring"): six little ones (`Cheer`) and twelve parties (`Fun`), five
  tunes, a dozen things she says, ten headlines. Each is picked at random and never
  the same as the last. A new one is a variant of the enum, its entry in `ALL`, and
  an arm in `App::throw` or `App::cheer`; tests go through `ALL`.
- **The levels** are the user's age range cut in three: Easy 4-5 (single letters, the
  wanted key lit on the screen's keyboard), Medium 6-7 (short words), Hard 8-10
  (longer words, sentences, spelling by ear). The level is one setting for all games.
- **In a typing game a letter is only a letter.** So the commands that are letters on
  the home screen (`T`, `S`, `M`, `Q`, `E`, and `H` `J` `K` `L` as arrows) are not
  commands there; only `Esc` and `Tab` are. Easy story time is the same, because its
  answers are letters. Ctrl with a letter does nothing anywhere, except `Ctrl-C`.
- **Esc does not quit on the home screen; only Q does** (the user asked for this in
  fungeo), and Q is a goodbye wave before the program ends.
- **The keyboard on the screen** makes the typing games playable by mouse (the user
  wants everything usable by both), and shows a beginner where the keys are.
- **She speaks with recorded clips**, not a speech synthesizer at run time (the user
  chose "generated": made once with Piper). `tools/voice.py` makes one clip for each
  line of `words::spoken()` and records the text in `voice/lines.tsv`; a test fails
  when a line has no clip or the clip was made from other text. What she says is
  several clips joined ("Can you type" + "cat"), so there are about 370 clips and not
  thousands. The clips are IMA ADPCM (a quarter the size of plain samples, about 3.5 MB
  in all) and `voice.rs` decodes them itself; the decoder was checked sample for
  sample against ffmpeg's.
- **The voice's licence matters.** The clips are distributed with the program, so the
  Piper voice must be one whose training data allows that. The current voice and its
  licence are named at the top of `tools/voice.py`. `en_US-hfc_female` was clearer in
  tests but its data is CC BY-NC-SA, so it was not used.
- **What she says is always in her bubble too.** So the game is the same with the
  sound off or with no player installed, and a child sees the words she hears. A word
  that would only be heard (Hard "Kitty says") is shown when she cannot be heard
  (`App::concealed`), and after two wrong keys.
- **Sound** is the sister projects' design: no audio library, WAV files written to
  the state directory and played by `pw-play`/`paplay`/`aplay`/`afplay`. What she says
  queues (`Speaker::say`, `poll`), one thing at a time, at most two waiting; a new
  game hushes her.
- **Gifts are kept between runs** (`gifts` in the state directory), unlike fungeo's
  stars, which the user asked to reset. This was my choice, not the user's: a
  dressing room that empties every start seemed worse. `funkitty reset` gives them
  back. If the user wants them to reset, `Gifts` simply stops being loaded.
- **Words are plain files** in `data/`, as fungeo's questions are. They were written
  by Claude for children of 4 to 10 and not reviewed by a person. Hard words must not
  sound like another word spelled differently, since they are typed by ear.
- **Text size.** The user asked (in fungeo) that text not be small. What is typed and
  read is drawn in `font.rs`'s big letters wherever it fits (`ui::letters`), each
  letter in its own color: typed, next, still to come.
- **A click counts on press; a release with no press before it counts too**, in case
  a terminal only reports the release. This came from funchess.
- **Animations off** (`M`) means nothing waits: no opening, no flight of the treat, no
  goodbye, balls of yarn simply sit there, a story waits for "Next", and the screen is
  only redrawn on input.
- **Particles are drawn only on empty cells**, behind or in front, so nothing covers a
  word. There are never more than 600.
- **Self-update** is the sister projects' design exactly: a `VERSION` file from the
  latest release (no GitHub API, so no rate limit), a `share/funkitty/managed-by`
  marker by which Homebrew and pacman switch it off, and a check at most once a day
  (`last-update-check` in the state directory). The release asset names
  (`funkitty-<target>`, `SHA256SUMS`, `VERSION`, `PKGBUILD`) are a contract with
  `install.sh`, the updater, the tap's formula generator and the AUR package.
- **Colors** are 24-bit; without `COLORTERM=truecolor` the finished frame is mapped
  to the 256-color cube (`ui::indexed`).

## Verifying a change

- `cargo test` draws every screen (each game at each level, at its start and part of
  the way through, every kind of party) at eight sizes from 80x24 to 300x90, and
  plays every game to its party; a layout that puts a button off the window or on top
  of another fails it, and so does a button the arrows cannot reach.
- `cargo build --release && tests/e2e.sh`: the real program in tmux on a private
  socket, by keyboard and by mouse, every game to its party, the gifts file, the
  sounds and her voice (through stand-in players), and `install.sh` and the updater
  against made-up releases served from `file://`. CI runs it on Linux and macOS.
- To see it: `docs/screenshot.sh` runs it in tmux on a **private socket** (never the
  user's server), captures the pane with its colors (`capture-pane -p -e -N`) and
  draws that as a picture. Pass it a size, options and keys to look at any screen;
  `SHOT_GIFTS` gives her gifts to start with. Look at 80x24 and at about 140x42: she
  is a different drawing at each.
- A scripted player cannot read big letters. `tests/e2e.sh` gets through snack time
  and yarn balls by typing the whole alphabet (wrong keys cost nothing), through
  "Kitty says" by reading the letter out of her speech bubble, and through story time
  by trying A, B and C.
- The voice: `tools/voice-check.py` has a speech recognizer transcribe every clip.
  Piper does not say a thing the same way twice, and about one single word in six
  comes out with its first sound swallowed ("pie" heard as "hi"), so after making
  clips run it with `--retake 8`, which has those said again until a take is heard
  as written. What is left is mostly words that sound like another ("sea", "deer");
  treat its list as clips to listen to, not as failures.

## Known gaps

- **Nobody has heard it.** The sound effects were checked as numbers only, and the
  voice only through a speech recognizer, which hears 356 of the 372 clips as written
  (and what the running game says, joined up, as the sentences they are). Whether she
  sounds pleasant, and like a kitty a child would like, no test can say. Whoever
  listens first should expect to find clips to remake: delete them and run
  `tools/voice.py`, or change the numbers at its top.
- The animations were only seen as captured frames, not live.
- Never run by a person on a Mac; CI runs the tests there.
- The AUR package `funkitty-bin` is rendered for each release but not pushed: the
  user has no AUR account.
- The big font has no lower case, so everything read in big letters is in capitals.
  A child learning lower-case letters sees them only in the speech bubble.
- The words, sentences and stories are in English only, and unreviewed.
- Ideas offered and not built: counting (how many fish?), words for pictures, a
  mouse game (popping bubbles), rhymes, capital and small letters, simple sums,
  a daily surprise, a profile for each child.
