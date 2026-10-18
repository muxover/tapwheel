"""Writes the website's sample music: a calm 20 second loop, every sound made in code.

python3 synth.py out.wav seconds
"""
import sys, wave
import numpy as np

SR = 44100
out, secs = sys.argv[1], float(sys.argv[2])
n = int(SR * secs)
t = np.arange(n) / SR
bpm = 96
beat = 60 / bpm

def note(m):
    return 440.0 * 2 ** ((m - 69) / 12)

# A minor, F, C, G, one bar each
chords = [[57, 60, 64], [53, 57, 60], [48, 52, 55], [55, 59, 62]]
bar = beat * 4
mix = np.zeros(n)

# warm pad: detuned sines with slow swells
for i in range(int(secs / bar) + 1):
    start = i * bar
    s0, s1 = int(start * SR), min(n, int((start + bar + 0.6) * SR))
    if s0 >= n:
        break
    tt = t[s0:s1] - start
    env = np.minimum(1, tt / 0.8) * np.exp(-np.maximum(0, tt - bar) * 3)
    for m in chords[i % 4]:
        f = note(m)
        for d in (-0.15, 0.15):
            mix[s0:s1] += 0.045 * env * np.sin(2 * np.pi * f * (1 + d / 100) * tt)

# plucked arpeggio in eighths
pattern = [0, 1, 2, 1, 0, 2, 1, 2]
for k in range(int(secs / (beat / 2))):
    start = k * beat / 2
    chord = chords[int(start / bar) % 4]
    m = chord[pattern[k % 8]] + 12
    s0, s1 = int(start * SR), min(n, int((start + 1.2) * SR))
    tt = t[s0:s1] - start
    env = np.exp(-tt * 5.5) * np.minimum(1, tt / 0.004)
    f = note(m)
    mix[s0:s1] += 0.11 * env * (np.sin(2 * np.pi * f * tt) + 0.3 * np.sin(4 * np.pi * f * tt) * np.exp(-tt * 9))

# soft kick on beats 1 and 3, a brushed hat on the off beats
for k in range(int(secs / beat)):
    start = k * beat
    s0 = int(start * SR)
    if k % 2 == 0:
        s1 = min(n, s0 + int(0.35 * SR))
        tt = t[s0:s1] - start
        f = 45 + 70 * np.exp(-tt * 30)
        mix[s0:s1] += 0.35 * np.exp(-tt * 9) * np.sin(2 * np.pi * np.cumsum(f) / SR)
    h0 = int((start + beat / 2) * SR)
    h1 = min(n, h0 + int(0.08 * SR))
    if h0 < n:
        rng = np.random.default_rng(k)
        tt = t[h0:h1] - (start + beat / 2)
        noise = rng.standard_normal(h1 - h0)
        noise = np.diff(noise, prepend=0)  # brighter
        mix[h0:h1] += 0.025 * np.exp(-tt * 45) * noise

# gentle fade in and out, then normalise
fade = np.minimum(1, t / 0.5) * np.minimum(1, (secs - t) / 1.5)
mix *= fade
mix /= np.max(np.abs(mix)) * 1.12
stereo = np.stack([mix, np.roll(mix, int(0.012 * SR))], axis=1)
data = (stereo * 32767).astype("<i2")
with wave.open(out, "wb") as w:
    w.setnchannels(2)
    w.setsampwidth(2)
    w.setframerate(SR)
    w.writeframes(data.tobytes())
