"""VCTK — studio-quality read speech, ~110 speakers, gender-labelled, everyone
reads overlapping prompts (text_id lets us compare the same sentence across
speakers). Pulled from the parquet mirror `sanchit-gandhi/vctk` via
huggingface_hub (no `datasets`/torchcodec needed — we decode the bytes with
soundfile). Each ~400 MB shard is removed after processing to respect the budget.
"""
import io
import os

import pandas as pd
import soundfile as sf
from huggingface_hub import HfApi, hf_hub_download

REPO = "sanchit-gandhi/vctk"
RAW = os.path.join(os.path.dirname(__file__), "..", "..", "data", "raw", "vctk")
GENDER = {"F": "female", "M": "male"}


def _shards():
    files = HfApi().list_repo_files(REPO, repo_type="dataset")
    return sorted(f for f in files if f.endswith(".parquet"))


def items(num_shards: int = 6, shard_start: int = 0, utts_per_speaker: int = 15):
    os.makedirs(RAW, exist_ok=True)
    for shard in _shards()[shard_start:shard_start + num_shards]:
        local = hf_hub_download(REPO, shard, repo_type="dataset")
        df = pd.read_parquet(local, columns=["speaker_id", "gender", "text_id", "audio"])
        per = {}
        for _, row in df.iterrows():
            spk = row["speaker_id"]
            if per.get(spk, 0) >= utts_per_speaker:
                continue
            per[spk] = per.get(spk, 0) + 1
            out = os.path.join(RAW, f"{spk}_{row['text_id']}.flac")
            if not os.path.exists(out):
                arr, sr = sf.read(io.BytesIO(row["audio"]["bytes"]))
                sf.write(out, arr, sr)
            yield dict(dataset="vctk", speaker=f"vctk-{spk}",
                       gender=GENDER.get(row["gender"], row["gender"]),
                       path=os.path.abspath(out))
        # Free the big cached parquet shard — we only keep the small clips we wrote.
        try:
            os.remove(local)
        except OSError:
            pass
