"""Resonance metric from formants (vocal-tract-length based), evaluated on the
pitch-matched ("vocally ambiguous") set where pitch is uninformative.

Metrics compared (all pitch-free):
  - F2 alone                     (trans-voice literature: strongest single formant)
  - mean formant (F1-F4)
  - formant dispersion ΔF        (Fitch)
  - VTL via regression           (Anikin/Reby: fit Fn to (2n-1)c/4L)
  - all formants F1-F4           (learned logistic)
vs pitch (F0) — which should collapse to chance on the matched set.

Run:  .venv/bin/python -m analysis.resonance_metric
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
FEATURES = os.path.join(HERE, "kept_features.csv")
C_CM = 35000.0          # speed of sound, cm/s (Fitch convention)
TOL = 6.0               # Hz, pitch-matching tolerance

# ── the resonance metric ─────────────────────────────────────────────────────
def vtl_cm(row) -> float:
    """Vocal tract length (cm) from F1-F4: least-squares fit of Fn to the
    quarter-wavelength pattern Fn = (2n-1)·c/(4L) through the origin."""
    F = np.array([row.f1, row.f2, row.f3, row.f4], float)
    ns = np.array([1, 3, 5, 7], float)
    slope = np.sum(ns * F) / np.sum(ns * ns)   # = c/(4L)
    return C_CM / (4.0 * slope)


def load() -> pd.DataFrame:
    df = pd.read_csv(FEATURES)
    df = df[df.gender.isin(["male", "female"])].dropna(subset=["f0", "f1", "f2", "f3", "f4"])
    # plausibility: monotonic formants + sane ranges (drop bad LPC tracks / octave errors)
    df = df[(df.f1.between(200, 1100)) & (df.f2 > df.f1) & (df.f3 > df.f2) & (df.f4 > df.f3)
            & (df.f0.between(80, 320))]
    df = df.copy()
    df["vtl"] = df.apply(vtl_cm, axis=1)
    df["delta_f"] = df[["f1", "f2", "f3", "f4"]].apply(
        lambda r: np.mean(np.diff(r.values)), axis=1)
    df["mean_formant"] = df[["f1", "f2", "f3", "f4"]].mean(axis=1)
    df["y"] = (df.gender == "female").astype(int)
    return df[df.vtl.between(8, 22)]   # plausible adult VTL range, cm


def pitch_match(df: pd.DataFrame) -> pd.DataFrame:
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


def cv_eval(df, feats):
    X, y = df[feats].values, df.y.values
    cv = StratifiedKFold(5, shuffle=True, random_state=0)
    proba = cross_val_predict(make_pipeline(StandardScaler(), LogisticRegression(max_iter=1000)),
                              X, y, cv=cv, method="predict_proba")[:, 1]
    return float(((proba >= 0.5) == y).mean()), float(roc_auc_score(y, proba))


FEATURE_SETS = {
    "Pitch (F0)  [baseline]": ["f0"],
    "F2 only": ["f2"],
    "mean formant F1-F4": ["mean_formant"],
    "formant dispersion ΔF": ["delta_f"],
    "VTL (regression)": ["vtl"],
    "all formants F1-F4": ["f1", "f2", "f3", "f4"],
}


def main():
    df = load()
    matched = pitch_match(df)
    matched["y"] = (matched.gender == "female").astype(int)

    nM = int((matched.gender == "male").sum())
    nF = int((matched.gender == "female").sum())
    mp, fp = matched[matched.gender == "male"], matched[matched.gender == "female"]
    print(f"Kept pool with formants: {len(df)} speakers "
          f"({(df.gender=='male').sum()} M / {(df.gender=='female').sum()} F)")
    print(f"Pitch-matched set: {len(matched)} ({nM} M / {nF} F), "
          f"mean F0 M={mp.f0.mean():.0f} F={fp.f0.mean():.0f} Hz")
    print(f"VTL by gender (cm): male={mp.vtl.mean():.1f}  female={fp.vtl.mean():.1f}\n")

    print(f"{'metric':28} {'matched acc':>12} {'matched AUC':>12} {'full-pool AUC':>14}")
    for name, feats in FEATURE_SETS.items():
        m_acc, m_auc = cv_eval(matched, feats)
        _, full_auc = cv_eval(df, feats)
        print(f"{name:28} {m_acc:>11.0%} {m_auc:>12.2f} {full_auc:>14.2f}")


if __name__ == "__main__":
    main()
