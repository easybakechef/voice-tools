"""Download → extract features → store, with per-speaker time caps + a disk cap.

Usage:
    python -m pipeline.ingest librispeech --subset dev-clean --seconds-per-speaker 40
"""
import argparse
import os

from .db import connect, insert_sample, DATA_DIR
from .features import extract_features
from .datasets import librispeech, vctk, common_voice

SOURCES = {"librispeech": librispeech, "vctk": vctk, "common_voice": common_voice}


def _dir_size_gb(path: str) -> float:
    total = 0
    for root, _, files in os.walk(path):
        for f in files:
            try:
                total += os.path.getsize(os.path.join(root, f))
            except OSError:
                pass
    return total / 1e9


def run(name: str, seconds_per_speaker: float, max_speakers: int | None,
        disk_cap_gb: float, **source_kwargs):
    con = connect()
    source = SOURCES[name]
    existing = {p for (p,) in con.execute("select path from samples")}
    per_speaker = {}      # speaker -> seconds accumulated
    n = skipped = 0

    for it in source.items(**source_kwargs):
        if it["path"] in existing:
            continue  # already ingested — idempotent re-runs
        spk = it["speaker"]
        if max_speakers and spk not in per_speaker and len(per_speaker) >= max_speakers:
            continue
        if per_speaker.get(spk, 0.0) >= seconds_per_speaker:
            continue
        if _dir_size_gb(DATA_DIR) > disk_cap_gb:
            print(f"Disk cap {disk_cap_gb} GB reached — stopping.")
            break
        try:
            feat = extract_features(it["path"])
        except Exception as e:  # noqa: BLE001
            skipped += 1
            print("  skip", it["path"], e)
            continue
        per_speaker[spk] = per_speaker.get(spk, 0.0) + (feat["duration"] or 0.0)
        insert_sample(con, **it, **feat)
        n += 1
        if n % 50 == 0:
            con.commit()
            print(f"  {n} samples, {len(per_speaker)} speakers …")

    con.commit()
    con.close()
    print(f"Done: {n} samples from {name} across {len(per_speaker)} speakers "
          f"({skipped} skipped). Data dir: {_dir_size_gb(DATA_DIR):.2f} GB")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("source", choices=SOURCES.keys())
    ap.add_argument("--subset", default="dev-clean")
    ap.add_argument("--num-shards", type=int, default=6, help="VCTK: parquet shards to pull (~5 speakers each)")
    ap.add_argument("--shard-start", type=int, default=0, help="VCTK: first shard index (to fetch new speakers)")
    ap.add_argument("--cv-per-gender", type=int, default=150, help="Common Voice: samples to collect per gender")
    ap.add_argument("--seconds-per-speaker", type=float, default=40.0)
    ap.add_argument("--max-speakers", type=int, default=None)
    ap.add_argument("--disk-cap-gb", type=float, default=10.0)
    args = ap.parse_args()
    if args.source == "librispeech":
        kwargs = {"subset": args.subset}
    elif args.source == "vctk":
        kwargs = {"num_shards": args.num_shards, "shard_start": args.shard_start}
    elif args.source == "common_voice":
        kwargs = {"per_gender": args.cv_per_gender}
    else:
        kwargs = {}
    run(args.source, args.seconds_per_speaker, args.max_speakers, args.disk_cap_gb, **kwargs)


if __name__ == "__main__":
    main()
