#!/usr/bin/env python3
"""Generate the project's original, deterministic pistol and glass effects (no external samples)."""
from pathlib import Path
import math
import random
import struct
import wave

root = Path(__file__).resolve().parents[1] / 'assets/audio'
root.mkdir(parents=True, exist_ok=True)
for name, duration in [('pistol', .18), ('glass', .36)]:
    rng = random.Random(114 if name == 'pistol' else 779)
    samples = []
    for i in range(int(44100 * duration)):
        t = i / 44100
        if name == 'pistol':
            value = (rng.uniform(-1, 1) * .7 * math.exp(-t * 55) + math.sin(2 * math.pi * (120 * t - 100 * t * t)) * .38 * math.exp(-t * 32)) * (1 - math.exp(-t * 4000))
        else:
            value = sum(math.sin(2 * math.pi * frequency * t + frequency * .01) * math.exp(-t * (13 + j * 3)) for j, frequency in enumerate([1709, 2903, 4217, 5701, 7213])) * .08 + rng.uniform(-1, 1) * .1 * math.exp(-t * 25)
        samples.append(int(max(-.85, min(.85, value)) * 32767))
    with wave.open(str(root / (name + '.wav')), 'wb') as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(44100)
        output.writeframes(struct.pack('<' + 'h' * len(samples), *samples))
