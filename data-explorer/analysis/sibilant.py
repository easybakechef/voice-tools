"""Extract sibilant-fricative spectral features per speaker, at NATIVE 16 kHz.

Detects unvoiced high-frequency (fricative) frames and computes the four spectral
moments (M1 center of gravity, M2 spread, M3 skew, M4 kurtosis) over the sibilant
band, plus a high-band (>5 kHz) energy ratio and the peak frequency — the standard
sibilant descriptors (Jongman, Wayland & Wong 2000). Aggregated to per-speaker
medians. Unlike the main pipeline this uses NO resampling, so it keeps the
5.5-8 kHz band where /s/ energy lives.

Run:  .venv/bin/python -m analysis.sibilant
Out:  analysis/sibilant_features.csv
"""
import glob
import os
from collections import defaultdict
from concurrent.futures import ProcessPoolExecutor

import numpy as np
import pandas as pd
import soundfile as sf

HERE = os.path.dirname(__file__)
KEPT = os.path.join(HERE, "..", "data", "kept")
BAND = (1000.0, 8000.0)   # sibilant analysis band (Hz)


def clip_sibilant(path):
    x, sr = sf.read(path)
    if getattr(x, "ndim", 1) > 1:
        x = x.mean(axis=1)
    w = int(0.025 * sr)
    hop = int(0.010 * sr)
    if len(x) < w + hop:
        return None
    nfr = (len(x) - w) // hop + 1
    idx = np.arange(w)[None, :] + hop * np.arange(nfr)[:, None]
    fr = x[idx]
    win = np.hanning(w)
    sp = np.abs(np.fft.rfft(fr * win, axis=1)) ** 2  # power
    f = np.fft.rfftfreq(w, 1.0 / sr)
    energy = sp.sum(1)
    zcr = np.mean(np.abs(np.diff(np.sign(fr), axis=1)), axis=1) / 2.0
    cog_full = (f[None, :] * sp).sum(1) / (energy + 1e-12)
    hi = sp[:, f >= 1000].sum(1)
    lo = sp[:, f < 1000].sum(1)
    # sibilant frame: unvoiced (high ZCR), high-frequency dominant, not silence
    sib = (zcr > 0.12) & (cog_full > 3000) & (hi > lo) & (energy > energy.max() * 0.02)
    if sib.sum() < 5:
        return None

    band = (f >= BAND[0]) & (f <= BAND[1])
    fb = f[band]
    P = sp[sib][:, band]
    Psum = P.sum(1) + 1e-12
    m1 = (fb[None, :] * P).sum(1) / Psum
    var = ((fb[None, :] - m1[:, None]) ** 2 * P).sum(1) / Psum
    m2 = np.sqrt(var)
    m3 = ((fb[None, :] - m1[:, None]) ** 3 * P).sum(1) / (Psum * (m2 ** 3 + 1e-12))
    m4 = ((fb[None, :] - m1[:, None]) ** 4 * P).sum(1) / (Psum * (m2 ** 4 + 1e-12)) - 3.0
    hiband = P[:, fb >= 5000].sum(1) / Psum          # >5 kHz energy fraction
    peak = fb[np.argmax(P, axis=1)]
    return dict(sib_m1=np.median(m1), sib_m2=np.median(m2), sib_m3=np.median(m3),
                sib_m4=np.median(m4), sib_hi=np.median(hiband), sib_peak=np.median(peak),
                sib_frac=float(sib.mean()))


def speaker_row(item):
    spk, clips = item
    feats = [clip_sibilant(c) for c in clips[:4]]
    feats = [f for f in feats if f]
    if not feats:
        return None
    out = {"speaker": f"ls-{spk}"}
    for k in ("sib_m1", "sib_m2", "sib_m3", "sib_m4", "sib_hi", "sib_peak", "sib_frac"):
        out[k] = float(np.median([f[k] for f in feats]))
    return out


def main():
    by_spk = defaultdict(list)
    for p in sorted(glob.glob(os.path.join(KEPT, "*.flac"))):
        spk = os.path.basename(p).split("-")[1]
        by_spk[spk].append(p)
    items = list(by_spk.items())
    rows = []
    with ProcessPoolExecutor() as ex:
        for i, r in enumerate(ex.map(speaker_row, items), 1):
            if r:
                rows.append(r)
            if i % 200 == 0:
                print(f"  {i}/{len(items)}", flush=True)
    df = pd.DataFrame(rows)
    out = os.path.join(HERE, "sibilant_features.csv")
    df.to_csv(out, index=False)
    print(f"wrote {len(df)} speakers → {out}")


if __name__ == "__main__":
    main()
