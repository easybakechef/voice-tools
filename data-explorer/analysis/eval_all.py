"""Do vowel-space-dispersion and sibilant features improve the CV gender AUC?

Test A: vowel-space features on the full rich pool (no sibilant dependency).
Test B: everything together on speakers that also have sibilant features.

Run:  .venv/bin/python -m analysis.eval_all
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
VOWEL = ["vsa", "f1_disp", "f2_disp"]
VHULL = ["vsa_hull", "vsa_hull_norm"]
DYN = ["f0_range", "traj_f12", "spec_rate", "f2_range"]
SIB = ["sib_m1", "sib_m2", "sib_m3", "sib_m4", "sib_hi", "sib_peak"]
RESON = ["f1", "f2", "f3", "f4", "f5", "vtl"] + LPCC + SPEC
TOL = 6.0


def base(df):
    df = df[df.gender.isin(["male", "female"])].dropna(subset=["f0"] + RESON + VOWEL + VHULL + DYN)
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


def report(df, sets, title):
    matched = pitch_match(df)
    matched["y"] = (matched.gender == "female").astype(int)
    print(f"\n=== {title} ===")
    print(f"pool={len(df)} ({(df.gender=='male').sum()}M/{(df.gender=='female').sum()}F)  matched={len(matched)}")
    print(f"{'feature set':30} {'matched: acc  AUC':>20} {'full: acc  AUC':>18}")
    for name, feats in sets.items():
        ma, mu = cv(matched, feats)
        fa, fu = cv(df, feats)
        print(f"{name:30}   {ma:>6.1%} {mu:>6.3f}     {fa:>6.1%} {fu:>6.3f}")


def main():
    rich = pd.read_csv(os.path.join(HERE, "rich_features.csv"))
    dfA = base(rich)
    report(dfA, {
        "resonance (rich)": RESON,
        "dynamic only (VISC/movement)": DYN,
        "resonance + dynamic": RESON + DYN,
        "resonance + vowel-hull": RESON + VHULL,
        "resonance + dynamic + vowel": RESON + DYN + VHULL + VOWEL,
        "combo (F0+rich)": ["f0"] + RESON,
        "combo + dynamic": ["f0"] + RESON + DYN,
    }, "Test A — dynamic + vowel cues, full pool")

    sib = pd.read_csv(os.path.join(HERE, "sibilant_features.csv"))
    dfB = base(rich.merge(sib, on="speaker", how="inner")).dropna(subset=SIB)
    report(dfB, {
        "resonance (rich)": RESON,
        "resonance + sibilant": RESON + SIB,
        "resonance + dynamic": RESON + DYN,
        "resonance + sibilant + dynamic": RESON + SIB + DYN,
        "combo + sibilant + dynamic": ["f0"] + RESON + SIB + DYN,
    }, "Test B — do sibilant + dynamic stack? (speakers with sibilant too)")


if __name__ == "__main__":
    main()
