"""Sample-level explorer: search / filter the scanned samples, select one to play
it back, and see data on both the sample and its speaker."""
import os
import sqlite3

import pandas as pd
import plotly.express as px
import streamlit as st

DB_PATH = os.path.join(os.path.dirname(__file__), "..", "..", "data", "samples.db")
COLORS = {"female": "#F5A9B8", "male": "#5BCEFA", "nonbinary": "#b57edc"}


@st.cache_data(ttl=10)
def load() -> pd.DataFrame:
    if not os.path.exists(DB_PATH):
        return pd.DataFrame()
    con = sqlite3.connect(DB_PATH)
    df = pd.read_sql_query("select * from samples", con)
    con.close()
    return df


st.set_page_config(page_title="Sample Explorer", layout="wide")
st.title("🔎 Sample Explorer")

df = load()
if df.empty:
    st.info("No data yet — run an ingest/scan first.")
    st.stop()

df = df[df["gender"].notna()].copy()

# ── Filters ─────────────────────────────────────────────────────────────────
sb = st.sidebar
sb.header("Filters")
ds_opts = sorted(df["dataset"].unique())
g_opts = sorted(df["gender"].unique())
sel_ds = sb.multiselect("Dataset", ds_opts, default=ds_opts)
sel_g = sb.multiselect("Gender", g_opts, default=g_opts)
kept_only = sb.checkbox("Only samples with audio (kept)", value=True)

# Pitch filter: combine with the gender selection above for queries like
# "female  +  pitch < 170 Hz"  or  "male  +  pitch > 150 Hz".
valid = df["f0_median"].dropna()
p_lo, p_hi = int(valid.min()), int(valid.max())
pmode = sb.radio("Pitch filter", ["Any", "Greater than", "Less than", "Range"], index=0)
prange = (p_lo, p_hi)
pthr = 160
if pmode in ("Greater than", "Less than"):
    pthr = sb.number_input("Pitch threshold (Hz)", min_value=p_lo, max_value=p_hi, value=160, step=1)
elif pmode == "Range":
    prange = sb.slider("Pitch range (Hz)", p_lo, p_hi, (p_lo, p_hi))

search = sb.text_input("Search speaker id contains")

m = df["dataset"].isin(sel_ds) & df["gender"].isin(sel_g)
if pmode == "Greater than":
    m &= df["f0_median"] > pthr
elif pmode == "Less than":
    m &= df["f0_median"] < pthr
elif pmode == "Range":
    m &= df["f0_median"].between(*prange)
if kept_only:
    m &= df["kept"] == 1
if search.strip():
    m &= df["speaker"].str.contains(search.strip(), case=False, na=False)

fdf = df[m].sort_values(["speaker", "f0_median"]).reset_index(drop=True)
st.caption(f"**{len(fdf):,}** samples match · {fdf['speaker'].nunique()} speakers")

# ── Selectable table ────────────────────────────────────────────────────────
view = fdf.assign(
    pitch=fdf["f0_median"].round(0),
    dur=fdf["duration"].round(1),
    voiced=(fdf["voiced_frac"] * 100).round(0),
)[["speaker", "gender", "pitch", "dur", "voiced", "dataset", "kept"]]
view.columns = ["speaker", "gender", "pitch (Hz)", "dur (s)", "voiced %", "dataset", "audio"]

st.write("Select a row to inspect it:")
event = st.dataframe(view, use_container_width=True, height=380, hide_index=True,
                     on_select="rerun", selection_mode="single-row")

rows = event.selection.rows if event and event.selection else []
if not rows:
    st.info("⬆️ Click a row in the table to play it back and see speaker/sample details.")
    st.stop()

sample = fdf.iloc[rows[0]]
spk_rows = df[df["speaker"] == sample["speaker"]]

# ── Detail panel ────────────────────────────────────────────────────────────
left, right = st.columns([1, 1])

with left:
    st.subheader("Sample")
    st.markdown(
        f"- **speaker:** `{sample.speaker}`  ·  **gender:** {sample.gender}\n"
        f"- **pitch (F0):** {sample.f0_median:.0f} Hz\n"
        f"- **duration:** {sample.duration:.1f} s  ·  **voiced:** {sample.voiced_frac*100:.0f}%\n"
        f"- **formants:** "
        + (f"F1 {sample.f1_median:.0f} / F2 {sample.f2_median:.0f} Hz" if pd.notna(sample.f1_median) else "_(pitch-only scan)_")
        + f"\n- **dataset:** {sample.dataset}  ·  **audio kept:** {'yes' if sample.kept else 'no'}"
    )
    if sample.kept and os.path.exists(sample.path):
        ext = os.path.splitext(sample.path)[1].lstrip(".") or "wav"
        st.audio(open(sample.path, "rb").read(), format=f"audio/{ext}")
    else:
        st.caption("No audio on disk for this sample.")

with right:
    st.subheader("Speaker")
    pitches = spk_rows["f0_median"].dropna()
    st.markdown(
        f"- **samples scanned:** {len(spk_rows)}  ·  **with audio:** {int(spk_rows.kept.sum())}\n"
        f"- **median pitch:** {pitches.median():.0f} Hz  "
        f"(range {pitches.min():.0f}–{pitches.max():.0f})\n"
        f"- **total audio:** {spk_rows.duration.sum()/60:.1f} min"
    )
    if len(pitches) > 1:
        fig = px.histogram(spk_rows, x="f0_median", nbins=20,
                           color_discrete_sequence=[COLORS.get(sample.gender, "#888")],
                           title="This speaker's per-sample pitch")
        fig.update_layout(height=240, margin=dict(t=30, b=10), showlegend=False,
                          xaxis_title="F0 (Hz)")
        st.plotly_chart(fig, use_container_width=True)

# other kept clips from this speaker
kept_clips = spk_rows[(spk_rows.kept == 1) & (spk_rows.path != sample.path)]
kept_clips = kept_clips[kept_clips.path.apply(os.path.exists)]
if not kept_clips.empty:
    with st.expander(f"More audio from {sample.speaker} ({len(kept_clips)} clips)"):
        for _, r in kept_clips.head(8).iterrows():
            st.caption(f"{r.f0_median:.0f} Hz · {r.duration:.1f}s")
            ext = os.path.splitext(r.path)[1].lstrip(".") or "wav"
            st.audio(open(r.path, "rb").read(), format=f"audio/{ext}")
