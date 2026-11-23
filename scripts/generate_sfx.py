#!/usr/bin/env python3
"""Procedural SFX generator for Anti_VS.
Generates synthesized audio effects compliant with the project's GNU GPL v3 license.

Currently generates:
  - assets/audio/obelisk_activate.wav: Deep seismic ground-cracking and tectonic fracture rumble.

Usage:
    python scripts/generate_sfx.py
"""

from __future__ import annotations

import math
import random
import struct
import wave
from pathlib import Path


def generate_obelisk_activate(output_path: Path) -> None:
    sample_rate = 44100
    duration = 3.2
    total_samples = int(sample_rate * duration)
    samples = [0.0] * total_samples

    # Deterministic seed for reproducible builds
    rng = random.Random(42)

    # 1. Seismic low-frequency tectonic rumble (30-65 Hz)
    for i in range(total_samples):
        t = i / sample_rate
        env = math.sin(math.pi * min(t / 2.8, 1.0)) ** 1.5
        f1 = 42.0 + 8.0 * math.sin(2.0 * math.pi * 1.5 * t)
        f2 = 68.0 + 12.0 * math.cos(2.0 * math.pi * 2.2 * t)
        rumble = 0.45 * math.sin(2.0 * math.pi * f1 * t) + 0.30 * math.sin(2.0 * math.pi * f2 * t)
        samples[i] += rumble * env

    # 2. Sharp stone cracks / fractures (snapping bedrock under tectonic pressure)
    crack_times = [0.06, 0.35, 0.72, 1.15, 1.60]
    crack_strengths = [1.0, 0.75, 0.85, 0.60, 0.45]
    for ct, cs in zip(crack_times, crack_strengths):
        start_idx = int(ct * sample_rate)
        crack_len = int(0.20 * sample_rate)
        for j in range(crack_len):
            idx = start_idx + j
            if idx >= total_samples:
                break
            tj = j / sample_rate
            # Sharp transient envelope
            env = math.exp(-tj * 26.0)
            # High frequency noise burst + resonant stone snap
            noise = (rng.random() * 2.0 - 1.0) * 0.75
            ping = math.sin(2.0 * math.pi * (820.0 - tj * 1800.0) * tj) * 0.5
            samples[idx] += (noise + ping) * env * cs * 0.65

    # 3. Grinding subterranean granite slabs (chattering bandpass texture)
    for i in range(total_samples):
        t = i / sample_rate
        env = (t / 0.5) if t < 0.5 else math.exp(-(t - 0.5) * 1.1)
        grind_noise = (rng.random() * 2.0 - 1.0)
        # Modulate with 11-14 Hz seismic shudder
        mod = 0.5 + 0.5 * math.sin(2.0 * math.pi * 12.5 * t)
        samples[i] += grind_noise * env * mod * 0.24

    # 4. Normalize and export as 16-bit PCM WAV
    max_val = max(abs(s) for s in samples) or 1.0
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(output_path), 'wb') as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(sample_rate)
        for s in samples:
            norm = (s / max_val) * 0.90
            val = int(max(-32767, min(32767, norm * 32767.0)))
            wav.writeframes(struct.pack('<h', val))

    print(f"Synthesized '{output_path}' successfully ({duration}s, {sample_rate}Hz, 16-bit PCM).")


if __name__ == "__main__":
    out_dir = Path("assets/audio")
    generate_obelisk_activate(out_dir / "obelisk_activate.wav")
