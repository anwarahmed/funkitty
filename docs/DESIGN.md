# How funkitty was designed

`CLAUDE.md` says what the rules of this project are and why. This is the longer
record behind it: what was asked for, what was offered, what was chosen, what was
tried and thrown away, and the numbers that decided things. It was written when the
game was first built (October 2026) so that none of it lives only in a conversation.
When a decision here changes, change it here too.

## What was asked for

The owner wanted a new game beside funchess, fungeo and funwordl:

- A terminal game for macOS and Linux.
- For young children, so the theme, animations and sounds are playful, pleasant and
  positive.
- The star is **Pink Kitty**: a white kitty with a light pink scarf around her neck
  and head, and a large, round, cute face.
- The child does something with her that is educational, or builds reading and
  typing. What exactly was left open, and ideas were asked for.
- Rewards and celebrations often, whenever a task is done. No long goal: a child
  comes, does one quick thing, leaves, and comes back later.
- Could she speak, for example say "Good job!"?
- Her look was to be settled first, from options.

Answers given later: look **1C**; build all the activities offered; ages **4 to
10** (the youngest have just started to read, type and use a computer; the oldest
read and type well but are a little slow at the computer); a **generated** voice;
the name **funkitty**; the title "Fun time with Pink Kitty". And, while it was being
built: **many kinds of celebration, so that it does not get boring.**

## Her look

`tools/kitty-sheet.py` is the sheet the look was chosen from. It draws four ways of
wearing the scarf and three faces, in the half-block pixels the game uses, so what
was chosen is what is shown.

| | Scarf | | Face |
|---|---|---|---|
| **1** | **a snug hood, ears tucked inside (chosen)** | A | big sparkly eyes |
| 2 | a headscarf tied in a bow, ears poking out | B | small simple eyes |
| 3 | a band over her head and a scarf round her neck | **C** | **smiling eyes (chosen)** |
| 4 | a loose shawl hanging down both sides | | |

