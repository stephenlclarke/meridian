"""Source-transcribed waveform fixtures, NOT executed Swift oracle results.
Reference: a11fe16924d87d06021816f9037952afcbac08db, Sound.swift.
Run only to intentionally update the reference fixtures after source review.
No Rust implementation is imported. No audio output is opened.
"""
import hashlib
import json
import math
from pathlib import Path
import struct


def reference_wave(name, duration, frequency):
    rate = 22050
    count = int(duration * rate)
    phase = 0.0
    noise = 0x454C4954
    samples = []
    for index in range(count):
        progress = index / count
        phase += frequency(progress) / rate
        noise = (noise * 1664525 + 1013904223) & 0xFFFFFFFF
        sign = (1 if noise & 1 else -1) if name == "explosion" else (-1 if math.fmod(phase, 1) < .5 else 1)
        samples.append(int(sign * 5000 * (1 - progress)))
    pcm = struct.pack("<" + "h" * count, *samples)
    return b"RIFF" + struct.pack("<I", 36 + len(pcm)) + b"WAVEfmt " + struct.pack("<IHHIIHH", 16, 1, 1, rate, rate * 2, 2, 16) + b"data" + struct.pack("<I", len(pcm)) + pcm


def main():
    root = Path(__file__).parent
    effects = [
        ("laser", .12, lambda p: 1100 - p * 900),
        ("launch", .8, lambda p: 180 + p * 800),
        ("dock", .6, lambda p: 900 - p * 700),
        ("explosion", .35, lambda p: 60 + p * 30),
        ("ping", .1, lambda p: 880),
        ("ecm", .3, lambda p: 600 + math.sin(p * math.pi * 12) * 300),
    ]
    manifest = {"referenceCommit": "a11fe16924d87d06021816f9037952afcbac08db", "source": "Sources/L337Core/Sound.swift", "method": "independent Python transcription, not an executed Swift oracle", "fixtures": []}
    for name, duration, frequency in effects:
        wave = reference_wave(name, duration, frequency)
        (root / (name + ".wav")).write_bytes(wave)
        manifest["fixtures"].append({"name": name, "bytes": len(wave), "samples": (len(wave) - 44) // 2, "sha256": hashlib.sha256(wave).hexdigest()})
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
