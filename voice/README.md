# Pink Kitty's voice

The `.wav` files here are what Pink Kitty says: one clip for each line of
`lines.tsv`, made by `tools/voice.py` and built into the game by `build.rs`. They are
WAV files in IMA ADPCM, one channel, 22,050 samples a second; most players and
`ffplay` will play them.

They were made with [Kokoro](https://huggingface.co/hexgrad/Kokoro-82M) (Kokoro-82M,
v1.0, by hexgrad; model and voices under the
[Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0)), run through
[kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx) (MIT licence), in its voice
`bf_lily`, then raised in pitch, a little more for what she says when she is pleased.
Every word here was generated; none is a recording of a person.

Do not edit the clips by hand; change the text in `data/` or the numbers at the top
of `tools/voice.py` and run it again.
