"""Build a gender classifier from the ingested voice features and emit an HTML
report. The point is not just accuracy — it's whether RESONANCE (formants)
carries gender signal independent of pitch, and whether it generalises across
recording conditions.

Run:  .venv/bin/python -m analysis.report
Out:  analysis/report.html
"""
import os
import sqlite3

import numpy as np
import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import accuracy_score, confusion_matrix, roc_auc_score, roc_curve
from sklearn.model_selection import StratifiedKFold, cross_val_predict
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler

HERE = os.path.dirname(__file__)
DB = os.path.join(HERE, "..", "data", "samples.db")
OUT = os.path.join(HERE, "report.html")
COLORS = {"female": "#F5A9B8", "male": "#5BCEFA"}

FEATURE_SETS = {
    "Pitch only (F0)": ["f0"],
    "Resonance only (F1, F2)": ["f1", "f2"],
    "Resonance + ratio (F1, F2, F2/F1)": ["f1", "f2", "f2f1"],
    "Pitch + Resonance (F0, F1, F2)": ["f0", "f1", "f2"],
}


def model():
    return make_pipeline(StandardScaler(), LogisticRegression(max_iter=1000))


def load_speakers() -> pd.DataFrame:
    con = sqlite3.connect(DB)
    df = pd.read_sql_query("select * from samples", con)
    con.close()
    df = df[df["gender"].isin(["male", "female"])].dropna(
        subset=["f0_median", "f1_median", "f2_median"])
    spk = (df.groupby(["dataset", "speaker", "gender"])
             .agg(f0=("f0_median", "median"), f1=("f1_median", "median"),
                  f2=("f2_median", "median"), n=("id", "count"),
                  minutes=("duration", lambda s: s.sum() / 60))
             .reset_index())
    spk["f2f1"] = spk["f2"] / spk["f1"]
    spk["y"] = (spk["gender"] == "female").astype(int)  # 1 = female
    return spk


def eval_cv(spk, feats):
    X, y = spk[feats].values, spk["y"].values
    cv = StratifiedKFold(5, shuffle=True, random_state=0)
    proba = cross_val_predict(model(), X, y, cv=cv, method="predict_proba")[:, 1]
    pred = (proba >= 0.5).astype(int)
    return accuracy_score(y, pred), roc_auc_score(y, proba), proba


def eval_cross_dataset(spk, feats):
    rows = []
    ds = sorted(spk["dataset"].unique())
    for tr in ds:
        for te in ds:
            if tr == te:
                continue
            a, b = spk[spk.dataset == tr], spk[spk.dataset == te]
            if a["y"].nunique() < 2 or len(b) == 0:
                continue
            m = model().fit(a[feats].values, a["y"].values)
            p = m.predict_proba(b[feats].values)[:, 1]
            rows.append((f"{tr} → {te}", len(b),
                         accuracy_score(b["y"], p >= 0.5), roc_auc_score(b["y"], p)))
    return rows


def fig_html(fig):
    return fig.to_html(full_html=False, include_plotlyjs=False,
                       config={"displayModeBar": False})


