#!/bin/sh
# End-to-end tests: the built program, run the way a user runs it.
#
#   tests/e2e.sh [path to the funkitty binary]     default: target/release/funkitty
#
#   1. the command line
#   2. install.sh and the self-updater, against releases made up here and served
#      from file:// (needs curl)
#   3. the game itself in detached tmux sessions (skipped when tmux is missing):
#      the opening, the home screen, each of the four games played to its party by
#      keyboard, story time and the dressing room by mouse, the gifts that a new start is without,
#      and the sounds and Pink Kitty's voice (played by stand-ins that only note what
#      they were given)
#
# The words, the games, the layout and the drawing are covered by `cargo test`; this
# is for what only shows when the real program meets a real terminal.
set -u

cd "$(dirname "$0")/.." || exit 1
ROOT=$PWD
BIN=${1:-target/release/funkitty}
case $BIN in /*) ;; *) BIN=$ROOT/$BIN ;; esac
[ -x "$BIN" ] || { echo "e2e.sh: $BIN is not built (cargo build --release)" >&2; exit 1; }

TMP=$(mktemp -d)
SOCK=funkitty-e2e-$$
trap 'tmux -L "$SOCK" kill-server 2>/dev/null; rm -rf "$TMP"' EXIT

fails=0
pass() { printf 'ok    %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1"; fails=$((fails + 1)); }
is() { # name expected actual
    if [ "$2" = "$3" ]; then pass "$1"; else fail "$1: expected '$2', got '$3'"; fi
}
has() { # name text-to-find text
    case $3 in *"$2"*) pass "$1" ;; *) fail "$1: no '$2' in '$3'" ;; esac
}

VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)

# ---------------------------------------------------------- command line ----

has "--version" "funkitty $VERSION (" "$("$BIN" --version)"
has "--help" "funkitty update" "$("$BIN" --help)"
has "an unknown option is refused" "unknown argument" "$("$BIN" --nonsense 2>&1)"
has "an unknown theme is refused" "--theme needs one of" "$("$BIN" --theme plaid 2>&1)"
has "needs a terminal" "needs a terminal" "$("$BIN" </dev/null 2>&1)"
has "a bad animations switch is refused" "--animations needs on or off" "$("$BIN" --animations sometimes 2>&1)"
XDG_STATE_HOME="$TMP/xdg-anim" "$BIN" --animations off </dev/null >/dev/null 2>&1
is "--animations off is remembered" "animations=0" "$(grep -x 'animations=0' "$TMP/xdg-anim/funkitty/settings" 2>/dev/null)"
has "a bad sound switch is refused" "--sound needs on or off" "$("$BIN" --sound loud 2>&1)"
XDG_STATE_HOME="$TMP/xdg-anim" "$BIN" --sound off </dev/null >/dev/null 2>&1
is "--sound off is remembered" "sound=0" "$(grep -x 'sound=0' "$TMP/xdg-anim/funkitty/settings" 2>/dev/null)"
has "a checkout never updates itself" "running from a source checkout" "$("$BIN" update 2>&1)"

has "a bad level is refused" "--level needs easy, medium or hard" "$("$BIN" --level huge 2>&1)"
XDG_STATE_HOME="$TMP/xdg-anim" "$BIN" --level hard </dev/null >/dev/null 2>&1
is "--level hard is remembered" "level=hard" "$(grep -x 'level=hard' "$TMP/xdg-anim/funkitty/settings" 2>/dev/null)"
has "reset: there is no such command now" "unknown argument 'reset'" "$("$BIN" reset 2>&1)"
lines=$("$BIN" voice-lines)
has "voice-lines: the letters" "letter-a	A." "$lines"
has "voice-lines: the phrases" "praise-1	Good job!" "$lines"

# ------------------------------------------------- install and self-update ----

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64 | Linux-amd64) ASSET=funkitty-x86_64-unknown-linux-musl ;;
    Linux-aarch64 | Linux-arm64) ASSET=funkitty-aarch64-unknown-linux-musl ;;
    Darwin-arm64) ASSET=funkitty-aarch64-apple-darwin ;;
    Darwin-x86_64) ASSET=funkitty-x86_64-apple-darwin ;;
    *) ASSET= ;;
esac

# make_release <dir> <version> <file to publish as this platform's binary>
make_release() {
    mkdir -p "$1"
    cp "$3" "$1/$ASSET"
    echo "$2" > "$1/VERSION"
    echo "$(sha256_of "$1/$ASSET")  $ASSET" > "$1/SHA256SUMS"
}

INST=$TMP/inst/bin/funkitty
installed() { XDG_STATE_HOME="$TMP/ustate" "$INST" "$@" 2>&1; }
# The copy a user would have: outside any checkout, in a directory they own.
fresh_copy() {
    rm -rf "$TMP/inst"
    mkdir -p "$TMP/inst/bin"
    cp "$BIN" "$INST"
}

if ! command -v curl >/dev/null 2>&1 || [ -z "$ASSET" ]; then
    echo "skip  install and self-update (no curl, or no release binary for this platform)"
else
    # A "newer release" whose binary is a script, so that it is plain which one runs.
    printf '#!/bin/sh\necho "funkitty 99.0.0 (fake)"\n' > "$TMP/fake"
    chmod 755 "$TMP/fake"
    make_release "$TMP/rel-now" "$VERSION" "$BIN"
    make_release "$TMP/rel-new" 99.0.0 "$TMP/fake"
    make_release "$TMP/rel-old" 0.0.1 "$TMP/fake"
    make_release "$TMP/rel-bad" 99.0.0 "$TMP/fake"
    echo "0000000000000000000000000000000000000000000000000000000000000000  $ASSET" > "$TMP/rel-bad/SHA256SUMS"

    out=$(FUNKITTY_RELEASE_URL="file://$TMP/rel-now" FUNKITTY_BIN_DIR="$TMP/inst/bin" sh install.sh 2>&1)
    has "install.sh: installs the release" "funkitty $VERSION (" "$(installed --version)"
    has "install.sh: says where" "Installed $INST" "$out"
    out=$(FUNKITTY_RELEASE_URL="file://$TMP/rel-bad" FUNKITTY_BIN_DIR="$TMP/inst2/bin" sh install.sh 2>&1)
    has "install.sh: refuses a bad checksum" "checksum mismatch" "$out"
    if [ -e "$TMP/inst2/bin/funkitty" ]; then fail "install.sh: installed despite a bad checksum"; else pass "install.sh: a refused download installs nothing"; fi
    FUNKITTY_BIN_DIR="$TMP/inst/bin" sh install.sh --uninstall >/dev/null 2>&1
    if [ -e "$INST" ]; then fail "install.sh: --uninstall left the binary"; else pass "install.sh: --uninstall removes it"; fi

    fresh_copy
    has "update: nothing newer" "funkitty $VERSION is up to date (latest release is $VERSION)." "$(FUNKITTY_RELEASE_URL="file://$TMP/rel-now" installed update)"
    has "update: never downgrades" "is up to date (latest release is 0.0.1)." "$(FUNKITTY_RELEASE_URL="file://$TMP/rel-old" installed update)"
    has "update: refuses a bad checksum" "checksum mismatch" "$(FUNKITTY_RELEASE_URL="file://$TMP/rel-bad" installed update)"
    has "update: a refused update changes nothing" "funkitty $VERSION (" "$(installed --version)"
    has "update: reports an unreachable server" "could not check for updates" "$(FUNKITTY_RELEASE_URL="file://$TMP/nowhere" installed update)"

    # What a package does when it installs: a marker beside the binary's directory.
    mkdir -p "$TMP/inst/share/funkitty"
    echo "Homebrew; use brew upgrade funkitty" > "$TMP/inst/share/funkitty/managed-by"
    is "update: a package's copy refuses" "funkitty: this copy can't update itself: installed with Homebrew; use brew upgrade funkitty" "$(FUNKITTY_RELEASE_URL="file://$TMP/rel-new" installed update)"
    has "update: a package's copy is untouched" "funkitty $VERSION (" "$(installed --version)"

    # Reached through a link, as Homebrew's bin directory does it: still refused.
    mkdir -p "$TMP/link"
    ln -s "$INST" "$TMP/link/funkitty"
    has "update: a package's copy refuses through a link too" "installed with Homebrew" "$(FUNKITTY_RELEASE_URL="file://$TMP/rel-new" XDG_STATE_HOME="$TMP/ustate" "$TMP/link/funkitty" update 2>&1)"

    fresh_copy
    has "update: installs a newer release" "Updated to 99.0.0." "$(FUNKITTY_RELEASE_URL="file://$TMP/rel-new" installed update)"
    is "update: the new version is what runs" "funkitty 99.0.0 (fake)" "$(installed --version)"

    # Through a link, the real file is replaced and the link is left alone.
    fresh_copy
    FUNKITTY_RELEASE_URL="file://$TMP/rel-new" XDG_STATE_HOME="$TMP/ustate" "$TMP/link/funkitty" update >/dev/null 2>&1
    if [ -L "$TMP/link/funkitty" ] && [ "$("$INST" --version)" = "funkitty 99.0.0 (fake)" ]; then
        pass "update: through a link, the real file is replaced and the link survives"
    else
        fail "update: through a link, the link was replaced or the file was not"
    fi

    fresh_copy
    has "update off" "The update check at startup is off." "$(installed update off)"
    is "update off is remembered" "update=0" "$(grep -x 'update=0' "$TMP/ustate/funkitty/settings")"
    has "update on" "The update check at startup is on." "$(installed update on)"
fi

# -------------------------------------------------------------- the game ----

if ! command -v tmux >/dev/null 2>&1; then
    echo "skip  the game in tmux (tmux is not installed)"
else
    # Everything runs on a private tmux server, so no other session is ever touched.
    T() { tmux -L "$SOCK" "$@"; }
    screen() { T capture-pane -p -t main 2>/dev/null; }
    expect() { # name text -- waits up to 10 seconds for the text to be on screen
        i=0
        while [ "$i" -lt 100 ]; do
            if screen | grep -qF -- "$2"; then pass "$1"; return; fi
            sleep 0.1
            i=$((i + 1))
        done
        fail "$1: '$2' never appeared"
        screen | sed 's/^/        | /'
    }
    absent() { # name text
        if screen | grep -qF -- "$2"; then fail "$1: '$2' is on screen"; else pass "$1"; fi
    }
    keys() { T send-keys -t main "$@"; }
    # click <text>: a left click (SGR mouse press and release) on the first character
    # of the text, wherever it is on screen. Not every awk counts characters (the one
    # on macOS counts bytes, and a check mark is three), so the bytes that continue a
    # character are dropped first, from the screen and from the text alike, and
    # everything is then counted as bytes.
    click() {
        want=$(printf '%s' "$1" | LC_ALL=C tr -d '\200-\277')
        at=$(screen | LC_ALL=C tr -d '\200-\277' | LC_ALL=C awk -v t="$want" '{ i = index($0, t); if (i) { print i ";" NR; exit } }')
        [ -n "$at" ] || { fail "click: '$1' is not on screen"; return; }
        T send-keys -t main -l "$(printf '\033[<0;%sM\033[<0;%sm' "$at" "$at")"
    }
    # start <state dir> <command and arguments>: a session running the game
    start() {
        state=$1
        shift
        T kill-session -t main 2>/dev/null
        T -f /dev/null new-session -d -s main -x 80 -y 24 \
            "env XDG_STATE_HOME='$state' FUNKITTY_NO_UPDATE=1 FUNKITTY_NO_SOUND=1 $*; echo \"EXIT=\$?\"; sleep 20"
    }
    # The letter Pink Kitty is asking for, once she asks for one that is not $1.
    asked() {
        i=0
        while [ "$i" -lt 100 ]; do
            letter=$(screen | sed -n 's/.*Can you find the letter \(.\)?.*/\1/p' | head -n 1)
            if [ -n "$letter" ] && [ "$letter" != "$1" ]; then printf '%s' "$letter"; return 0; fi
            sleep 0.1
            i=$((i + 1))
        done
        return 1
    }
    # Types every letter there is. One of them is the right one, the others cost nothing.
    alphabet() { keys a b c d e f g h i j k l m n o p q r s t u v w x y z; }
    # Does $1 over and over until the party comes, which shows "Play again". A party
    # takes no notice of the keyboard for its first second, so that what was still
    # being typed does not press its buttons; this waits that out.
    until_the_party() {
        tries=0
        while [ "$tries" -lt 80 ]; do
            if screen | grep -qF 'Play again'; then sleep 1.3; return 0; fi
            "$1"
            sleep 0.4
            tries=$((tries + 1))
        done
        return 1
    }
    # Story time: tries the three words in turn, and goes on when one was right.
    one_story_step() {
        for word in a b c; do
            if screen | grep -qF 'Next'; then break; fi
            keys "$word"
            sleep 0.3
        done
        if screen | grep -qF 'Next'; then keys Enter; fi
    }

    # The opening plays by itself and waits for nobody, but any key ends it.
    start "$TMP/xdg" "'$BIN'"
    expect "opening: it plays" "Press any key or click to start"
    keys x
    expect "home: any key ends the opening" "What shall we do?"
    expect "home: the games are there" "1 Snack time"
    expect "home: no gifts yet" "Gifts: 0 of 12"

    # The settings, by key and by mouse, and that they are remembered.
    keys t
    expect "home: T changes the theme" "T sunny"
    is "home: the theme is remembered" "theme=sunny" "$(grep -x 'theme=sunny' "$TMP/xdg/funkitty/settings" 2>/dev/null)"
    click "T sunny"
    expect "mouse: a click changes the theme" "T sky"
    keys m
    expect "home: M switches the animations off" "M Motion off"
    is "home: animations off is remembered" "animations=0" "$(grep -x 'animations=0' "$TMP/xdg/funkitty/settings" 2>/dev/null)"
    click "M Motion"
    sleep 0.3
    absent "mouse: a click switches them on again" "Motion off"
    keys e
    sleep 0.3
    is "home: E changes the level, and it is remembered" "level=medium" "$(grep -x 'level=medium' "$TMP/xdg/funkitty/settings" 2>/dev/null)"
    click "Easy (4-5)"
    sleep 0.3
    is "mouse: a click on a level chooses it" "level=easy" "$(grep -x 'level=easy' "$TMP/xdg/funkitty/settings" 2>/dev/null)"
    keys '?'
    expect "help: opens" "Everything can be clicked."
    keys x
    sleep 0.3
    absent "help: any key closes it" "Everything can be clicked."
    keys Escape
    sleep 0.5
    absent "home: Esc does not quit" "EXIT="

    # Kitty says, Easy: she asks for five letters, one after the other.
    keys 2
    expect "Kitty says: starts" "Tab Say again"
    last=
    typed=0
    while [ "$typed" -lt 5 ] && letter=$(asked "$last"); do
        keys "$(printf '%s' "$letter" | tr '[:upper:]' '[:lower:]')"
        last=$letter
        typed=$((typed + 1))
    done
    is "Kitty says: she asks for five letters" "5" "$typed"
    expect "Kitty says: five letters typed bring the party" "Play again"
    expect "Kitty says: and a gift" "A gift for Pink Kitty: "
    gift=$(screen | sed -n 's/.*A gift for Pink Kitty: \(.*\)!.*/\1/p' | head -n 1)

    # The party's buttons with the arrows, then the dressing room with the mouse.
    sleep 1.3
    keys Right Right Enter
    expect "dress up: the arrows and Enter go there from the party" "Gifts: 1 of 12"
    expect "dress up: the gift is there" "$gift"
    expect "dress up: and she has it on" "✓"
    click "$gift"
    sleep 0.4
    absent "mouse: a click takes the gift off" "✓"
    click "$gift"
    expect "mouse: and a click puts it on again" "✓"
    click "Esc Back"
    expect "mouse: Back goes home" "What shall we do?"
    expect "home: the gift is counted" "Gifts: 1 of 12"

    # Snack time, Easy: any wrong key costs nothing, so the whole alphabet feeds her.
    keys 1
    expect "snack time: starts" "I'm hungry!"
    if until_the_party alphabet; then pass "snack time: five treats bring the party"; else fail "snack time: the party never came"; fi
    keys Enter
    expect "party: Enter plays the same game again" "Snack time"
    absent "party: and the party is over" "Play again"
    keys Escape
    expect "snack time: Esc goes home" "Gifts: 2 of 12"

    # Yarn balls, with vim's keys for the arrows on the way there.
    keys h j l k h j Enter
    expect "yarn balls: H J K L move the marker as the arrows do" "Yarn balls"
    if until_the_party alphabet; then pass "yarn balls: six caught bring the party"; else fail "yarn balls: the party never came"; fi
    keys Right Enter
    expect "party: More games goes home" "Gifts: 3 of 12"

    # Story time on Medium, by keyboard: a wrong word only sends you to another.
    keys e 4
    expect "story time: starts" "Story time! Pick the word that fits."
    if until_the_party one_story_step; then pass "story time: three stories bring the party"; else fail "story time: the party never came"; fi
    keys Escape
    expect "story time: Esc leaves the party" "Gifts: 4 of 12"

    # The same by mouse: a word, and whatever came of it.
    click "4 Story time"
    expect "mouse: a clicked game starts" "Tab Say again"
    for word in A B C; do
        if screen | grep -qF "$word: "; then click "$word: "; sleep 0.4; fi
    done
    expect "mouse: clicked words find the right one" "Next"
    click "Next"
    sleep 0.5
    absent "mouse: Next brings the next story" "Next"
    click "Esc Back"
    expect "mouse: Back goes home from a game" "What shall we do?"

    T resize-window -t main -x 40 -y 8 2>/dev/null
    expect "window: a tiny one says so" "Please make the window bigger"
    T resize-window -t main -x 200 -y 55 2>/dev/null
    expect "window: a big one draws the title in big letters" "█"
    keys 1
    expect "window: a big one starts a game" "Esc Back"
    keys Escape
    expect "window: and goes home" "T Theme: sky"
    keys q
    expect "home: Q quits cleanly, after a wave" "EXIT=0"

    # Every start is an empty dressing room: the four gifts won above are gone, and
    # nothing about them was written down.
    start "$TMP/xdg" "'$BIN'" --no-intro
    expect "gifts: a new start has none" "Gifts: 0 of 12"
    if [ -e "$TMP/xdg/funkitty/gifts" ]; then fail "gifts: a gifts file was written"; else pass "gifts: nothing is kept"; fi
    keys 2
    expect "game: starts again" "Tab Say again"
    keys C-c
    expect "game: Ctrl-C quits at once" "EXIT=0"

    # Sound: the system's player is asked to play a file for the opening, for what
    # Pink Kitty says and for a letter typed. Stand-ins for every player it might look
    # for note what they were given.
    mkdir -p "$TMP/players"
    for player in pw-play paplay aplay afplay; do
        # shellcheck disable=SC2016  # the stand-in expands them, not this script
        printf '#!/bin/sh\nfor a in "$@"; do echo "$a"; done >> "%s"\n' "$TMP/played" > "$TMP/players/$player"
        chmod 755 "$TMP/players/$player"
    done
    start "$TMP/xdg5" env -u FUNKITTY_NO_SOUND "PATH='$TMP/players:$PATH'" "'$BIN'"
    expect "sound: the opening plays" "Press any key or click to start"
    sleep 1.6
    keys x
    expect "sound: the home screen comes" "What shall we do?"
    keys 2
    letter=$(asked "")
    keys "$(printf '%s' "$letter" | tr '[:upper:]' '[:lower:]')"
    sleep 0.8
    played=$(cat "$TMP/played" 2>/dev/null)
    has "sound: the opening tune is heard" "$TMP/xdg5/funkitty/sounds/intro.wav" "$played"
    has "sound: Pink Kitty says hello" "$TMP/xdg5/funkitty/sounds/say-0.wav" "$played"
    has "sound: and says what to type" "$TMP/xdg5/funkitty/sounds/say-1.wav" "$played"
    has "sound: a right letter is heard" "$TMP/xdg5/funkitty/sounds/chime.wav" "$played"
    is "sound: what is played is a WAV file" "RIFF" "$(head -c 4 "$TMP/xdg5/funkitty/sounds/intro.wav" 2>/dev/null)"
    is "sound: and so is what she says" "RIFF" "$(head -c 4 "$TMP/xdg5/funkitty/sounds/say-0.wav" 2>/dev/null)"
    # Her voice is more than a beep: "Hi! I'm Pink Kitty!" alone is half a second, at
    # 44,100 bytes a second.
    size=$(wc -c < "$TMP/xdg5/funkitty/sounds/say-0.wav" 2>/dev/null || echo 0)
    if [ "$size" -gt 15000 ]; then pass "sound: what she says is as long as speech"; else fail "sound: say-0.wav is only $size bytes"; fi
    keys Escape
    expect "sound: home again" "What shall we do?"
    keys s
    sleep 0.5
    is "sound: S switches it off, and that is remembered" "sound=0" "$(grep -x 'sound=0' "$TMP/xdg5/funkitty/settings" 2>/dev/null)"
    sleep 1
    rm -f "$TMP/played"
    keys 2
    expect "sound: a game starts with the sound off" "Can you find the letter"
    sleep 0.6
    if [ -e "$TMP/played" ]; then fail "sound: something was played after S"; else pass "sound: switched off, nothing is played"; fi
    keys Escape
    expect "sound: home once more" "What shall we do?"
    keys s
    sleep 0.6
    has "sound: switched on, it is heard at once" "click.wav" "$(cat "$TMP/played" 2>/dev/null)"
    keys C-c
    expect "sound: quits cleanly" "EXIT=0"

    # Starting the installed copy when a newer release exists: it updates and restarts
    # as the new version. Switched off, it starts the game without updating.
    if command -v curl >/dev/null 2>&1 && [ -n "$ASSET" ]; then
        STAMP=$TMP/ustate/funkitty/last-update-check
        launch() { # <release directory>
            T kill-session -t main 2>/dev/null
            T -f /dev/null new-session -d -s main -x 80 -y 24 \
                "env XDG_STATE_HOME='$TMP/ustate' FUNKITTY_NO_SOUND=1 FUNKITTY_RELEASE_URL='file://$1' '$INST' --no-intro; echo \"EXIT=\$?\"; sleep 20"
        }
        fresh_copy
        rm -f "$STAMP"
        installed update off >/dev/null
        launch "$TMP/rel-new"
        expect "update: switched off, the game starts" "What shall we do?"
        keys q
        expect "update: switched off, it quits cleanly" "EXIT=0"
        has "update: switched off, nothing is updated" "funkitty $VERSION (" "$(installed --version)"
        if [ -e "$STAMP" ]; then fail "update: switched off, yet a check was noted"; else pass "update: switched off, nothing is checked"; fi
        installed update on >/dev/null

        # At most one check a day: a start that finds nothing newer notes the time, and
        # the next start does not look again, even with a newer release on offer.
        launch "$TMP/rel-now"
        expect "once a day: the game starts after a check that finds nothing" "What shall we do?"
        keys q
        expect "once a day: it quits cleanly" "EXIT=0"
        if [ -s "$STAMP" ]; then pass "once a day: the check is noted"; else fail "once a day: no $STAMP"; fi
        launch "$TMP/rel-new"
        expect "once a day: the next start goes straight to the game" "What shall we do?"
        keys q
        expect "once a day: and quits cleanly" "EXIT=0"
        has "once a day: no second check, so no update" "funkitty $VERSION (" "$(installed --version)"
        has "once a day: asking explicitly still checks" "Updating funkitty $VERSION -> 99.0.0" "$(FUNKITTY_RELEASE_URL="file://$TMP/rel-bad" installed update)"

        # A day later (the noted time is old), a start checks again and updates.
        fresh_copy
        echo 1000 > "$STAMP"
        launch "$TMP/rel-new"
        expect "update: a day later, starting the game updates it and runs the new version" "funkitty 99.0.0 (fake)"
    fi
    T kill-server 2>/dev/null
fi

echo
if [ "$fails" -gt 0 ]; then
    echo "$fails failed"
    exit 1
fi
echo "all passed"
