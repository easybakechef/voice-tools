"""Build a large, balanced PITCH-MATCHED dataset from the full corpus scan.

Pairs each man with a woman of nearly identical pitch, so the male/female pitch
distributions are the same → pitch alone is forced to chance. Cleans octave
errors first (plausibility bounds on speaker-median pitch).

Run:  .venv/bin/python -m analysis.build_matched
Out:  analysis/pitch_matched_full.csv
"""
import os
import sqlite3

import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_auc_score
from sklearn.model_selection import StratifiedKFold, cross_val_predict
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler

HERE = os.path.dirname(__file__)
DB = os.path.join(HERE, "..", "data", "samples.db")
TOL = 6.0                 # Hz max gap within a matched pair
PLAUSIBLE = (80.0, 320.0)  # speaker-median pitch bounds (drop octave-error outliers)


def speakers() -> pd.DataFrame:
    con = sqlite3.connect(DB)
    df = pd.read_sql_query(
        "select speaker, gender, f0_median, kept from samples "
        "where gender in ('male','female') and f0_median is not null", con)
    con.close()
    spk = (df.groupby(["speaker", "gender"])
             .agg(pitch=("f0_median", "median"), n=("f0_median", "count"),
                  kept=("kept", "max"))
             .reset_index())
    return spk[spk.pitch.between(*PLAUSIBLE)]


def match(spk: pd.DataFrame) -> pd.DataFrame:
    men = spk[spk.gender == "male"].sort_values("pitch").to_dict("records")
    women = spk[spk.gender == "female"].sort_values("pitch").copy()
    used, rows = set(), []
    for m in men:
        avail = women[~women.speaker.isin(used)]
        if avail.empty:
            break
        j = (avail.pitch - m["pitch"]).abs().idxmin()
        w = women.loc[j]
        if abs(w.pitch - m["pitch"]) <= TOL:
            rows.append(m)
            rows.append(w.to_dict())
            used.add(w.speaker)
    return pd.DataFrame(rows)


def main():
    spk = speakers()
    matched = match(spk)
    matched["y"] = (matched.gender == "female").astype(int)

    nM = int((matched.gender == "male").sum())
    nF = int((matched.gender == "female").sum())
    mp, fp = matched[matched.gender == "male"].pitch, matched[matched.gender == "female"].pitch

    # confirm pitch is now uninformative
    cv = StratifiedKFold(5, shuffle=True, random_state=0)
    proba = cross_val_predict(make_pipeline(StandardScaler(), LogisticRegression(max_iter=1000)),
                              matched[["pitch"]].values, matched.y.values, cv=cv,
                              method="predict_proba")[:, 1]
    pit_acc = ((proba >= 0.5).astype(int) == matched.y.values).mean()
    pit_auc = roc_auc_score(matched.y.values, proba)

    print(f"Full speaker pool (after outlier clean): {len(spk)} "
          f"({(spk.gender=='male').sum()} M / {(spk.gender=='female').sum()} F)")
    print(f"\nPitch-matched dataset: {len(matched)} speakers ({nM} M / {nF} F)")
    print(f"  pitch {matched.pitch.min():.0f}-{matched.pitch.max():.0f} Hz | "
          f"mean M={mp.mean():.0f}  F={fp.mean():.0f}  (gap {abs(mp.mean()-fp.mean()):.1f} Hz)")
    print(f"  speakers with audio kept on disk: {int(matched.kept.sum())}")
    print(f"  pitch-only classifier: acc={pit_acc:.0%}  auc={pit_auc:.2f}   <- ~chance = matched ✓")

    out = os.path.join(HERE, "pitch_matched_full.csv")
    matched[["speaker", "gender", "pitch", "n", "kept"]].to_csv(out, index=False)
    print(f"\nWrote {out}")


if __name__ == "__main__":
    main()
