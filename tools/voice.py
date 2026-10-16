#!/usr/bin/env python3
"""Makes Pink Kitty's voice: one clip in voice/ for every line the game can say.

    cargo run -q -- voice-lines | tools/voice.py          only what is new or changed
    cargo run -q -- voice-lines | tools/voice.py --all    everything again

The lines come from the game itself (`funkitty voice-lines`: a name, a tab, the text),
so the game and its clips cannot disagree; `cargo test` fails until they match.
voice/lines.tsv records the text each clip was made from.

Needs ffmpeg, and Kokoro (https://huggingface.co/hexgrad/Kokoro-82M, Apache 2.0) run
through kokoro-onnx, in a Python that has it (the one tools/voice-check.py uses too):

    FUNKITTY_KOKORO   the folder of kokoro-v1.0.onnx and voices-v1.0.bin
                      (default ~/.cache/funkitty-voice/kokoro)

    python3 -m venv ~/.cache/funkitty-voice/venv
    ~/.cache/funkitty-voice/venv/bin/pip install kokoro-onnx soundfile faster-whisper
    mkdir -p ~/.cache/funkitty-voice/kokoro && cd ~/.cache/funkitty-voice/kokoro
    r=https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0
    curl -LO $r/kokoro-v1.0.onnx -LO $r/voices-v1.0.bin
    cargo run -q -- voice-lines | ~/.cache/funkitty-voice/venv/bin/python tools/voice.py

The clips are given away with the game, so the voice must be one whose licence allows
that: Kokoro's model and voices are Apache 2.0 (voice/README.md gives the credit).
Before 0.2.0 she was Piper's en_US-libritts_r-medium; docs/DESIGN.md says why that
changed.

How she sounds is the numbers below: the user listened to six of Kokoro's voices at
three pitches and chose bf_lily at 1.3. Each kind of line has a mood of its own (MOODS):
a little quicker and brighter when she is pleased, slower and softer when she comforts,
slow and clear for a single letter or word. Change them and run with --all, then
listen, or at least run tools/voice-check.py.
"""
import array, os, re, subprocess, sys, tempfile

VOICE = "bf_lily"     # which of Kokoro's voices she is
LANG = "en-gb"        # how her words are sounded out: bf_ is a British voice
RATE = 22050          # samples a second; src/voice.rs expects exactly this
KOKORO_RATE = 24000   # what Kokoro makes
PITCH = 1.3           # how much higher than the voice she speaks, and how much smaller
QUIET = 600           # below this (of 32767) is silence, to be cut from both ends
PEAK = 27000          # every clip is made this loud at its loudest

# (clip names, as a pattern) -> (speed, pitch on top of PITCH). The first that fits.
MOODS = [
    (r"(praise|party|yum|pounce|gift|dress)-\d+|heart|hello.*|.*-intro", (1.04, 1.05)),
    (r"comfort-\d+", (0.88, 0.97)),
    (r"bye", (0.95, 1.02)),
    (r"(letter|word|number)-.*", (0.88, 1.0)),
    (r"story-.*", (0.94, 1.0)),
]
ASKING = (0.94, 1.0)  # everything else: what she asks

# Said alone, a short word often comes out with a sound after it ("sheep" as "Sheeba");
# in quotation marks less often. tools/voice-check.py retakes one that is still
# misheard by cutting it out of one of these sentences, where it is said whole.
SINGLE = r"(letter|word|number)-.*"
CARRIERS = ["Can you type {}?", "Where is the {}?", "Pop the word {}!", "I like the {}."]

HOME = os.path.expanduser("~/.cache/funkitty-voice")
KOKORO = os.environ.get("FUNKITTY_KOKORO", f"{HOME}/kokoro")


def mood(name):
    return next((m for pattern, m in MOODS if re.fullmatch(pattern, name)), ASKING)


_kokoro = None


def single(name):
    return re.fullmatch(SINGLE, name) is not None


