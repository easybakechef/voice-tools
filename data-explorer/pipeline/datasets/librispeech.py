"""LibriSpeech (OpenSLR-12) — read speech from LibriVox, gender-labelled.

dev-clean / test-clean are small (~337 MB each), already single-speaker per
file, with SEX in SPEAKERS.TXT. No diarization needed.
"""
import glob
import os
import tarfile
import urllib.request

RAW = os.path.join(os.path.dirname(__file__), "..", "..", "data", "raw")
SUBSETS = {
    "dev-clean": "https://www.openslr.org/resources/12/dev-clean.tar.gz",
    "test-clean": "https://www.openslr.org/resources/12/test-clean.tar.gz",
    "dev-other": "https://www.openslr.org/resources/12/dev-other.tar.gz",
    "test-other": "https://www.openslr.org/resources/12/test-other.tar.gz",
    "train-clean-100": "https://www.openslr.org/resources/12/train-clean-100.tar.gz",
    "train-clean-360": "https://www.openslr.org/resources/12/train-clean-360.tar.gz",
    "train-other-500": "https://www.openslr.org/resources/12/train-other-500.tar.gz",
}


def _download_extract(subset: str) -> str:
    os.makedirs(RAW, exist_ok=True)
    root = os.path.join(RAW, "LibriSpeech")
    if not os.path.isdir(os.path.join(root, subset)):
        tgz = os.path.join(RAW, f"{subset}.tar.gz")
        if not os.path.exists(tgz):
            print(f"Downloading LibriSpeech {subset} …")
            urllib.request.urlretrieve(SUBSETS[subset], tgz)
        print(f"Extracting {subset} …")
        with tarfile.open(tgz) as t:
            t.extractall(RAW)
        os.remove(tgz)  # keep only the (already-compressed) flac, save space
    return root


def _gender_map(root: str) -> dict:
    out = {}
    with open(os.path.join(root, "SPEAKERS.TXT")) as f:
        for line in f:
            if line.startswith(";"):
                continue
            parts = [p.strip() for p in line.split("|")]
            if len(parts) >= 2 and parts[0].isdigit():
                out[parts[0]] = {"F": "female", "M": "male"}.get(parts[1], parts[1])
    return out


def items(subset: str = "dev-clean"):
    """Yield {dataset, speaker, gender, path} for every utterance."""
    root = _download_extract(subset)
    gmap = _gender_map(root)
    base = os.path.join(root, subset)
    for spk in sorted(os.listdir(base)):
        spath = os.path.join(base, spk)
        if not os.path.isdir(spath):
            continue
        for fl in sorted(glob.glob(os.path.join(spath, "*", "*.flac"))):
            yield dict(dataset="librispeech", speaker=f"ls-{spk}",
                       gender=gmap.get(spk), path=os.path.abspath(fl))
