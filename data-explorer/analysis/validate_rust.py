"""Validate the Rust formant/VTL extraction against the parselmouth baseline.

1. Per-speaker agreement: correlation + mean abs error of F1-F4 and VTL between
   analysis/kept_features.csv (parselmouth) and analysis/rust_kept_features.csv (Rust).
2. Metric parity: rerun the full resonance_metric evaluation on the Rust features
   and confirm the pitch-matched / full-pool AUC still lands near 0.85.

Run:  .venv/bin/python -m analysis.validate_rust
"""
import os
import numpy as np
import pandas as pd

from . import resonance_metric as rm

HERE = os.path.dirname(__file__)
PY = os.path.join(HERE, "kept_features.csv")
RS = os.path.join(HERE, "rust_kept_features.csv")


def _prep(path):
    df = pd.read_csv(path)
    df = df[df.gender.isin(["male", "female"])].dropna(subset=["f0", "f1", "f2", "f3", "f4"])
    df = df[(df.f1.between(200, 1100)) & (df.f2 > df.f1) & (df.f3 > df.f2) & (df.f4 > df.f3)
            & (df.f0.between(80, 320))].copy()
    df["vtl"] = df.apply(rm.vtl_cm, axis=1)
    return df[df.vtl.between(8, 22)].set_index("speaker")


def agreement():
    py, rs = _prep(PY), _prep(RS)
    common = py.index.intersection(rs.index)
    py, rs = py.loc[common], rs.loc[common]
    print(f"Per-speaker agreement on {len(common)} speakers shared by both extractors:\n")
    print(f"{'feature':10} {'corr':>7} {'MAE':>9} {'py mean':>9} {'rust mean':>10}")
    for col in ["f0", "f1", "f2", "f3", "f4", "vtl"]:
        a, b = py[col].values, rs[col].values
        corr = np.corrcoef(a, b)[0, 1]
        mae = np.mean(np.abs(a - b))
        print(f"{col:10} {corr:>7.3f} {mae:>9.1f} {a.mean():>9.1f} {b.mean():>10.1f}")
    print()


def metric_on(path, label):
    orig = rm.FEATURES
    try:
        rm.FEATURES = path
        df = rm.load()
        matched = rm.pitch_match(df)
        matched["y"] = (matched.gender == "female").astype(int)
        mp, fp = matched[matched.gender == "male"], matched[matched.gender == "female"]
        print(f"[{label}]  pool={len(df)}  matched={len(matched)} "
              f"({(matched.gender=='male').sum()}M/{(matched.gender=='female').sum()}F)  "
              f"VTL male={mp.vtl.mean():.1f} female={fp.vtl.mean():.1f} cm")
        print(f"{'  metric':28} {'matched acc':>12} {'matched AUC':>12} {'full AUC':>10}")
        for name, feats in rm.FEATURE_SETS.items():
            m_acc, m_auc = rm.cv_eval(matched, feats)
            _, full_auc = rm.cv_eval(df, feats)
            print(f"  {name:26} {m_acc:>11.0%} {m_auc:>12.2f} {full_auc:>10.2f}")
        print()
    finally:
        rm.FEATURES = orig


if __name__ == "__main__":
    agreement()
    metric_on(PY, "parselmouth baseline")
    metric_on(RS, "Rust")
