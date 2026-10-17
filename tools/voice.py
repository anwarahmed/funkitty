#!/usr/bin/env python3
"""Makes Pink Kitty's voice: one clip in voice/ for every line the game can say.

    cargo run -q -- voice-lines | tools/voice.py          only what is new or changed
    cargo run -q -- voice-lines | tools/voice.py --all    everything again

The lines come from the game itself (`funkitty voice-lines`: a name, a tab, the text),
so the game and its clips cannot disagree; `cargo test` fails until they match.
voice/lines.tsv records the text each clip was made from.

She is Chatterbox (https://huggingface.co/ResembleAI/chatterbox, MIT), which can say a
line with more or less feeling, speaking in the voice of a reference clip: Kokoro's
af_bella (https://huggingface.co/hexgrad/Kokoro-82M, Apache 2.0) reading REFERENCE.
Needs ffmpeg, Python 3.12 (torch has no wheels for newer yet) and, for Kokoro:

    FUNKITTY_KOKORO   the folder of kokoro-v1.0.onnx and voices-v1.0.bin
                      (default ~/.cache/funkitty-voice/kokoro)

    python3.12 -m venv ~/.cache/funkitty-voice/venv-chatterbox
    ~/.cache/funkitty-voice/venv-chatterbox/bin/pip install --extra-index-url https://download.pytorch.org/whl/cpu \
        chatterbox-tts kokoro-onnx soundfile faster-whisper "setuptools<81"
    mkdir -p ~/.cache/funkitty-voice/kokoro && cd ~/.cache/funkitty-voice/kokoro
    r=https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0
    curl -LO $r/kokoro-v1.0.onnx -LO $r/voices-v1.0.bin
    cargo run -q -- voice-lines | ~/.cache/funkitty-voice/venv-chatterbox/bin/python tools/voice.py

Chatterbox downloads its model the first time. On a laptop's processor it takes about
thirty seconds a clip, so the whole voice is four hours or so. (setuptools is held
back for the watermarker Chatterbox puts in what it makes, which is not heard.)

The clips are given away with the game, so the voice must be one whose licence allows
that (voice/README.md gives the credit). Before 0.2.0 she was Piper's
en_US-libritts_r-medium, before 0.3.0 Kokoro's bf_lily; docs/DESIGN.md says why.

How she sounds is the numbers below: the user's daughter listened to five voices at two
pitches and chose this one at 1.3. Each kind of line has a mood of its own (MOODS), the
feeling Chatterbox puts in it: the most when she is pleased, less when she comforts or
asks, least for a single letter or word. Change them and run with --all, then listen,
or at least run tools/voice-check.py.
"""
import array, os, re, subprocess, sys, tempfile

VOICE = "af_bella"    # whose voice she speaks in: Kokoro's, reading REFERENCE
REFERENCE = ("Oh, hello there! I'm so happy to see you today. Shall we play a little game together? "
             "We can read, and count, and find the letters. Oh, wow, you did it! That was wonderful!")
RATE = 22050          # samples a second; src/voice.rs expects exactly this
PITCH = 1.3           # how much higher than the voice she speaks, and how much smaller
QUIET = 600           # below this (of 32767) is silence, to be cut from both ends
PEAK = 27000          # every clip is made this loud at its loudest

# (clip names, as a pattern) -> (exaggeration, cfg_weight). The first that fits.
# Exaggeration is how much feeling; a lower cfg_weight lets a lively line take its time.
MOODS = [
    (r"(praise|party|yum|pounce|gift|dress)-\d+|heart|hello.*|bye|.*-intro", (1.0, 0.3)),
    (r"comfort-\d+", (0.65, 0.4)),
    (r"(letter|word|number)-.*", (0.5, 0.5)),
]
ASKING = (0.6, 0.45)  # everything else: what she asks, and the stories

# Said alone, a short line comes out as something else ("cat" as "but", "Wow!" as
# "How?"), so it is said in a sentence and cut out of it, at the quietest moment between
# it and the rest where the recognizer hears them: a single letter, word or number as a
# sentence of its own after one of these, a take at a time (run on, as in "The word
# is, cat.", the cut kept the end of "is": "It's cat")...
SINGLE = r"(letter|word|number)-.*"
CARRIERS = {"letter": ["Here is a letter. {}.", "Ready? {}."], "number": ["Here is a number. {}.", "Ready? {}."]}
CARRIER = ["Here is a word. {}.", "Ready? {}."]
# ...and a cheer of one or two words before this (any other short line, such as "plus",
# is a word).
AFTER = " Good job!"

HOME = os.path.expanduser("~/.cache/funkitty-voice")
KOKORO = os.environ.get("FUNKITTY_KOKORO", f"{HOME}/kokoro")


def mood(name):
    return next((m for pattern, m in MOODS if re.fullmatch(pattern, name)), ASKING)


