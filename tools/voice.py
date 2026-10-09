#!/usr/bin/env python3
"""Makes Pink Kitty's voice: one clip in voice/ for every line the game can say.

    cargo run -q -- voice-lines | tools/voice.py          only what is new or changed
    cargo run -q -- voice-lines | tools/voice.py --all    everything again

The lines come from the game itself (`funkitty voice-lines`: a name, a tab, the text),
so the game and its clips cannot disagree; `cargo test` fails until they match.
voice/lines.tsv records the text each clip was made from.

Needs ffmpeg, and Piper (https://github.com/rhasspy/piper) with a voice:

    FUNKITTY_PIPER   the piper program    (default ~/.cache/funkitty-voice/piper/piper)
    FUNKITTY_VOICE   the voice's .onnx    (default ~/.cache/funkitty-voice/en_US-libritts_r-medium.onnx)

    mkdir -p ~/.cache/funkitty-voice && cd ~/.cache/funkitty-voice
    curl -L https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz | tar xz
    v=https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx
    curl -LO $v -LO $v.json

The clips are given away with the game, so the voice must be one whose licence allows
that. This one was trained on LibriTTS-R, which is CC BY 4.0: voice/README.md gives
the credit that asks for. (en_US-hfc_female was a little clearer in tests, but its
data is CC BY-NC-SA, which a game under the MIT licence cannot pass on.)

How she sounds is the numbers below. Change them and run with --all, then listen, or
at least run tools/voice-check.py.
"""
import array, json, os, subprocess, sys, tempfile

SPEAKER = 20          # which of the voice's 904 readers she is
RATE = 22050          # samples a second; src/voice.rs expects exactly this
PITCH = 1.35          # how much higher than the voice she speaks
SLOW = 1.12           # how much slower than the voice she speaks: she talks to small children
QUIET = 600           # below this (of 32767) is silence, to be cut from both ends
PEAK = 27000          # every clip is made this loud at its loudest

HOME = os.path.expanduser("~/.cache/funkitty-voice")
PIPER = os.environ.get("FUNKITTY_PIPER", f"{HOME}/piper/piper")
VOICE = os.environ.get("FUNKITTY_VOICE", f"{HOME}/en_US-libritts_r-medium.onnx")


def run(*command, **more):
    return subprocess.run(command, check=True, stderr=subprocess.DEVNULL, **more)


def speak(lines, folder):
    """Has Piper say every (name, text), each into folder/name.wav."""
    asked = "".join(json.dumps({"text": text, "output_file": f"{folder}/{name}.wav"}) + "\n" for name, text in lines)
    run(PIPER, "--model", VOICE, "--speaker", str(SPEAKER), "--json-input", "--length_scale", str(SLOW), "--sentence_silence", "0.12",
        input=asked.encode(), stdout=subprocess.DEVNULL)


def finish(spoken, clip):
    """Raises the pitch, cuts the silence from the ends, evens the loudness and packs the clip."""
    higher = f"asetrate={RATE * PITCH},aresample={RATE},atempo={1 / PITCH}"
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
