"""Per-recording acoustic features via Praat (parselmouth): pitch + formants.

Uses median over voiced frames (robust to octave errors and unvoiced noise).
Formants come from Burg LPC — far more reliable than FFT peak-picking.
"""
import numpy as np
import soundfile as sf
import parselmouth


def pitch_array(arr, sr: int) -> dict:
    """Fast pitch-only features from an in-memory audio array (for bulk scanning).
    Returns the same keys as extract_features, with formants left null."""
    if getattr(arr, "ndim", 1) > 1:
        arr = arr.mean(axis=1)
    snd = parselmouth.Sound(arr.astype("float64"), sampling_frequency=sr)
    pitch = snd.to_pitch(time_step=0.01, pitch_floor=60.0, pitch_ceiling=500.0)
    f0 = pitch.selected_array["frequency"]
    voiced = f0[f0 > 0]
    return dict(duration=float(snd.get_total_duration()),
                f0_median=float(np.median(voiced)) if voiced.size else None,
                voiced_frac=float(voiced.size / f0.size) if f0.size else 0.0,
                f1_median=None, f2_median=None)


def extract_features(path: str) -> dict:
    audio, sr = sf.read(path, dtype="float32", always_2d=False)
    if audio.ndim > 1:
        audio = audio.mean(axis=1)
    snd = parselmouth.Sound(audio, sampling_frequency=sr)

    duration = float(snd.get_total_duration())

    pitch = snd.to_pitch(time_step=0.01, pitch_floor=60.0, pitch_ceiling=500.0)
    f0 = pitch.selected_array["frequency"]
    voiced = f0[f0 > 0]
    f0_median = float(np.median(voiced)) if voiced.size else None
    voiced_frac = float(voiced.size / f0.size) if f0.size else 0.0

    # Single fixed ceiling (5500 Hz) so extraction isn't biased by the gender
    # label — we want the metric to discover gender, not be told it.
    f1_median = f2_median = None
    if voiced.size:
        formant = snd.to_formant_burg(time_step=0.01, max_number_of_formants=5,
                                      maximum_formant=5500.0)
        times = pitch.xs()
        f1s, f2s = [], []
        for t, v in zip(times, f0):
            if v <= 0:
                continue
            a = formant.get_value_at_time(1, t)
            b = formant.get_value_at_time(2, t)
            if a and not np.isnan(a):
                f1s.append(a)
            if b and not np.isnan(b):
                f2s.append(b)
        if f1s:
            f1_median = float(np.median(f1s))
        if f2s:
            f2_median = float(np.median(f2s))

    return dict(duration=duration, f0_median=f0_median, voiced_frac=voiced_frac,
                f1_median=f1_median, f2_median=f2_median)
