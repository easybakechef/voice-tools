# Voice Data Explorer

A standalone project (separate from the voice-trainer webapp) for assembling a
labelled male/female (and non-binary, where available) voice dataset and
exploring it, on the way to a stable, pitch-independent **resonance** metric.

Everything is selective (per-speaker time caps + a disk cap), so the full
dataset stays well under 10 GB.

## Setup

```bash
cd data-explorer
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
```

## Ingest data

Downloads a subset, extracts pitch (F0) + formants (F1/F2 via Praat/parselmouth)
+ duration per recording, and stores them in `data/samples.db`.

```bash
# LibriSpeech (read speech, gender-labelled, already speaker-isolated)
.venv/bin/python -m pipeline.ingest librispeech --subset dev-clean  --seconds-per-speaker 40
.venv/bin/python -m pipeline.ingest librispeech --subset test-clean --seconds-per-speaker 40

# VCTK (studio quality, same prompts across speakers — best for formant work)
.venv/bin/python -m pipeline.ingest vctk --num-shards 6

# Common Voice (diversity + non-binary) — gated, see common_voice.py header:
#   1) accept terms on the HF dataset page
#   2) .venv/bin/huggingface-cli login
#   3) .venv/bin/pip install torchcodec   (mp3 backend)
.venv/bin/python -m pipeline.ingest common_voice --cv-per-gender 150
```

Flags: `--seconds-per-speaker` (cap audio per speaker), `--max-speakers`,
`--num-shards` (VCTK), `--cv-per-gender` (Common Voice), `--disk-cap-gb` (default 10).

## Dashboard

```bash
.venv/bin/streamlit run dashboard/app.py
```

Per-gender metric cards (minutes of audio, # samples, # speakers, avg pitch),
filters (dataset / gender / pitch), per-speaker pitch distributions (isolated +
overlaid), an F1/F2 formant scatter, and per-dataset drill-down with audio
preview.

## Layout

```
pipeline/
  db.py                 SQLite schema + insert
  features.py           parselmouth pitch + formant extraction
  ingest.py             orchestrator (caps, disk guard)
  datasets/
    librispeech.py      OpenSLR-12 dev/test-clean
dashboard/app.py        Streamlit dashboard
data/                   downloaded audio + samples.db   (gitignored)
```

## Status

- ✅ **LibriSpeech** dev-clean + test-clean — 80 speakers (40 F / 40 M).
- ✅ **VCTK** — studio quality, ~26 speakers; pulled from `sanchit-gandhi/vctk`
  parquet via `huggingface_hub` (no `datasets`/torchcodec dependency).
- ⏳ **Common Voice** — implemented, but **gated**: needs your HF login + accepted
  terms + an mp3 backend (see `pipeline/datasets/common_voice.py`). This is the
  source for non-binary + maximum diversity.

## Roadmap

- **Metric development**: once data is assembled, explore what combination of
  formants (+ a vocal-tract-length estimate) separates gender *independent of
  F0*, validated on held-out speakers — then port a robust version to the live
  bar in the webapp.

## Notes

- Formant ceiling is fixed (5500 Hz) at extraction so the features aren't biased
  by the gender label — we want the metric to *discover* gender, not be told it.
- Pitch + formants use the median over voiced frames (robust to octave errors).
