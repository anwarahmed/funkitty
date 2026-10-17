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

- **Six more games** (0.2.0). Asked to make it "more fun and educational", I offered
  the ideas left from before. The owner chose all four (counting, a bubble game for
  the mouse, picture words, capital and small letters) "and a few more"; the more
  were pairs, a memory game, and "What's next?", patterns. Rhymes and sums, two more
  of the old ideas, went into Hard bubbles and pairs, and Hard counting. What each
  asks at each level:

  | Game | Easy (4-5) | Medium (6-7) | Hard (8-10) |
  |---|---|---|---|
  | Counting | 1 to 5 things | 3 to 10 | adding, and taking away, to 10 |
  | Bubbles | pop the letter | pop the word | pop a word that rhymes |
  | Picture words | which picture is the word | which word is the picture | spell the picture |
  | Big and little | the little letter for a big one | the big for a little | the same word in little letters |
  | Pairs | 3 pairs of pictures | 4 of a big and a little letter | 5 of rhyming words |
  | What's next? | a pattern of pictures | counting up and down, the alphabet, patterns of three | twos, fives, tens, every other letter |

  Four of them are story time's engine (three answers, one right, a wrong one
  costing nothing), which kept the rules for keys and for drawing answers in one
  place. The wrong answers are chosen to teach: the nearest numbers, the letter
  turned over (b for d, p for q), the same word with a letter turned over.
- **Small letters in the big font** (0.2.0), which big and little needed. They are
  used wherever text is written in mixed case, so the title, a story's sentence and
  her bigger speech bubbles changed from all capitals.
- **A new voice** (0.2.0). The owner found Piper's "robotic/androidish" and asked for
  something more like a cute kitty, with tone and feeling. Kokoro-82M (Apache 2.0,
  so its clips can be given away) was tried in six voices at three pitches each, on
  five lines that each have a mood ("Hi! I'm Pink Kitty! Let's have fun!", a
  question, "Yummy! Perfect!", "Oh, almost! You can do it!", "Hooray! You are a
  star!"), beside Piper as it was. The recognizer heard all nineteen samples whole.
  The owner chose `bf_lily`, a British voice, at 1.3. Kokoro has no control for
  feeling, so each kind of line has its own speed and pitch (`MOODS` in
  `tools/voice.py`): pleased is quicker and higher, comforting slower and lower,
  single letters and words slow and clear. Kokoro also says a thing the same way
  every time, so a retake is said a shade faster or slower instead.

  Made in Lily, the whole set was heard as written 304 times of 468, against Piper's
  338 of 372, and nearly all the misses were single words. Said alone, a short word
  often came out with a sound after it ("sheep" as "Sheeba", "wall" as "wallet");
  in a sentence it was whole. On the 44 words missed most, nothing about how they
  were said helped Lily much (speed 1.0, no pitch, American sounds, "!", "...": 0 to
  3 right), quotation marks did a little (12), and the other voices did better in
  quotation marks: Bella 32, Heart 29, Nicole 28, Sky 18. Joined after "Can you
  type", Lily's clips of those words were heard 14 times of 42 and Piper's 30. Offered
  Bella for everything, Lily for everything as she was, Lily with her words fixed
  up, or Lily with Bella saying the words, the owner chose Lily with her words fixed
  up: in quotation marks, and a word still misheard cut out of a sentence ("Can you
  type sheep?") from where the recognizer hears it begin. That made 375 of 468.
- **An expressive voice** (0.3.0). The owner's daughter did not like Lily, and the
  owner asked for a voice that is expressive, right for a cute little kitty, not
  monotonous, with emotions in it, and for a few options. Kokoro has no control for
  feeling. Of the open models that do, Chatterbox (Resemble AI, MIT) runs on this
  laptop's processor and has one knob for it, exaggeration; it speaks in the voice of
  any reference clip. (Also considered: Dia and Step Audio EditX, Apache 2.0, heavier;
  Orpheus, whose weights carry Llama's licence; Fish Speech and Higgs Audio, not for
  commercial use.) Five options were made, each saying the same ten lines (hello, a
  question, comfort, praise, a party, a treat, goodbye) with the most feeling when
  pleased: Chatterbox in the voice of Kokoro's `af_heart`, `af_bella` and `af_sky`
  reading a cheerful passage, in its own voice, and Chatterbox Turbo in Heart's voice
  with its `[happy]`, `[laugh]` and `[chuckle]` tags. Each at pitch 1.15 and 1.3, beside
  Lily as she was. How far the pitch moves, from the 5th to the 95th percentile, was
  11 semitones for Lily and 13 to 19 for the new ones (Chatterbox's own voice the
  most). The daughter chose Bella at 1.3.

  Chatterbox says a short line badly alone: of thirteen letters, words and cheers said
  alone in Bella's voice, the recognizer heard five right ("cat" as "but", "Wow!" as
  "How?"). Said at the end of a sentence ("The word is, cat.") or before "Good job!"
  and cut out of it, ten of ten were right, nine at the first take. The first cut was
  where the recognizer heard the word begin, which kept the end of the word before
  ("or A.", "is butterfly"); the cut is now at the quietest 20 ms between the two
  words, with a comma in the sentence to make a pause there. Chatterbox says a thing
  differently every time, so a retake is just another take.

  The first whole set, cut out of "The word is, X.", was heard as written 317 times of
  468, and 115 of the misses were words. Of 24 of those words said again, none was
  right in that sentence, 12 as a sentence of their own after "Here is a word." and
  13 after "Ready?", so a short line is now its own sentence after one of those two,
  in turns. Six retakes took the set to 426; eight more for each of the 35 still
  missed, and then nine more said as an exclamation ("Ready? Book!") keeping the take
  heard closest to the word, took it to 442. (Those last two passes were a throwaway
  script; tools/voice-check.py --retake does the first six.) Before them, four clips
  the recognizer heard as rude words ("book", "hook", "chick", "six") were in the set;
  none is now, and a check for that is worth repeating whenever the clips are made
  again. What is left is short words with a short vowel and words that sound like
  another.

## Decisions that were mine, not the owner's

These were not asked for or answered. They are easy to change, and each is where a
second opinion is most likely.

- **One level for all the games**, chosen on the home screen and remembered.
- **The keys of the home screen** since there are ten games: 1 to 9 and 0, D for
  dress up and R for surprise (they were 5 and 6). Three games to a row.
- **What the six new games ask** at each level, the pictures (drawn by me, 22 of them
  and 7 little ones), the rhymes, and that bubbles and pairs end after six pops and
  when every pair is found.
- **Twelve gifts**, and a heart for each game after that.
- **Esc does not leave the game from the home screen**, as in fungeo; Q does, after
  a wave.
- **One switch for all sound**, her voice included.
- **The words, sentences and stories**, all written for this game and read by nobody
  else yet.
- **How much feeling** goes into each kind of line (`MOODS` in `tools/voice.py`), the
  passage Bella reads for the reference clip, and the sentences short lines are cut
  out of. (The voice and the pitch are the owner's daughter's choice.)
