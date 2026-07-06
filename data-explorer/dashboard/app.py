"""Voice Data Explorer — Streamlit dashboard over the ingested sample metrics."""
import os
import sqlite3

import pandas as pd
import plotly.express as px
import streamlit as st

DB_PATH = os.path.join(os.path.dirname(__file__), "..", "data", "samples.db")
GENDER_COLORS = {"female": "#F5A9B8", "male": "#5BCEFA", "nonbinary": "#b57edc"}


@st.cache_data(ttl=5)
def load() -> pd.DataFrame:
    if not os.path.exists(DB_PATH):
        return pd.DataFrame()
    con = sqlite3.connect(DB_PATH)
    df = pd.read_sql_query("select * from samples", con)
    con.close()
    return df


def speaker_frame(df: pd.DataFrame) -> pd.DataFrame:
    return (df.groupby(["dataset", "speaker", "gender"], dropna=False)
              .agg(duration=("duration", "sum"),
                   pitch=("f0_median", "median"),
                   f1=("f1_median", "median"),
                   f2=("f2_median", "median"),
                   n=("id", "count"))
              .reset_index())


st.set_page_config(page_title="Voice Data Explorer", layout="wide")
st.title("🎙️ Voice Data Explorer")

df = load()
if df.empty:
    st.info("No data yet. From `data-explorer/`, run e.g.:\n\n"
            "```\npython -m pipeline.ingest librispeech --subset dev-clean\n```")
    st.stop()

# ── Sidebar filters ─────────────────────────────────────────────────────────
st.sidebar.header("Filters")
all_ds = sorted(df["dataset"].unique())
sel_ds = st.sidebar.multiselect("Dataset", all_ds, default=all_ds)
all_g = sorted(g for g in df["gender"].dropna().unique())
sel_g = st.sidebar.multiselect("Gender", all_g, default=all_g)
valid_pitch = df["f0_median"].dropna()
p_lo, p_hi = int(valid_pitch.min()), int(valid_pitch.max())
prange = st.sidebar.slider("Pitch range (Hz)", p_lo, p_hi, (p_lo, p_hi))

fdf = df[df["dataset"].isin(sel_ds) & df["gender"].isin(sel_g)
         & df["f0_median"].between(*prange)]
spk = speaker_frame(fdf)

if fdf.empty:
    st.warning("No samples match the current filters.")
    st.stop()

# ── Overview metrics (per gender) ───────────────────────────────────────────
st.subheader("Overview")
present = sorted(fdf["gender"].dropna().unique())
cols = st.columns(max(1, len(present)))
for col, g in zip(cols, present):
    sub = fdf[fdf["gender"] == g]
    subspk = spk[spk["gender"] == g]
    avg = subspk["pitch"].mean()
    with col:
        st.markdown(f"#### {g.title()}")
        st.metric("Minutes of audio", f"{sub['duration'].sum() / 60:.1f}")
        st.metric("Samples", f"{len(sub):,}")
        st.metric("Speakers", f"{subspk['speaker'].nunique():,}")
        st.metric("Avg pitch (per speaker)", f"{avg:.0f} Hz" if pd.notna(avg) else "—")

# ── Pitch distributions ─────────────────────────────────────────────────────
st.subheader("Average pitch per speaker — distribution")
c1, c2 = st.columns(2)
with c1:
    st.caption("Isolated per gender")
    fig = px.histogram(spk, x="pitch", color="gender", facet_row="gender",
                       nbins=30, color_discrete_map=GENDER_COLORS)
    fig.update_layout(showlegend=False, height=110 * max(1, len(present)) + 80,
                      margin=dict(t=20, b=10))
    fig.for_each_annotation(lambda a: a.update(text=a.text.split("=")[-1]))
    st.plotly_chart(fig, use_container_width=True)
with c2:
    st.caption("Overlaid")
    fig2 = px.histogram(spk, x="pitch", color="gender", barmode="overlay",
                        nbins=30, opacity=0.6, color_discrete_map=GENDER_COLORS)
    fig2.update_layout(height=110 * max(1, len(present)) + 80, margin=dict(t=20, b=10))
    st.plotly_chart(fig2, use_container_width=True)

# ── Formant scatter (preview of the resonance signal) ───────────────────────
st.subheader("Formant space (F1 vs F2) — does resonance separate gender?")
sc = spk.dropna(subset=["f1", "f2"])
if not sc.empty:
    fig3 = px.scatter(sc, x="f2", y="f1", color="gender", hover_data=["speaker", "pitch"],
                      color_discrete_map=GENDER_COLORS)
    fig3.update_xaxes(autorange="reversed", title="F2 (Hz)")
    fig3.update_yaxes(autorange="reversed", title="F1 (Hz)")
    st.plotly_chart(fig3, use_container_width=True)

# ── Per-dataset drill-down ──────────────────────────────────────────────────
st.subheader("Datasets")
for ds in sel_ds:
    dsub = fdf[fdf["dataset"] == ds]
    if dsub.empty:
        continue
    with st.expander(f"{ds} — {len(dsub):,} samples, "
                     f"{dsub['speaker'].nunique()} speakers, "
                     f"{dsub['duration'].sum() / 60:.0f} min", expanded=len(sel_ds) == 1):
        st.dataframe(
            dsub[["speaker", "gender", "duration", "f0_median", "f1_median", "f2_median"]]
            .sort_values("speaker"),
            use_container_width=True, height=260,
        )
        pick = st.selectbox("Preview a sample", dsub["path"].tolist(),
                            key=f"pick-{ds}", format_func=lambda p: os.path.basename(p))
        if pick and os.path.exists(pick):
            ext = os.path.splitext(pick)[1].lstrip(".") or "wav"
            st.audio(open(pick, "rb").read(), format=f"audio/{ext}")
