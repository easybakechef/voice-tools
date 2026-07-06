"""Evaluate richer resonance representations for gender separation.

Compares pitch, VTL, F1-F5, LPC-cepstrum, spectral source/shape features, and
combinations — on the pitch-matched set (pitch neutralized, the honest test of a
pitch-independent resonance measure) and on the full pool. Logistic (linear) and
gradient boosting (nonlinear ceiling).

Run:  .venv/bin/python -m analysis.eval_rich
"""
import numpy as np
import pandas as pd
from sklearn.ensemble import GradientBoostingClassifier
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_auc_score
from sklearn.model_selection import StratifiedKFold, cross_val_predict
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler

import os
HERE = os.path.dirname(__file__)
RICH = os.path.join(HERE, "rich_features.csv")
TOL = 6.0
LPCC = [f"c{k}" for k in range(1, 13)]
SPEC = ["centroid", "tilt", "rolloff", "h1h2"]


def load():
    df = pd.read_csv(RICH)
    df = df[df.gender.isin(["male", "female"])].dropna(
        subset=["f0", "f1", "f2", "f3", "f4", "f5", "vtl"] + LPCC + SPEC)
    df = df[(df.f1.between(200, 1100)) & (df.f2 > df.f1) & (df.f3 > df.f2) & (df.f4 > df.f3)
            & (df.f0.between(80, 320)) & (df.vtl.between(8, 22))].copy()
    df["y"] = (df.gender == "female").astype(int)
    return df


def pitch_match(df):
    men = df[df.gender == "male"].sort_values("f0").to_dict("records")
    women = df[df.gender == "female"].sort_values("f0").copy()
    used, rows = set(), []
    for m in men:
        avail = women[~women.speaker.isin(used)]
        if avail.empty:
            break
        j = (avail.f0 - m["f0"]).abs().idxmin()
        w = women.loc[j]
        if abs(w.f0 - m["f0"]) <= TOL:
            rows.append(m); rows.append(w.to_dict()); used.add(w.speaker)
    return pd.DataFrame(rows)


def cv(df, feats, clf):
    X, y = df[feats].values, df.y.values
    cvk = StratifiedKFold(5, shuffle=True, random_state=0)
    proba = cross_val_predict(clf(), X, y, cv=cvk, method="predict_proba")[:, 1]
    acc = float(((proba >= 0.5) == y).mean())
    return acc, float(roc_auc_score(y, proba))


def lin():
    return make_pipeline(StandardScaler(), LogisticRegression(max_iter=2000))


def gb():
    return GradientBoostingClassifier(random_state=0)


SETS = {
    "pitch F0": ["f0"],
    "VTL (current)": ["vtl"],
    "F1-F4": ["f1", "f2", "f3", "f4"],
    "F1-F5": ["f1", "f2", "f3", "f4", "f5"],
    "LPC-cepstrum (c1-12)": LPCC,
    "spectral (cen/tilt/roll/h1h2)": SPEC,
    "F1-F5 + LPCC": ["f1", "f2", "f3", "f4", "f5"] + LPCC,
    "LPCC + spectral": LPCC + SPEC,
    "resonance-all (no F0)": ["f1", "f2", "f3", "f4", "f5", "vtl"] + LPCC + SPEC,
    "combo: F0 + resonance-all": ["f0", "f1", "f2", "f3", "f4", "f5", "vtl"] + LPCC + SPEC,
}


def main():
    df = load()
    matched = pitch_match(df)
    matched["y"] = (matched.gender == "female").astype(int)
    print(f"pool={len(df)} ({(df.gender=='male').sum()}M/{(df.gender=='female').sum()}F)  "
          f"matched={len(matched)}\n")
    print(f"{'feature set':32} {'  linear: mAUC fAUC':>22} {'  boosted: mAUC fAUC':>22}")
    for name, feats in SETS.items():
        _, l_m = cv(matched, feats, lin)
        _, l_f = cv(df, feats, lin)
        _, g_m = cv(matched, feats, gb)
        _, g_f = cv(df, feats, gb)
        print(f"{name:32}   {l_m:>6.3f} {l_f:>6.3f}        {g_m:>6.3f} {g_f:>6.3f}")


if __name__ == "__main__":
    main()
