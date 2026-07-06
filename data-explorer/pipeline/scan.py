"""Wide pitch scan of full LibriSpeech subsets, indexing every scanned sample but
keeping audio ONLY for pitch-crossover voices (female F0 < 180 Hz, male F0 > 140).

Streams each tarball over HTTP and decodes flac bytes in memory — so we never
store the ~60 GB of train audio on disk; only the crossover clips are written.

Usage:
    python -m pipeline.scan train-clean-100 [cap_per_speaker]
    python -m pipeline.scan train-clean-360
    python -m pipeline.scan train-other-500
"""
import io
import os
import sys
import tarfile

import requests
import soundfile as sf

from .datasets import librispeech
from .db import connect, insert_sample, DATA_DIR
from .features import pitch_array

KEEP_DIR = os.path.join(DATA_DIR, "kept")
SPEAKERS = os.path.join(DATA_DIR, "raw", "LibriSpeech", "SPEAKERS.TXT")

FEMALE_MAX = 180.0
MALE_MIN = 140.0


def keep_audio(gender, f0) -> bool:
    if f0 is None:
        return False
    return (gender == "female" and f0 < FEMALE_MAX) or (gender == "male" and f0 > MALE_MIN)


def gender_map() -> dict:
    if not os.path.exists(SPEAKERS):
        raise SystemExit("SPEAKERS.TXT missing — run a small librispeech ingest first "
                         "(e.g. `python -m pipeline.ingest librispeech --subset dev-clean`).")
    out = {}
    with open(SPEAKERS) as f:
        for line in f:
            if line.startswith(";"):
                continue
            p = [x.strip() for x in line.split("|")]
            if len(p) >= 2 and p[0].isdigit():
                out[p[0]] = {"F": "female", "M": "male"}.get(p[1], p[1])
    return out


def scan(subset: str, cap_per_speaker: int = 12):
    con = connect()
    gmap = gender_map()
    os.makedirs(KEEP_DIR, exist_ok=True)
    url = librispeech.SUBSETS[subset]
    print(f"Streaming {subset} …", flush=True)

    per_spk = {}
    n = kept = 0
    with requests.get(url, stream=True, timeout=120) as r:
        r.raise_for_status()
        r.raw.decode_content = False
        with tarfile.open(fileobj=r.raw, mode="r|gz") as tar:
            for m in tar:
                if not m.name.endswith(".flac"):
                    continue
                parts = m.name.split("/")
                if len(parts) < 4:
                    continue
                spk = parts[2]
                if per_spk.get(spk, 0) >= cap_per_speaker:
                    continue
                fh = tar.extractfile(m)
                if fh is None:
                    continue
                try:
                    arr, sr = sf.read(io.BytesIO(fh.read()))
                    feat = pitch_array(arr, sr)
                except Exception:  # noqa: BLE001
                    continue
                per_spk[spk] = per_spk.get(spk, 0) + 1
                gender = gmap.get(spk)
                keep = keep_audio(gender, feat["f0_median"])
                if keep:
                    dst = os.path.join(KEEP_DIR, f"ls-{spk}-{os.path.basename(m.name)}")
                    sf.write(dst, arr, sr, format="FLAC")
                    path = os.path.abspath(dst)
                    kept += 1
                else:
                    path = f"discarded://librispeech/{os.path.basename(m.name)}"
                insert_sample(con, dataset="librispeech", speaker=f"ls-{spk}", gender=gender,
                              path=path, kept=1 if keep else 0, **feat)
                n += 1
                if n % 1000 == 0:
                    con.commit()
                    print(f"  {n} scanned · {kept} kept · {len(per_spk)} speakers", flush=True)
    con.commit()
    con.close()
    print(f"Done {subset}: {n} samples indexed, {kept} kept ({len(per_spk)} speakers).", flush=True)


if __name__ == "__main__":
    sub = sys.argv[1] if len(sys.argv) > 1 else "train-clean-100"
    cap = int(sys.argv[2]) if len(sys.argv) > 2 else 12
    scan(sub, cap)
