"""Extract F0 + F1–F4 (Praat/parselmouth) for the kept crossover clips, so we can
build resonance (vocal-tract-length) metrics. Aggregates to per-speaker medians.

Run:  .venv/bin/python -m analysis.extract_kept_formants [cap_per_speaker]
Out:  analysis/kept_features.csv
"""
import glob
import os
import sys
from collections import defaultdict

import numpy as np
import pandas as pd
import parselmouth
import soundfile as sf

HERE = os.path.dirname(__file__)
KEPT = os.path.join(HERE, "..", "data", "kept")
SPEAKERS = os.path.join(HERE, "..", "data", "raw", "LibriSpeech", "SPEAKERS.TXT")
MAX_FORMANT = 5500.0


def gender_map():
    out = {}
    with open(SPEAKERS) as f:
        for line in f:
            if line.startswith(";"):
                continue
            p = [x.strip() for x in line.split("|")]
            if len(p) >= 2 and p[0].isdigit():
                out[p[0]] = {"F": "female", "M": "male"}.get(p[1], p[1])
    return out


def clip_features(path):
    arr, sr = sf.read(path)
    if getattr(arr, "ndim", 1) > 1:
        arr = arr.mean(axis=1)
    snd = parselmouth.Sound(arr.astype("float64"), sampling_frequency=sr)
    pitch = snd.to_pitch(time_step=0.01, pitch_floor=60.0, pitch_ceiling=500.0)
    f0 = pitch.selected_array["frequency"]
    voiced_t = pitch.xs()[f0 > 0]
    if voiced_t.size < 5:
        return None
    formant = snd.to_formant_burg(time_step=0.01, max_number_of_formants=5,
                                  maximum_formant=MAX_FORMANT)
    cols = {1: [], 2: [], 3: [], 4: []}
    for t in voiced_t:
        for k in cols:
            v = formant.get_value_at_time(k, t)
            if v and not np.isnan(v):
                cols[k].append(v)
    out = {"f0": float(np.median(f0[f0 > 0]))}
    for k in cols:
        out[f"f{k}"] = float(np.median(cols[k])) if cols[k] else None
    return out


def main(cap=4):
    gmap = gender_map()
    by_spk = defaultdict(list)
    for p in sorted(glob.glob(os.path.join(KEPT, "*.flac"))):
        # ls-<spk>-<chap>-<utt>.flac
        spk = os.path.basename(p).split("-")[1]
        by_spk[spk].append(p)

    rows = []
    done = 0
    for spk, clips in by_spk.items():
        feats = [clip_features(c) for c in clips[:cap]]
        feats = [f for f in feats if f]
        if not feats:
            continue
        agg = {"speaker": f"ls-{spk}", "gender": gmap.get(spk), "n_clips": len(feats)}
        for key in ("f0", "f1", "f2", "f3", "f4"):
            vals = [f[key] for f in feats if f.get(key) is not None]
            agg[key] = float(np.median(vals)) if vals else None
        rows.append(agg)
        done += 1
        if done % 50 == 0:
            print(f"  {done} speakers …", flush=True)

    df = pd.DataFrame(rows)
    out = os.path.join(HERE, "kept_features.csv")
    df.to_csv(out, index=False)
    print(f"Done: {len(df)} speakers → {out}")


if __name__ == "__main__":
    main(int(sys.argv[1]) if len(sys.argv) > 1 else 4)
