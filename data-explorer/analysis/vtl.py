"""Estimate vocal tract length (and a resonance read) from a single audio file.

    .venv/bin/python -m analysis.vtl path/to/clip.wav
"""
import os
import sys

import numpy as np

from .extract_kept_formants import clip_features

C_CM = 35000.0  # speed of sound, cm/s


def vtl_from_formants(f1: float, f2: float, f3: float, f4: float) -> float:
    """VTL (cm) from F1-F4 via least-squares fit to Fn = (2n-1)·c/(4L)."""
    F = np.array([f1, f2, f3, f4], dtype=float)
    ns = np.array([1, 3, 5, 7], dtype=float)
    slope = np.sum(ns * F) / np.sum(ns * ns)   # = c/(4L)
    return C_CM / (4.0 * slope)


def estimate(path: str) -> dict | None:
    """Extract formants from one recording and return F0, F1-F4, and VTL (cm)."""
    feat = clip_features(path)
    if not feat or any(feat.get(k) is None for k in ("f1", "f2", "f3", "f4")):
        return None
    feat["vtl_cm"] = vtl_from_formants(feat["f1"], feat["f2"], feat["f3"], feat["f4"])
    return feat


def _interpret(vtl: float) -> str:
    # rough guide from the matched-set means (male ~16.5 cm, female ~15.5 cm)
    if vtl >= 16.5:
        return "longer tract → masculine-leaning resonance"
    if vtl <= 15.5:
        return "shorter tract → feminine-leaning resonance"
    return "intermediate resonance"


def main():
    if len(sys.argv) < 2:
        print("usage: python -m analysis.vtl <audio-file>")
        return
    path = sys.argv[1]
    r = estimate(path)
    if not r:
        print("Could not estimate (no clear voiced formants).")
        return
    print(f"{os.path.basename(path)}")
    print(f"  F0  {r['f0']:.0f} Hz")
    print(f"  F1-F4  {r['f1']:.0f} / {r['f2']:.0f} / {r['f3']:.0f} / {r['f4']:.0f} Hz")
    print(f"  estimated VTL  {r['vtl_cm']:.1f} cm  →  {_interpret(r['vtl_cm'])}")


if __name__ == "__main__":
    main()
