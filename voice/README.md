# Pink Kitty's voice

The `.wav` files here are what Pink Kitty says: one clip for each line of
`lines.tsv`, made by `tools/voice.py` and built into the game by `build.rs`. They are
WAV files in IMA ADPCM, one channel, 22,050 samples a second; most players and
`ffplay` will play them.

They were made with [Piper](https://github.com/rhasspy/piper) (MIT licence) and its
voice `en_US-libritts_r-medium`, then raised in pitch and slowed a little. That voice
was trained on [LibriTTS-R](https://www.openslr.org/141/) (Koizumi and others, 2023),
a restored version of [LibriTTS](https://www.openslr.org/60/) (Zen and others, 2019),
which is made from public-domain [LibriVox](https://librivox.org) recordings.
LibriTTS-R and LibriTTS are licensed under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Every word
here was generated; none is a recording of a person.

Do not edit the clips by hand; change the text in `data/` or the numbers at the top
of `tools/voice.py` and run it again.