def plain(text):
    return re.sub(r"[^a-z0-9 ]", "", text.lower().replace("-", " ")).split()


def carrier(name, text, take=0):
    """The sentence a line is said in, to be cut out of: (sentence, words before it,
    words of it), or None for a line said as it is."""
    words = len(plain(text))
    if words > 2 and not re.fullmatch(SINGLE, name):
        return None
    if text.endswith("!") and not re.fullmatch(SINGLE, name):
        return text + AFTER, 0, words
    sentences = CARRIERS.get(name.split("-")[0], CARRIER)
    sentence = sentences[take % len(sentences)].format(text[0].upper() + text[1:].rstrip("."))
    return sentence, len(plain(sentence)) - words, words


_chatterbox = _whisper = None


def reference():
    """The clip whose voice she speaks in, made once by Kokoro, which says a thing the
    same way every time."""
    path = f"{HOME}/reference-{VOICE}.wav"
    if not os.path.exists(path):
        import numpy, soundfile
        from kokoro_onnx import Kokoro
        samples, rate = Kokoro(f"{KOKORO}/kokoro-v1.0.onnx", f"{KOKORO}/voices-v1.0.bin").create(
            REFERENCE, voice=VOICE, speed=1.0, lang="en-us")
        soundfile.write(path, numpy.asarray(samples, numpy.float32), rate)
    return path


def words(path):
    """The words the recognizer hears in a clip, with when each starts and ends."""
    global _whisper
    import numpy
    from faster_whisper import WhisperModel
    if _whisper is None:
        _whisper = WhisperModel(os.environ.get("WHISPER_MODEL", "small.en"), device="cpu", compute_type="int8")
    raw = run("ffmpeg", "-v", "error", "-i", path, "-ar", "16000", "-ac", "1", "-f", "f32le", "-", stdout=subprocess.PIPE).stdout
    segments, _ = _whisper.transcribe(numpy.frombuffer(raw, numpy.float32), language="en", beam_size=5, word_timestamps=True)
    return [w for s in segments for w in s.words]


def speak(lines, folder, take=0):
    """Has Chatterbox say every (name, text), each into folder/name.wav. It says a thing
    differently every time, and `take` picks which. A short line is cut out of the
    sentence it is said in; when the recognizer does not hear that sentence as written,
    the next of a few tries, else the line said alone."""
    global _chatterbox
    import contextlib, io, warnings
    warnings.filterwarnings("ignore")
    import torch, torchaudio
    from chatterbox.tts import ChatterboxTTS
    if _chatterbox is None:
        _chatterbox = ChatterboxTTS.from_pretrained(device="cpu")
    voice = reference()
    for name, text in lines:
        exaggeration, cfg = mood(name)
        cut = carrier(name, text, take)
        path = f"{folder}/{name}.wav"
        for attempt in range(4 if cut else 1):
            torch.manual_seed(take * 10 + attempt)
            said = cut[0] if cut and attempt < 3 else text
            with contextlib.redirect_stderr(io.StringIO()):
                samples = _chatterbox.generate(said, audio_prompt_path=voice, exaggeration=exaggeration, cfg_weight=cfg)
            torchaudio.save(path, samples, _chatterbox.sr)
            if said == text:
                break
            # Only the words around the line need be heard right ("Twenty-five" may be "25").
            heard = words(path)
            _, before, length = cut
            around = plain(" ".join(h.word for h in heard[:before] if before) or " ".join(h.word for h in heard[length:]))
            if len(heard) <= max(before, length) or around != plain(said[:-len(text) - 1] if before else AFTER):
                continue
            first, second = (heard[before - 1], heard[before]) if before else (heard[length - 1], heard[length])
            at = quietest(samples[0], _chatterbox.sr, first.end - 0.04, second.start + 0.08)
            torchaudio.save(path, samples[:, at:] if before else samples[:, :at], _chatterbox.sr)
            break


def quietest(samples, rate, start, end):
    """Where between two times (in seconds) a clip is quietest, by 20 ms windows."""
    step = rate // 100
    first, last = max(0, int(start * rate)), min(len(samples) - 2 * step, int(end * rate))
    if last <= first:
        return int((start + end) / 2 * rate)
    return min(range(first, last, step), key=lambda i: float(samples[i:i + 2 * step].abs().mean())) + step


def run(*command, **more):
    return subprocess.run(command, check=True, stderr=subprocess.DEVNULL, **more)


def finish(spoken, clip):
    """Raises the pitch, cuts the silence from the ends, evens the loudness and packs the clip."""
    rate = int(run("ffprobe", "-v", "error", "-show_entries", "stream=sample_rate", "-of", "csv=p=0", spoken,
                   stdout=subprocess.PIPE, text=True).stdout.split()[0])
    higher = f"asetrate={rate * PITCH},aresample={RATE},atempo={1 / PITCH}"
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
