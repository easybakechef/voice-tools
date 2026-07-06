"""Common Voice (Mozilla) — crowd-sourced speech with self-reported gender,
including non-binary. Best source for gender/pitch diversity.

⚠️  GATED + needs an audio backend. One-time setup (your HF account):
  1. Accept the terms on the dataset page:
     https://huggingface.co/datasets/mozilla-foundation/common_voice_17_0
  2. Log in so the CLI has a token:
       .venv/bin/huggingface-cli login
  3. CV audio is mp3, so install an mp3 decoder backend:
       .venv/bin/pip install torchcodec        # (or ensure ffmpeg is installed)
Then:
  .venv/bin/python -m pipeline.ingest common_voice --cv-per-gender 150
"""
import io
import os

import soundfile as sf

RAW = os.path.join(os.path.dirname(__file__), "..", "..", "data", "raw", "common_voice")


def _gender(v):
    if not v:
        return None
    v = str(v).lower()
    if "female" in v or v == "f":
        return "female"
    if "male" in v or v == "m":
        return "male"
    if "non" in v or v == "nb":
        return "nonbinary"
    return None  # ignore unlabelled / other


def items(lang: str = "en", per_gender: int = 150, utts_per_speaker: int = 6):
    from datasets import load_dataset  # imported lazily; only needed for CV

    os.makedirs(RAW, exist_ok=True)
    ds = load_dataset("mozilla-foundation/common_voice_17_0", lang,
                      split="train", streaming=True)

    counts, per_spk = {}, {}
    for ex in ds:
        if counts.get("male", 0) >= per_gender and counts.get("female", 0) >= per_gender:
            break
        g = _gender(ex.get("gender"))
        if not g:
            continue
        if g in ("male", "female") and counts.get(g, 0) >= per_gender:
            continue  # this gender is full; keep scanning for non-binary
        spk = str(ex.get("client_id", "anon"))[:16]
        if per_spk.get(spk, 0) >= utts_per_speaker:
            continue

        audio = ex["audio"]
        out = os.path.join(RAW, f"{spk}_{counts.get(g, 0)}.flac")
        if not os.path.exists(out):
            sf.write(out, audio["array"], audio["sampling_rate"])
        per_spk[spk] = per_spk.get(spk, 0) + 1
        counts[g] = counts.get(g, 0) + 1
        yield dict(dataset="common_voice", speaker=f"cv-{spk}", gender=g,
                   path=os.path.abspath(out))
