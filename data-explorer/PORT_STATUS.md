# Rust + SvelteKit port — status / plan

Goal: convert the (Python/Streamlit) data-explorer to **full Rust + SvelteKit**,
and add a dashboard page showing the samples the resonance metric struggles on.

Decision: **full Rust** — including LPC/Burg formant extraction (no Praat). The
risk is degrading the validated metric (VTL, AUC≈0.85 on the pitch-matched set),
so the Rust formant extraction is **validated against the Python/parselmouth
baseline before anything else is ported on top.**

Reuse the existing `data/` (2.7 GB): `data/samples.db` (SQLite) + `data/kept/`
(6,164 crossover flac clips) + the parselmouth-derived `analysis/kept_features.csv`
as the ground-truth to validate Rust formants against.

## Layout (new)
```
data-explorer/
  rust/                    Rust workspace
    crates/voice-analysis/ DSP lib: decode, pitch (NSDF), LPC/Burg formants, VTL
    crates/pipeline/       binary: ingest/scan/extract → SQLite; metric/eval
  web/                     SvelteKit dashboard (reads samples.db)
  data/                    reused as-is
  analysis/ pipeline/ dashboard/   OLD python (kept until the port validates)
```

## Tasks
1. [DONE] voice-analysis crate: audio decode + NSDF pitch + Burg LPC formants + VTL
2. [DONE] Validate Rust F1-F4/VTL vs parselmouth — PASSED (see below)
3. [DONE] Rust scan binary (stream LibriSpeech tarball → rusqlite); validated F0 corr 0.995 vs Praat
4. [DONE] Rust metric binary → resonance table + hard cases
5. [DONE] SvelteKit dashboard: overview + sample explorer + hard-cases page

## VALIDATION RESULT (Full Rust preserved the metric)
Rust vs parselmouth per-speaker agreement (1040 shared speakers):
F1 corr 0.93, F2 0.91, F3 0.95, F4 0.96, VTL 0.95 (MAE 0.2 cm).
Metric AUC — Rust VTL matched 0.87 / full 0.87 vs parselmouth 0.84/0.85;
VTL means male 16.4 / female 15.4 cm; matched accuracy 80.4%; threshold 15.91 cm.
So Full Rust did NOT degrade (slightly improved) the metric.

KEY DSP LESSON: a hard formant-bandwidth cutoff (was 400 Hz) drops real broad
formants and cascades into misassignment (F2→F3 shift), which crashed AUC to 0.71.
Raising the cap to 3000 Hz (near-off, sort poles by frequency like Praat) fixed it.
Other gotcha: nalgebra complex_eigenvalues per-frame was fine single-thread but
malloc-contended under rayon (177 CPU-min); replaced with alloc-free Durand-Kerner
root finding → 8 CPU-min for the whole kept set.

## How to run
- scan a subset:      rust/target/release/scan train-clean-100 data 12   (streams tarball → samples.db)
- extract features:  rust/target/release/extract data/kept data/raw/LibriSpeech/SPEAKERS.TXT analysis/rust_kept_features.csv 4
- build metric table: rust/target/release/metric analysis/rust_kept_features.csv data/samples.db
- dashboard:          cd web && npm run build && node build/index.js   (or npm run dev)
- validate vs Praat:  .venv/bin/python -m analysis.validate_rust

## Metric recap (target to preserve)
VTL (cm) from F1-F4: fit Fn = (2n-1)·c/(4L), c=35000 cm/s → L = c/(4·slope),
slope = Σ(2n-1)·Fn / Σ(2n-1)². Matched-set: male 16.5±0.8, female 15.5±0.6 cm,
Cohen's d≈1.4, AUC≈0.85, ~80% best-threshold accuracy.
