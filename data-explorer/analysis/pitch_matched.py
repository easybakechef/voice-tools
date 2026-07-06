"""Build a PITCH-MATCHED subset: pair each man with a woman of nearly identical
pitch (greedy nearest-F0, max gap TOL). In the resulting set the F0 distributions
are the same for both genders, so a pitch-only classifier is forced to chance —
and resonance-only accuracy on it is a clean read of resonance's independent
signal.

Run:  .venv/bin/python -m analysis.pitch_matched
Out:  analysis/pitch_matched.csv  + printed pitch-vs-resonance comparison
"""
import os

import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_auc_score
from sklearn.model_selection import LeaveOneOut, cross_val_predict
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler

from analysis.report import load_speakers

HERE = os.path.dirname(__file__)
TOL = 12.0  # Hz: max pitch gap allowed within a matched pair


def loo(X, y):
    proba = cross_val_predict(make_pipeline(StandardScaler(), LogisticRegression(max_iter=1000)),
                              X, y, cv=LeaveOneOut(), method="predict_proba")[:, 1]
    acc = ((proba >= 0.5).astype(int) == y).mean()
    auc = roc_auc_score(y, proba) if len(np.unique(y)) == 2 else float("nan")
    return acc, auc


def build_matched(spk: pd.DataFrame) -> pd.DataFrame:
    men = spk[spk.gender == "male"].sort_values("f0").to_dict("records")
    women = spk[spk.gender == "female"].sort_values("f0").copy()
    used, pairs = set(), []
    for m in men:
        avail = women[~women.speaker.isin(used)]
        if avail.empty:
            break
        j = (avail.f0 - m["f0"]).abs().idxmin()
        w = women.loc[j]
        if abs(w.f0 - m["f0"]) <= TOL:
            pairs.append(m)
            pairs.append(w.to_dict())
            used.add(w.speaker)
    return pd.DataFrame(pairs)


def main():
    spk = load_speakers()
    matched = build_matched(spk)
    if matched.empty:
        print("No pitch-matched pairs within tolerance.")
        return

    nM = int((matched.gender == "male").sum())
    nF = int((matched.gender == "female").sum())
    pit_acc, pit_auc = loo(matched[["f0"]].values, matched.y.values)
    res_acc, res_auc = loo(matched[["f1", "f2"]].values, matched.y.values)
    both_acc, both_auc = loo(matched[["f0", "f1", "f2"]].values, matched.y.values)

    # how matched is the pitch? (should be ~0 difference)
    mp = matched[matched.gender == "male"].f0
    fp = matched[matched.gender == "female"].f0
    print(f"\nPitch-matched subset: {len(matched)} speakers ({nM} M / {nF} F), "
          f"F0 {matched.f0.min():.0f}-{matched.f0.max():.0f} Hz")
    print(f"  mean F0  male={mp.mean():.0f}  female={fp.mean():.0f}  (gap {abs(mp.mean()-fp.mean()):.0f} Hz)")
    print(f"\n  classifier (leave-one-out):")
    print(f"    pitch only (F0)     acc={pit_acc:.0%}  auc={pit_auc:.2f}   <- should be ~chance")
    print(f"    resonance (F1,F2)   acc={res_acc:.0%}  auc={res_auc:.2f}")
    print(f"    pitch + resonance   acc={both_acc:.0%}  auc={both_auc:.2f}")

    out = os.path.join(HERE, "pitch_matched.csv")
    matched[["dataset", "speaker", "gender", "f0", "f1", "f2"]].to_csv(out, index=False)
    print(f"\nWrote {out}")


if __name__ == "__main__":
    main()
