#!/usr/bin/env python3
"""Listens to the clips in voice/ and lists those that are not heard as their text.

    tools/voice-check.py [FOLDER]             needs: ffmpeg, and pip install faster-whisper
    tools/voice-check.py --retake 6           and has Piper say each of those again, up to
                                              six times, keeping a take that is heard right

A speech recognizer transcribes each clip and the result is compared with
lines.tsv. It is a way to find clips worth listening to, not a verdict: a single
letter or a short word alone is hard for a recognizer too ("sea" and "see" are the
same sound), so expect false alarms there and trust your ears.

Piper does not say a thing the same way twice, and now and then a word comes out
with its first sound swallowed ("pie" as "hi"). That is what --retake is for: it
keeps the first take the recognizer hears as written, and the old clip when none is.
"""
import importlib.util, os, re, shutil, subprocess, sys, tempfile
import numpy
from faster_whisper import WhisperModel

args = sys.argv[1:]
retakes = int(args.pop(args.index("--retake") + 1)) if "--retake" in args else 0
args = [a for a in args if a != "--retake"]
folder = args[0] if args else "voice"
model = WhisperModel(os.environ.get("WHISPER_MODEL", "small.en"), device="cpu", compute_type="int8")
plain = lambda text: re.sub(r"[^a-z0-9 ]", "", text.lower().replace("-", " ")).split()
LETTERS = {"a": "a ay eh hey", "b": "b be bee", "c": "c see sea", "d": "d dee", "e": "e ee", "f": "f ef eff", "g": "g gee jee",
           "h": "h aitch age", "i": "i eye", "j": "j jay", "k": "k kay ok okay", "l": "l el elle", "m": "m em", "n": "n en and",
           "o": "o oh", "p": "p pee pea", "q": "q queue cue", "r": "r are", "s": "s es ass", "t": "t tea tee", "u": "u you",
           "v": "v vee", "w": "w", "x": "x ex", "y": "y why", "z": "z zee"}


def hear(clip):
    # The recognizer wants 16,000 samples a second, as numbers from -1 to 1.
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", clip, "-ar", "16000", "-ac", "1", "-f", "f32le", "-"],
                         check=True, stdout=subprocess.PIPE).stdout
    segments, _ = model.transcribe(numpy.frombuffer(raw, numpy.float32), language="en", beam_size=5)
    return " ".join(s.text for s in segments).strip()


def right(name, text, heard):
    want, got = plain(text), plain(heard)
    if name.startswith("letter-"):
        return len(got) == 1 and got[0] in LETTERS[want[0]].split() or "".join(got) == "doubleu"
    return want == got


if retakes:
    spec = importlib.util.spec_from_file_location("voice", os.path.join(os.path.dirname(os.path.abspath(__file__)), "voice.py"))
    voice = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(voice)
bad = total = 0
for line in open(f"{folder}/lines.tsv"):
    name, text = line.rstrip("\n").split("\t", 1)
    clip = f"{folder}/{name}.wav"
    heard = hear(clip)
    total += 1
    if right(name, text, heard):
        continue
    for take in range(retakes):
        with tempfile.TemporaryDirectory() as tmp:
            voice.speak([(name, text)], tmp)
            voice.finish(f"{tmp}/{name}.wav", f"{tmp}/take.wav")
            again = hear(f"{tmp}/take.wav")
            if right(name, text, again):
                shutil.copy(f"{tmp}/take.wav", clip)
                print(f"{name:40} said {text!r:40} heard right at take {take + 2}")
                break
    else:
        bad += 1
        print(f"{name:40} said {text!r:40} heard {heard!r}")
print(f"{total - bad} of {total} heard as written")