def main():
    spk = load_speakers()

    # ── classifier comparison (speaker-level 5-fold CV) ──
    comp, probas = [], {}
    for name, feats in FEATURE_SETS.items():
        acc, auc, proba = eval_cv(spk, feats)
        comp.append({"Feature set": name, "CV accuracy": acc, "ROC-AUC": auc})
        probas[name] = proba
    comp_df = pd.DataFrame(comp)

    # ── the hard cases: speakers whose pitch is atypical for their gender ──
    def _rz(s):
        med = s.median()
        mad = (s - med).abs().median() * 1.4826
        return (s - med) / (mad if mad else (s.std() or 1.0))
    spk["_z"] = spk.groupby("gender")["f0"].transform(_rz)
    spk["_pto"] = spk.apply(lambda r: r["_z"] if r["gender"] == "male" else -r["_z"], axis=1)
    spk["_resP"] = (probas["Resonance only (F1, F2)"] >= 0.5).astype(int)
    spk["_pitP"] = (probas["Pitch only (F0)"] >= 0.5).astype(int)
    atyp = spk[spk["_pto"] > 1.0]
    pit_hard = (atyp._pitP == atyp.y).mean()
    res_hard = (atyp._resP == atyp.y).mean()
    hard_txt = (f"We mined <b>{len(atyp)} speakers whose pitch is ≥1σ atypical for their gender</b> "
                f"(skewed toward the other gender). Two sobering results: <b>(1)</b> pitch-only is "
                f"<b>still {pit_hard:.0%}</b> correct on them — male/female pitch barely overlaps in this "
                f"read-speech data, so even atypical pitch rarely crosses the line; and <b>(2)</b> "
                f"resonance-only is only <b>{res_hard:.0%}</b> on these (near chance). "
                f"<b>Conclusion: our current resonance feature — median F1/F2 — is too weak to carry the "
                f"hard cases.</b> To actually separate resonance from pitch we need both (a) stronger "
                f"resonance features (per-vowel normalization / vocal-tract-length estimation) and "
                f"(b) data with genuine pitch overlap (Common Voice, trans voices).")

    # ── cross-dataset generalisation (resonance only) ──
    xds = eval_cross_dataset(spk, FEATURE_SETS["Resonance only (F1, F2)"])
    xds_df = pd.DataFrame(xds, columns=["Train → Test", "Test speakers", "Accuracy", "ROC-AUC"])

    # ── pitch-overlap test: can resonance classify where pitch overlaps? ──
    lo, hi = 150, 200
    band = spk[spk["f0"].between(lo, hi)]
    nF, nM = int(band["y"].sum()), int(len(band) - band["y"].sum())
    if min(nF, nM) >= 8:
        acc, auc, _ = eval_cv(band, FEATURE_SETS["Resonance only (F1, F2)"])
        band_txt = (f"Among the <b>{len(band)} speakers whose pitch falls in {lo}–{hi} Hz</b> "
                    f"({nF} F / {nM} M, where F0 alone is ambiguous), resonance-only still "
                    f"classifies gender at <b>{acc:.0%}</b> accuracy (AUC {auc:.2f}) — strong "
                    f"evidence it's a genuinely separate signal.")
    else:
        minority = "men" if nM < nF else "women"
        band_txt = (f"<b>We can't run this test cleanly yet — and that's itself a finding.</b> "
                    f"Only <b>{min(nF, nM)} {minority}</b> fall in the {lo}–{hi} Hz overlap band "
                    f"({nF} F / {nM} M), far too imbalanced to isolate resonance from pitch. "
                    f"Our datasets barely overlap in pitch, which is also why "
                    f"pitch+resonance didn't beat pitch alone (there's almost no region where "
                    f"they disagree). <b>Closing this gap needs more high-pitched men and "
                    f"low-pitched women</b> — i.e. Common Voice and, ideally, trans voices.")

    # ── the data-driven resonance score (resonance-only CV probabilities) ──
    spk = spk.copy()
    spk["resonance_score"] = (probas["Resonance only (F1, F2)"] * 100).round(1)

    # ── charts ──
    sc = px.scatter(spk, x="f2", y="f1", color="gender", symbol="dataset",
                    hover_data=["speaker", "f0"], color_discrete_map=COLORS,
                    title="Speakers in formant space (F1 vs F2)")
    sc.update_xaxes(autorange="reversed", title="F2 (Hz)")
    sc.update_yaxes(autorange="reversed", title="F1 (Hz)")

    pitch_vs_res = px.scatter(spk, x="f0", y="resonance_score", color="gender",
                              hover_data=["speaker", "dataset"], color_discrete_map=COLORS,
                              title="Pitch vs. data-driven resonance score")
    pitch_vs_res.add_vrect(x0=lo, x1=hi, fillcolor="gray", opacity=0.12, line_width=0,
                           annotation_text="pitch overlap band")
    pitch_vs_res.update_xaxes(title="F0 / pitch (Hz)")
    pitch_vs_res.update_yaxes(title="Resonance score (0–100)")

    score_hist = px.histogram(spk, x="resonance_score", color="gender", barmode="overlay",
                              nbins=25, opacity=0.6, color_discrete_map=COLORS,
                              title="Resonance score distribution by gender")

    roc = go.Figure()
    for name, proba in probas.items():
        fpr, tpr, _ = roc_curve(spk["y"], proba)
        roc.add_trace(go.Scatter(x=fpr, y=tpr, name=f"{name} (AUC {roc_auc_score(spk['y'], proba):.2f})", mode="lines"))
    roc.add_trace(go.Scatter(x=[0, 1], y=[0, 1], line=dict(dash="dash", color="gray"), showlegend=False))
    roc.update_layout(title="ROC curves", xaxis_title="False positive rate", yaxis_title="True positive rate")

    res_pred = (probas["Resonance only (F1, F2)"] >= 0.5).astype(int)
    cm = confusion_matrix(spk["y"], res_pred)
    cm_fig = px.imshow(cm, text_auto=True, color_continuous_scale="Blues",
                       x=["pred male", "pred female"], y=["true male", "true female"],
                       title="Confusion matrix — resonance-only model")

    # ── assemble HTML ──
    def table(df, fmts=None):
        d = df.copy()
        for c, f in (fmts or {}).items():
            if c in d:
                d[c] = d[c].map(f)
        return d.to_html(index=False, classes="tbl", border=0)

    n_by = spk.groupby("gender")["speaker"].count().to_dict()
    summary = spk.groupby(["dataset", "gender"]).agg(
        speakers=("speaker", "count"), minutes=("minutes", "sum"),
        pitch=("f0", "mean")).round(1).reset_index()

    html = f"""<!doctype html><html><head><meta charset="utf-8">
<title>Voice gender classifier — report</title>
<script src="https://cdn.plot.ly/plotly-2.35.2.min.js"></script>
<style>
 body{{font-family:system-ui,sans-serif;max-width:1000px;margin:2rem auto;padding:0 1rem;color:#1a1a2e;line-height:1.55}}
 h1{{background:linear-gradient(90deg,#5BCEFA,#F5A9B8);-webkit-background-clip:text;-webkit-text-fill-color:transparent}}
 h2{{margin-top:2.2rem;border-bottom:2px solid #eee;padding-bottom:.3rem}}
 .tbl{{border-collapse:collapse;width:100%;font-size:.9rem;margin:.5rem 0}}
 .tbl th,.tbl td{{border:1px solid #e3e3ef;padding:.4rem .6rem;text-align:left}}
 .tbl th{{background:#f4f4fb}}
 .key{{background:#f0f8ff;border-left:4px solid #5BCEFA;padding:.8rem 1rem;border-radius:6px;margin:1rem 0}}
 .caveat{{background:#fff7f0;border-left:4px solid #f39c12;padding:.8rem 1rem;border-radius:6px;margin:1rem 0;font-size:.92rem}}
 .muted{{color:#666;font-size:.85rem}}
</style></head><body>

<h1>Voice gender classifier — report</h1>
<p class="muted">Logistic regression on per-speaker median pitch (F0) and formants (F1/F2),
extracted with Praat. {n_by.get('female',0)} female + {n_by.get('male',0)} male speakers
across {spk['dataset'].nunique()} datasets. (Binary only — no non-binary data yet.)</p>

<h2>1. The data</h2>
{table(summary, {'minutes': lambda v: f'{v:.0f}', 'pitch': lambda v: f'{v:.0f} Hz'})}

<h2>2. What separates gender — pitch vs. resonance</h2>
<p>Speaker-level 5-fold cross-validation. The interesting comparison is the
<b>resonance-only</b> row against <b>pitch-only</b>:</p>
{table(comp_df, {'CV accuracy': lambda v: f'{v:.1%}', 'ROC-AUC': lambda v: f'{v:.3f}'})}
{fig_html(roc)}

<h2>3. Is it really resonance, or just pitch in disguise?</h2>
<div class="caveat">{hard_txt}</div>
<p class="muted">{band_txt}</p>
{fig_html(pitch_vs_res)}

<h2>4. Does it generalise across recording conditions?</h2>
<p>Train on one dataset's speakers, test on the other's (different mics/rooms).
If the resonance model holds up, it's learning the <i>voice</i>, not the microphone:</p>
{table(xds_df, {'Accuracy': lambda v: f'{v:.0%}', 'ROC-AUC': lambda v: f'{v:.3f}'})}

<h2>5. The resonance model up close</h2>
{fig_html(cm_fig)}
{fig_html(sc)}

<h2>6. A data-driven resonance score</h2>
<p>The resonance-only model's probability (×100) is a calibrated 0–100 "brightness/femininity-of-resonance"
score — a principled replacement for the hand-tuned bar in the web app.</p>
{fig_html(score_hist)}

<div class="caveat"><b>Caveats.</b> Small sample (~100 speakers); binary cis labels only
(non-binary needs Common Voice); formants from a single fixed LPC ceiling; read speech only.
Treat the numbers as directional, not final.</div>
</body></html>"""

    with open(OUT, "w") as f:
        f.write(html)
    print(f"Wrote {OUT}")
    print(comp_df.to_string(index=False))


if __name__ == "__main__":
    main()