Her other faces are expressions, not looks: wide eyes (A's) for a moment of
surprise, hearts when she is given something to wear, an open mouth when she talks
or eats.

How she is drawn (`src/kitty.rs`):

- In units of a drawing 44 wide and 48 tall. Head: an ellipse 39 by 32 for the hood,
  a face opening 29 by 23, ears as triangles behind the hood. A small body, a tail
  that sways, a scarf round the neck with one end hanging.
- **Medium** (60 by 28 cells) is drawn from those shapes each frame, one pixel to a
  unit, with a one-pixel outline round each shape and a darker and a lighter rim on
  the scarf.
- **Large** (120 by 56) is Medium with every pixel doubled, so she is the same
  drawing. Used when the window is about 230 by 70 or more.
- **Small** (30 by 15) is for an 80 by 24 window. Drawing the shapes at half scale
  was tried first and looked rough (ears one pixel tall, a flat top to her head,
  outlines too heavy), so the small one is a sprite drawn by hand (`kitty::SMALL`).
  Her face and her gifts are still drawn on top by the same code as the others.
- The picture has room round her (8 units each side, 6 above) for what she wears and
  holds. A test wears everything at once and checks nothing touches the edge.

## The games

Each is a few small things and then a party. None can be failed.

| Game | Easy (4-5) | Medium (6-7) | Hard (8-10) |
|---|---|---|---|
| **Snack time**: type what is under the treat, and it flies to her mouth | 5 letters | 5 short words | 3 sentences |
| **Kitty says**: she says it, the child types it | 5 letters, shown | 5 short words, shown | 5 longer words, heard and not shown |
| **Yarn balls**: press the letter on a rolling ball | 6 balls, one at a time, 11 s to cross | 8 balls, two at a time, 8 s | 10 balls, three at a time, 6 s |
| **Story time**: pick the missing word of three | what letter a word starts with | a short sentence | a longer sentence |

Details that were decided on the way:

- **Easy lights the wanted key** on the keyboard drawn on the screen. Medium and
  Hard light it only after a wrong key.
- **A hidden word is shown after two wrong keys**, and is never hidden when she
  cannot be heard (sound off, or no player installed).
- **A ball that gets past rolls round again.** After a catch the next ball comes
  within half a second, so nobody waits on an empty floor.
- **Easy story time is answered with the letter's own key.** It first used A, B and
  C like the other levels, and showed "A: E", which is nonsense to a child looking
  for E.
- **She reads the whole sentence out when it is right**, and the game waits for her
  to finish before the next one.
- **Words are not repeated** until every one in the pool has been met since the
  program started.
- **The treat sits beside her speech bubble in a small window**, not above the word,
  so that a sentence has room for three lines of big letters.

Ideas offered and not built: counting (how many fish?), a word for a picture, a mouse
game (popping bubbles, for the ones who are slow with the mouse), rhymes, capital and
small letters, simple sums, a daily surprise, a profile for each child.

## Celebrations

After each word or answer, one of six: sparkles, hearts, a fountain of stars, music
notes, confetti cannons from both corners, a ring of bubbles. After a finished game,
one of twelve parties: confetti, fireworks, balloons, hearts, shooting stars, bubbles,
a rainbow, a dance, a garden of flowers, a shoal of fish, a spiral of stars, glitter.
With them: one of five tunes, one of eight things she says, one of ten headlines.
Each is drawn at random and is never the one before.

Every finished game also wins one of twelve gifts, chosen at random from those she
does not have, and she puts it on at once. When she has all twelve, a heart.

Two things found by testing and fixed:

- **A sparkle could land in the space between two words** of a button and read as
  part of it. Particles are now drawn only on an empty cell with empty cells on both
  sides.
- **Keys typed as a party began reached the party.** An `L` moved its marker, an `M`
  switched animations off, `Esc` ended it unseen. A party now takes no notice of the
  keyboard for its first second.

## The screen

- The smallest window is 80 by 24, the size a terminal opens at.
- Home: the title, Pink Kitty on the left with her speech bubble beside her, six big
  buttons (the four games, dress up, surprise), the three levels, the settings along
  the bottom.
- A game: a row at the top (back, say again, the name, a dot for each thing to do),
  her on the left, the task beside her, and along the bottom the keyboard or the three
  words.
- She is as big as fits: the party and the opening give her the room first and the
  words what is left above her.
- Words to read are in the big letters of `font.rs` wherever they fit, each letter
  its own color in what is typed: done, next, to come.
- The arrows move one marker over whatever is on the screen. Sideways it stays in
  its row; up and down it takes what is most nearly ahead. (The first rule tried let
  Left from the first game land on the settings far below it.)

## Her voice

**Why recorded clips.** Three ways were offered: clips made once by a speech
program, a real voice recorded by the family, or both. Speaking through the system's
own voice at run time was advised against (fine on macOS, robotic on Linux). The
owner chose generated clips.

**How.** `words::spoken()` lists every line: 26 letters, about 220 words, about 55
phrases, 56 stories, 372 in all. `tools/voice.py` has Piper say each, raises the
pitch by 1.35 and slows it by 1.12 (she is a kitty talking to small children), trims
the silence, evens the loudness and packs the clip as IMA ADPCM at 22,050 samples a
second. About 3.5 MB. What she says in the game is clips joined with a tenth of a
second between them, so "Can you type" and "cat" make a sentence.

**Which voice.** Nobody could listen while this was built, so a speech recognizer
(Whisper, `tools/voice-check.py`) stood in for ears: it transcribed each clip and the
transcript was compared with the text. Single letters and short words are the hard
case, so voices were compared on those.

| Voice (Piper) | Licence of its data | Single words and letters heard as written |
|---|---|---|
| en_US-hfc_female-medium | CC BY-NC-SA 4.0 | 45 of 66 |
| en_US-libritts_r-medium, reader 20 | **CC BY 4.0** | **50 of 66** |
| en_US-libritts_r-medium, readers 0, 3, 10 | CC BY 4.0 | 38, 38, 43 of 66 |
| en_GB-alba-medium | CC BY 4.0 | 31 of 66 |
| en_GB-cori-high | public domain | 27 of 66 |
| en_US-ljspeech-high | public domain | 15 of 66 |
| en_US-kathleen-low | CC0 | 4 of 66 |
| en_US-amy-medium | unclear (built on a restricted voice) | 50 of 68, another sample |
| en_US-kristin-medium | public domain | 35 of 68, another sample |

The clips are given away with a program under the MIT licence, so a voice whose data
forbids commercial use (hfc_female) or whose terms are unclear (amy, lessac) was not
used, although hfc_female was the first choice until its licence was read. LibriTTS-R
asks only for credit, which `voice/README.md` gives. Of 21 of its female readers
tried on a smaller sample, reader 20 was heard best (30 of 38; the next 29).

Also tried, and no help: no pitch change (216 against 212 of 262), more room before
the first sound, and saying the word inside a sentence and cutting it out.

**Retakes.** Piper does not say a thing the same way twice, and about one single word
in six came out with its first sound swallowed ("pie" heard as "hi", "pear" as
"hair"). `tools/voice-check.py --retake 8` has each misheard clip said again until a
take is heard as written. That took the whole set from 317 to 356 of 372. The 16 left
are mostly words that sound like another ("sea", "deer", "shoe").

**The end of the chain was checked too**: the real game was run with a stand-in for
the system's player that kept what it was given, and the recognizer heard "Hi, I'm
Pink Kitty. Let's have fun!", "Can you type lamb?", "Well done."

What no recognizer can say is whether she sounds *nice*. That needs a person.

## Changed since

- **Gifts reset on every start** (0.1.1). 0.1.0 kept them between runs, which was
  my choice: fungeo's stars reset because the owner had asked for that, but a
  dressing room that empties seemed worse. Having seen it, the owner asked for the
  same as fungeo. Nothing about the gifts is written down now.
- **A higher voice** (0.1.2). The owner asked for it and chose from seven samples of one
  sentence: the pitch raised by 1.14 (as it was), 1.25, 1.35, 1.5 and 1.65 the way
  `tools/voice.py` does it, which makes the voice smaller as it rises, and by 1.35
  and 1.5 with ffmpeg's `rubberband` keeping the size of the voice. The choice was
  1.35 the first way. All 372 clips were made again. The recognizer heard 332 as
  written after eight retakes and 338 after eight more, against 356 at 1.14. The 34
  left are six letters (I, L, Q, R, V, Z), 27 short words and one story that lost
  its first "The"; "car", "bag" and three of the letters were heard as "Bye", which
  looks like the recognizer failing on a short, high sound more than like her saying
  it wrong. The owner kept 1.35 with that list in hand.
- **A large way back from the dressing room** (0.1.2). The owner asked for it. The
  small "Esc Back" in the top corner, the same as in a game, became one button
  along the bottom, a third of the window wide, like those after a game.
- **What she won stays readable while she hops** (0.1.2). In an 80x24 window the
  tips of her ears went over two letters of "A gift for Pink Kitty: ..." for a
  moment at the start of a party. It was found when CI's end-to-end run read the
  gift's name at that moment ("B▀▀terfly"). The words are now drawn after her.
- **The answers of a story in letters** (0.1.3). The owner found the terminal's own text
  odd beside the huge letters. A first try put small letters wherever they fit: her
  speech bubble, the six games of the home screen, the levels. That was more than
  was meant ("the menu text can still be plain text ... I mostly mean the answer
  boxes"), so it was taken back, and what stayed is story time: the three answers
  and "Next" in big letters, or in small ones where those are too wide, and all
  three alike. The small letters are the big font at half its size, drawn with the
  sixteen quarter-block characters. They are narrow, because a quarter of a cell is
  twice as tall as it is wide, but they read well. Whether every terminal font has
  those characters was not tried beyond the one the pictures were drawn with.

## Decisions that were mine, not the owner's

These were not asked for or answered. They are easy to change, and each is where a
second opinion is most likely.

- **One level for all four games**, chosen on the home screen and remembered.
- **Twelve gifts**, and a heart for each game after that.
- **Esc does not leave the game from the home screen**, as in fungeo; Q does, after
  a wave.
- **One switch for all sound**, her voice included.
- **The words, sentences and stories**, all written for this game and read by nobody
  else yet.
- **Reader 20, and the pace** of her voice. (The pitch has been the owner's since
  the change above.)
