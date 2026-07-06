"""Does adding sibilant features improve the cross-validated gender AUC?

Evaluates on the speakers that have BOTH rich features and sibilant features, so
the comparison (with vs without sibilant) is apples-to-apples.

Run:  .venv/bin/python -m analysis.eval_sibilant
"""
import os
import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_auc_score
from sklearn.model_selection import StratifiedKFold, cross_val_predict
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler

HERE = os.path.dirname(__file__)
LPCC = [f"c{k}" for k in range(1, 13)]
SPEC = ["centroid", "tilt", "rolloff", "h1h2"]
SIB = ["sib_m1", "sib_m2", "sib_m3", "sib_m4", "sib_hi", "sib_peak"]
RESON = ["f1", "f2", "f3", "f4", "f5", "vtl"] + LPCC + SPEC
TOL = 6.0


def load():
    df = pd.read_csv(os.path.join(HERE, "rich_features.csv"))
    sib = pd.read_csv(os.path.join(HERE, "sibilant_features.csv"))
    df = df.merge(sib, on="speaker", how="inner")   # only speakers with both
    df = df[df.gender.isin(["male", "female"])].dropna(
        subset=["f0"] + RESON + SIB)
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


def cv(df, feats):
    X, y = df[feats].values, df.y.values
    cvk = StratifiedKFold(5, shuffle=True, random_state=0)
    clf = make_pipeline(StandardScaler(), LogisticRegression(max_iter=2000))
    proba = cross_val_predict(clf, X, y, cv=cvk, method="predict_proba")[:, 1]
    return float(((proba >= 0.5) == y).mean()), float(roc_auc_score(y, proba))


SETS = {
    "sibilant only": SIB,
    "resonance (rich)": RESON,
    "resonance + sibilant": RESON + SIB,
    "combo (F0+rich)": ["f0"] + RESON,
    "combo + sibilant": ["f0"] + RESON + SIB,
}


def main():
    df = load()
    matched = pitch_match(df)
    matched["y"] = (matched.gender == "female").astype(int)
    print(f"speakers with both feature sets: pool={len(df)} "
          f"({(df.gender=='male').sum()}M/{(df.gender=='female').sum()}F)  matched={len(matched)}\n")
    print(f"{'feature set':26} {'matched: acc  AUC':>20} {'full: acc  AUC':>18}")
    for name, feats in SETS.items():
        m_acc, m_auc = cv(matched, feats)
        f_acc, f_auc = cv(df, feats)
        print(f"{name:26}   {m_acc:>6.1%} {m_auc:>6.3f}     {f_acc:>6.1%} {f_auc:>6.3f}")


if __name__ == "__main__":
    main()