def carrier(name, text, take):
    """The sentence a retake of a single word is said in, to be cut out of; None for a
    take that is the word alone. Every other retake is one."""
    if not single(name) or take % 2 == 0:
        return None
    return CARRIERS[(take // 2) % len(CARRIERS)].format(text.rstrip("."))


def speak(lines, folder, take=0):
    """Has Kokoro say every (name, text), each into folder/name.wav. Kokoro says a thing
    the same way every time, so another `take` is said a shade faster or slower. A
    single letter, word or number is said in quotation marks."""
    global _kokoro
    import numpy, soundfile
    from kokoro_onnx import Kokoro
    if _kokoro is None:
        _kokoro = Kokoro(f"{KOKORO}/kokoro-v1.0.onnx", f"{KOKORO}/voices-v1.0.bin")
    nudge = [1.0, 0.95, 1.05, 0.9, 1.1][take % 5] if take else 1.0
    for name, text in lines:
        if single(name) and not text.endswith(("?", "!")) and " " not in text.strip("."):
            text = f"\u201c{text}\u201d"
        samples, rate = _kokoro.create(text, voice=VOICE, speed=mood(name)[0] * nudge, lang=LANG)
        soundfile.write(f"{folder}/{name}.wav", numpy.asarray(samples, numpy.float32), rate)


def run(*command, **more):
    return subprocess.run(command, check=True, stderr=subprocess.DEVNULL, **more)


def finish(spoken, clip):
    """Raises the pitch, cuts the silence from the ends, evens the loudness and packs the clip."""
    pitch = PITCH * mood(os.path.basename(clip)[:-4])[1]
    higher = f"asetrate={KOKORO_RATE * pitch},aresample={RATE},atempo={1 / pitch}"
    raw = run("ffmpeg", "-v", "error", "-i", spoken, "-af", higher, "-ac", "1", "-ar", str(RATE), "-f", "s16le", "-",
              stdout=subprocess.PIPE).stdout
    samples = array.array("h", raw[:len(raw) // 2 * 2])
    loud = [i for i, s in enumerate(samples) if abs(s) > QUIET]
    if not loud:
        sys.exit(f"voice.py: {spoken} is silent")
    pad = RATE // 40
    samples = samples[max(0, loud[0] - pad):loud[-1] + 3 * pad]
    gain = PEAK / max(abs(s) for s in samples)
    fade = RATE // 100
    for i in range(len(samples)):
        edge = min(1.0, i / fade, (len(samples) - 1 - i) / fade)
        samples[i] = int(max(-32767, min(32767, samples[i] * gain * edge)))
    run("ffmpeg", "-v", "error", "-y", "-f", "s16le", "-ar", str(RATE), "-ac", "1", "-i", "-", "-c:a", "adpcm_ima_wav",
        "-bitexact", "-map_metadata", "-1", clip, input=samples.tobytes())


def main():
    out = "voice"
    if "--out" in sys.argv:          # somewhere else, for trying a voice out
        out = sys.argv[sys.argv.index("--out") + 1]
    elif not os.path.isdir("voice"):
        sys.exit("voice.py: run it from the top of the checkout")
    os.makedirs(out, exist_ok=True)
    wanted = [tuple(line.rstrip("\n").split("\t", 1)) for line in sys.stdin if "\t" in line]
    if not wanted:
        sys.exit("voice.py: no lines on standard input (cargo run -q -- voice-lines | tools/voice.py)")
    record = f"{out}/lines.tsv"
    made = dict(line.rstrip("\n").split("\t", 1) for line in open(record)) if os.path.exists(record) else {}
    todo = [(name, text) for name, text in wanted
            if "--all" in sys.argv or made.get(name) != text or not os.path.exists(f"{out}/{name}.wav")]
    with tempfile.TemporaryDirectory() as folder:
        if todo:
            speak(todo, folder)
        for name, text in todo:
            finish(f"{folder}/{name}.wav", f"{out}/{name}.wav")
    names = {name for name, _ in wanted}
    stale = [f for f in os.listdir(out) if f.endswith(".wav") and f[:-4] not in names]
    for f in stale:
        os.remove(f"{out}/{f}")
    with open(record, "w") as f:
        f.writelines(f"{name}\t{text}\n" for name, text in sorted(wanted))
    size = sum(os.path.getsize(f"{out}/{name}.wav") for name in names)
    print(f"{len(todo)} made, {len(stale)} removed, {len(wanted)} clips, {size / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
