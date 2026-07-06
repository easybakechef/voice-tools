"""Mine for voices where PITCH and RESONANCE disagree:

  • high-pitched men whose resonance is still male-typical
  • low-pitched women whose resonance is still female-typical

These are the speakers that let us prove resonance is independent of pitch
(their pitch points the "wrong" way; a good resonance metric should still call
them correctly). They also populate the pitch-overlap band the report flagged.

Run:  .venv/bin/python -m analysis.mine
Out:  prints a ranked table + writes analysis/divergent_candidates.csv
"""
import os
import sqlite3

import pandas as pd
from sklearn.linear_model import LogisticRegression
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler

from analysis.report import load_speakers

HERE = os.path.dirname(__file__)
DB = os.path.join(HERE, "..", "data", "samples.db")

# thresholds (robust z within gender for pitch; resonance score 0=male..100=female)
PITCH_Z = 1.0          # pitch this many σ toward the other gender
RES_MALE_MAX = 45.0    # men: resonance still reads male
RES_FEM_MIN = 55.0     # women: resonance still reads female


def robust_z(s: pd.Series) -> pd.Series:
    med = s.median()
    mad = (s - med).abs().median() * 1.4826
    scale = mad if mad else (s.std() or 1.0)
    return (s - med) / scale


def representative_paths(speakers):
    con = sqlite3.connect(DB)
    out = {}
    for dataset, spk in speakers:
        row = con.execute(
            "select path from samples where dataset=? and speaker=? order by duration desc limit 1",
            (dataset, spk)).fetchone()
        out[(dataset, spk)] = row[0] if row else None
    con.close()
    return out


def main():
    spk = load_speakers()

    # In-sample resonance axis (formants only) → 0..100 (higher = more female-typical)
    res_model = make_pipeline(StandardScaler(), LogisticRegression(max_iter=1000))
    res_model.fit(spk[["f1", "f2"]].values, spk["y"].values)
    spk["resonance_score"] = (res_model.predict_proba(spk[["f1", "f2"]].values)[:, 1] * 100).round(1)

    # Pitch deviation toward the OTHER gender, in robust σ (positive = atypical).
    spk["pitch_z_raw"] = spk.groupby("gender")["f0"].transform(robust_z)
    spk["pitch_toward_other"] = spk.apply(
        lambda r: r["pitch_z_raw"] if r["gender"] == "male" else -r["pitch_z_raw"], axis=1)

    male = spk[(spk.gender == "male") & (spk.pitch_toward_other > PITCH_Z)
               & (spk.resonance_score < RES_MALE_MAX)]
    female = spk[(spk.gender == "female") & (spk.pitch_toward_other > PITCH_Z)
                 & (spk.resonance_score > RES_FEM_MIN)]
    cand = pd.concat([male, female]).copy()
    cand["pitch_divergence"] = cand["pitch_toward_other"].round(2)
    cand = cand.sort_values("pitch_divergence", ascending=False)

    if cand.empty:
        print("No pitch-divergent / resonance-typical speakers at current thresholds.")
        return

    paths = representative_paths(list(cand[["dataset", "speaker"]].itertuples(index=False, name=None)))
    cand["sample"] = [paths.get((d, s)) for d, s in zip(cand.dataset, cand.speaker)]

    cols = ["dataset", "speaker", "gender", "f0", "f1", "f2",
            "pitch_divergence", "resonance_score"]
    print(f"\n{len(cand)} candidates (pitch atypical for gender, resonance still typical):\n")
    print(cand[cols].to_string(index=False,
          formatters={"f0": "{:.0f}".format, "f1": "{:.0f}".format, "f2": "{:.0f}".format}))

    out = os.path.join(HERE, "divergent_candidates.csv")
    cand[cols + ["sample"]].to_csv(out, index=False)
    print(f"\nWrote {out}")


if __name__ == "__main__":
    main()
