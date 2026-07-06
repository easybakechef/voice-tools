//! Voice analysis DSP: audio decode, NSDF pitch, Burg-LPC formants, VTL metric.
//!
//! Formant extraction mirrors Praat's `to_formant_burg`: resample to
//! 2*max_formant, pre-emphasis, short Gaussian window, Burg LPC of order
//! 2*n_formants, then formants from the roots of the prediction polynomial.
//! Aggregation matches `analysis/extract_kept_formants.py`: median of each
//! formant over voiced frames.

pub mod audio;

use rustfft::{num_complex::Complex, FftPlanner};

pub const C_CM: f64 = 35000.0; // speed of sound, cm/s (Fitch convention)
pub const MAX_FORMANT: f64 = 5500.0;
const RESAMPLE_SR: f64 = 2.0 * MAX_FORMANT; // 11025-ish target; we use exactly 2*Fmax
const N_FORMANTS: usize = 5;
const LPC_ORDER: usize = 2 * N_FORMANTS; // 10
pub const N_LPCC: usize = 12; // LPC-cepstral coeffs (vocal-tract envelope descriptor)
const FFT_SIZE: usize = 1024;
const PITCH_FLOOR: f64 = 60.0;
const PITCH_CEILING: f64 = 500.0;
const CLARITY_MIN: f64 = 0.5;
const HOP_S: f64 = 0.01;
const FORMANT_WIN_S: f64 = 0.025;
const FORMANT_BW_MAX: f64 = 3000.0; // near-off: dropping real (broad) poles cascades into misassignment

#[derive(Debug, Clone, Default)]
pub struct Features {
    pub f0: Option<f64>,
    pub f1: Option<f64>,
    pub f2: Option<f64>,
    pub f3: Option<f64>,
    pub f4: Option<f64>,
    pub f5: Option<f64>,
    /// Mean LPC-cepstral coefficients over voiced frames — a compact,
    /// pitch-independent description of the vocal-tract resonance envelope.
    pub lpcc: Vec<f64>,
    pub centroid: Option<f64>, // spectral centre of gravity (Hz)
    pub tilt: Option<f64>,     // spectral tilt (dB/kHz), source/breathiness cue
    pub rolloff: Option<f64>,  // fraction of energy above 2 kHz
    pub h1h2: Option<f64>,     // H1-H2 (dB), glottal open-quotient / breathiness cue
    // vowel-space dispersion (women's vowel space is larger / more peripheral)
    pub vsa: Option<f64>,      // vowel-space area proxy = sqrt(det cov(F1,F2))
    pub f1_disp: Option<f64>,  // F1 spread (IQR) over voiced frames
    pub f2_disp: Option<f64>,  // F2 spread (IQR) over voiced frames
    // vowel-identified space: k-means the F1×F2 cloud into vowel clusters, then
    // the convex-hull area of the cluster centroids (raw + tract-length-normalized)
    pub vsa_hull: Option<f64>,
    pub vsa_hull_norm: Option<f64>,
    // dynamic / temporal cues (women use wider excursions & more formant movement)
    pub f0_range: Option<f64>,   // F0 dynamic range, semitones (P95/P05)
    pub traj_f12: Option<f64>,   // mean formant movement per adjacent frame (F1×F2, Hz)
    pub spec_rate: Option<f64>,  // mean |ΔLPC-cepstrum| per adjacent frame (VISC proxy)
    pub f2_range: Option<f64>,   // F2 dynamic range, Hz (P90-P10)
    // sibilant (/s/,/ʃ/) spectral moments — unvoiced frames at NATIVE sr (keeps 5.5-8 kHz)
    pub sib_m1: Option<f64>,     // center of gravity (Hz)
    pub sib_m2: Option<f64>,     // spread (Hz)
    pub sib_m3: Option<f64>,     // skewness
    pub sib_m4: Option<f64>,     // excess kurtosis
    pub sib_hi: Option<f64>,     // >5 kHz energy fraction
    pub sib_peak: Option<f64>,   // peak frequency (Hz)
    pub voiced_frames: usize,
}

impl Features {
    pub fn has_formants(&self) -> bool {
        self.f1.is_some() && self.f2.is_some() && self.f3.is_some() && self.f4.is_some()
    }
    /// Vocal tract length (cm) from F1-F4 via least-squares fit to
    /// Fn = (2n-1)*c/(4L) through the origin.
    pub fn vtl_cm(&self) -> Option<f64> {
        match (self.f1, self.f2, self.f3, self.f4) {
            (Some(a), Some(b), Some(c), Some(d)) => {
                let f = [a, b, c, d];
                let ns = [1.0, 3.0, 5.0, 7.0];
                let num: f64 = ns.iter().zip(f).map(|(n, x)| n * x).sum();
                let den: f64 = ns.iter().map(|n| n * n).sum();
                let slope = num / den; // = c/(4L)
                Some(C_CM / (4.0 * slope))
            }
            _ => None,
        }
    }
}

/// Full pipeline: analyze mono samples at native `sr` and return median features.
pub fn analyze(samples: &[f32], sr: u32) -> Features {
    let sr = sr as f64;
    let x: Vec<f64> = samples.iter().map(|&v| v as f64).collect();
    let rs = resample(&x, sr, RESAMPLE_SR);
    let fsr = RESAMPLE_SR;

    let hop = (HOP_S * fsr).round() as usize;
    let fwin = (FORMANT_WIN_S * fsr).round() as usize;
    // pitch needs ~3 periods of the lowest pitch
    let pwin = ((3.0 / PITCH_FLOOR) * fsr).round() as usize;
    let half_p = pwin / 2;
    let half_f = fwin / 2;

    if rs.len() < pwin + hop {
        return Features::default();
    }

    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    let mut f0s: Vec<f64> = Vec::new();
    let mut cols: [Vec<f64>; 5] = Default::default(); // F1..F5
    let mut f12: Vec<(f64, f64)> = Vec::new(); // paired (F1,F2) per frame, for vowel-space area
    let mut lpcc_sum = vec![0.0f64; N_LPCC];
    let mut lpcc_n = 0usize;
    let mut centroids: Vec<f64> = Vec::new();
    let mut tilts: Vec<f64> = Vec::new();
    let mut rolloffs: Vec<f64> = Vec::new();
    let mut h1h2s: Vec<f64> = Vec::new();
    // dynamic-cue accumulators (over temporally adjacent voiced frames)
    let mut traj_sum = 0.0;
    let mut spec_sum = 0.0;
    let mut traj_cnt = 0usize;
    let mut last_voiced: Option<(usize, f64, f64, Vec<f64>)> = None;

    let mut center = half_p.max(half_f);
    let end = rs.len().saturating_sub(half_p.max(half_f) + 1);
    while center < end {
        let pbuf = &rs[center - half_p..center + half_p];
        if let Some((f0, clarity)) = detect_pitch(pbuf, fsr) {
            if clarity >= CLARITY_MIN && (PITCH_FLOOR..=PITCH_CEILING).contains(&f0) {
                f0s.push(f0);
                let fbuf = &rs[center - half_f..center + half_f];
                let (fmts, lpcc) = lpc_frame_features(fbuf, fsr);
                for (k, col) in cols.iter_mut().enumerate() {
                    if let Some(&v) = fmts.get(k) {
                        col.push(v);
                    }
                }
                if fmts.len() >= 2 {
                    f12.push((fmts[0], fmts[1]));
                }
                if lpcc.len() == N_LPCC {
                    for (s, v) in lpcc_sum.iter_mut().zip(&lpcc) {
                        *s += v;
                    }
                    lpcc_n += 1;
                }
                if let Some(s) = spectral_frame_features(fbuf, fsr, f0, fft.as_ref()) {
                    centroids.push(s.centroid);
                    tilts.push(s.tilt);
                    rolloffs.push(s.rolloff);
                    h1h2s.push(s.h1h2);
                }
                // dynamic cues: movement between temporally adjacent voiced frames
                if fmts.len() >= 2 {
                    let (cf1, cf2) = (fmts[0], fmts[1]);
                    if let Some((pc, pf1, pf2, plpcc)) = &last_voiced {
                        if center == pc + hop {
                            traj_sum += ((cf1 - pf1).powi(2) + (cf2 - pf2).powi(2)).sqrt();
                            if !lpcc.is_empty() && plpcc.len() == lpcc.len() {
                                spec_sum += lpcc
                                    .iter()
                                    .zip(plpcc)
                                    .map(|(a, b)| (a - b).abs())
                                    .sum::<f64>()
                                    / lpcc.len() as f64;
                            }
                            traj_cnt += 1;
                        }
                    }
                    last_voiced = Some((center, cf1, cf2, lpcc.clone()));
                }
            }
        }
        center += hop;
    }

    if f0s.len() < 5 {
        return Features::default();
    }
    let lpcc = if lpcc_n > 0 {
        lpcc_sum.iter().map(|s| s / lpcc_n as f64).collect()
    } else {
        Vec::new()
    };
    // sibilants run on the NATIVE-rate signal (keeps the 5.5-8 kHz /s/ band)
    let sib = sibilant_features(&x, sr);
    Features {
        f0: median(&mut f0s.clone()),
        f1: median(&mut cols[0].clone()),
        f2: median(&mut cols[1].clone()),
        f3: median(&mut cols[2].clone()),
        f4: median(&mut cols[3].clone()),
        f5: median(&mut cols[4].clone()),
        lpcc,
        centroid: median(&mut centroids.clone()),
        tilt: median(&mut tilts.clone()),
        rolloff: median(&mut rolloffs.clone()),
        h1h2: median(&mut h1h2s.clone()),
        vsa: vowel_space_area(&f12),
        f1_disp: iqr(&mut cols[0].clone()),
        f2_disp: iqr(&mut cols[1].clone()),
        vsa_hull: vowel_space_hull(&f12, 8).map(|h| h.0),
        vsa_hull_norm: vowel_space_hull(&f12, 8).map(|h| h.1),
        f0_range: semitone_range(&mut f0s.clone()),
        traj_f12: if traj_cnt > 0 { Some(traj_sum / traj_cnt as f64) } else { None },
        spec_rate: if traj_cnt > 0 { Some(spec_sum / traj_cnt as f64) } else { None },
        f2_range: pct_range(&mut cols[1].clone(), 0.10, 0.90),
        sib_m1: sib.map(|s| s.m1),
        sib_m2: sib.map(|s| s.m2),
        sib_m3: sib.map(|s| s.m3),
        sib_m4: sib.map(|s| s.m4),
        sib_hi: sib.map(|s| s.hi),
        sib_peak: sib.map(|s| s.peak),
        voiced_frames: f0s.len(),
    }
}

fn percentile_sorted(v: &[f64], p: f64) -> f64 {
    let idx = p * (v.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let frac = idx - lo as f64;
    v[lo] + frac * (v[(lo + 1).min(v.len() - 1)] - v[lo])
}

/// F0 dynamic range in semitones: 12*log2(P95/P05).
fn semitone_range(v: &mut Vec<f64>) -> Option<f64> {
    if v.len() < 8 {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let (lo, hi) = (percentile_sorted(v, 0.05), percentile_sorted(v, 0.95));
    if lo > 0.0 {
        Some(12.0 * (hi / lo).log2())
    } else {
        None
    }
}

/// Percentile range (Phi - Plo) of a sample.
fn pct_range(v: &mut Vec<f64>, plo: f64, phi: f64) -> Option<f64> {
    if v.len() < 8 {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(percentile_sorted(v, phi) - percentile_sorted(v, plo))
}

/// "Vowel identification" via k-means on the F1×F2 cloud, then the convex-hull
/// area of the cluster centroids. Returns (raw area, tract-length-normalized area):
/// the normalized version divides each axis by its mean, removing the anatomical
/// 1/L formant scaling so it reflects behavioural vowel-space *expansion*.
fn vowel_space_hull(pts: &[(f64, f64)], k: usize) -> Option<(f64, f64)> {
    if pts.len() < k * 6 {
        return None;
    }
    let cents = kmeans2(pts, k, 25);
    if cents.len() < 3 {
        return None;
    }
    let raw = convex_hull_area(&cents);
    let n = pts.len() as f64;
    let mx = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let my = pts.iter().map(|p| p.1).sum::<f64>() / n;
    if mx <= 0.0 || my <= 0.0 {
        return Some((raw, raw));
    }
    let normed: Vec<(f64, f64)> = cents.iter().map(|&(a, b)| (a / mx, b / my)).collect();
    Some((raw, convex_hull_area(&normed)))
}

/// Lloyd's k-means on 2-D points; returns the non-empty cluster centroids.
fn kmeans2(pts: &[(f64, f64)], k: usize, iters: usize) -> Vec<(f64, f64)> {
    let mut cent: Vec<(f64, f64)> = (0..k).map(|i| pts[i * pts.len() / k]).collect();
    let mut assign = vec![0usize; pts.len()];
    for _ in 0..iters {
        for (i, &p) in pts.iter().enumerate() {
            let mut best = 0;
            let mut bd = f64::MAX;
            for (j, &c) in cent.iter().enumerate() {
                let d = (p.0 - c.0).powi(2) + (p.1 - c.1).powi(2);
                if d < bd {
                    bd = d;
                    best = j;
                }
            }
            assign[i] = best;
        }
        let mut sum = vec![(0.0, 0.0); k];
        let mut cnt = vec![0usize; k];
        for (i, &p) in pts.iter().enumerate() {
            sum[assign[i]].0 += p.0;
            sum[assign[i]].1 += p.1;
            cnt[assign[i]] += 1;
        }
        for j in 0..k {
            if cnt[j] > 0 {
                cent[j] = (sum[j].0 / cnt[j] as f64, sum[j].1 / cnt[j] as f64);
            }
        }
    }
    // keep only non-empty (roughly): dedupe near-identical centroids
    cent
}

/// Convex-hull area (monotone chain + shoelace).
fn convex_hull_area(points: &[(f64, f64)]) -> f64 {
    let mut p: Vec<(f64, f64)> = points.to_vec();
    p.sort_by(|a, b| a.partial_cmp(b).unwrap());
    p.dedup();
    if p.len() < 3 {
        return 0.0;
    }
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut hull: Vec<(f64, f64)> = Vec::new();
    for &pt in &p {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], pt) <= 0.0 {
            hull.pop();
        }
        hull.push(pt);
    }
    let lower = hull.len() + 1;
    for &pt in p.iter().rev() {
        while hull.len() >= lower && cross(hull[hull.len() - 2], hull[hull.len() - 1], pt) <= 0.0 {
            hull.pop();
        }
        hull.push(pt);
    }
    hull.pop();
    // shoelace
    let mut area = 0.0;
    for i in 0..hull.len() {
        let j = (i + 1) % hull.len();
        area += hull[i].0 * hull[j].1 - hull[j].0 * hull[i].1;
    }
    area.abs() / 2.0
}

/// Interquartile range (robust spread) of a sample.
fn iqr(v: &mut Vec<f64>) -> Option<f64> {
    if v.len() < 8 {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| {
        let idx = p * (v.len() - 1) as f64;
        let lo = idx.floor() as usize;
        let frac = idx - lo as f64;
        v[lo] + frac * (v[(lo + 1).min(v.len() - 1)] - v[lo])
    };
    Some(q(0.75) - q(0.25))
}

/// Vowel-space area proxy: sqrt(det) of the F1×F2 covariance (∝ area of the
/// 1-SD dispersion ellipse). Larger = more peripheral/expanded vowel space.
fn vowel_space_area(pts: &[(f64, f64)]) -> Option<f64> {
    if pts.len() < 10 {
        return None;
    }
    let n = pts.len() as f64;
    let (mx, my) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    let (mx, my) = (mx / n, my / n);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for &(x, y) in pts {
        sxx += (x - mx) * (x - mx);
        syy += (y - my) * (y - my);
        sxy += (x - mx) * (y - my);
    }
    let (sxx, syy, sxy) = (sxx / n, syy / n, sxy / n);
    let det = sxx * syy - sxy * sxy;
    if det <= 0.0 {
        None
    } else {
        Some(det.sqrt())
    }
}

/// Fast pitch-only features for bulk scanning (mirrors pipeline/features.pitch_array):
/// duration, median F0 over voiced frames, and the voiced fraction. No formants,
/// no resampling — runs at the native sample rate.
#[derive(Debug, Clone)]
pub struct PitchFeatures {
    pub duration: f64,
    pub f0_median: Option<f64>,
    pub voiced_frac: f64,
}

pub fn pitch_features(samples: &[f32], sr: u32) -> PitchFeatures {
    let srf = sr as f64;
    let x: Vec<f64> = samples.iter().map(|&v| v as f64).collect();
    let duration = x.len() as f64 / srf;
    let hop = (HOP_S * srf).round() as usize;
    let pwin = ((3.0 / PITCH_FLOOR) * srf).round() as usize;
    let half = pwin / 2;
    if x.len() < pwin || hop == 0 {
        return PitchFeatures { duration, f0_median: None, voiced_frac: 0.0 };
    }
    let mut f0s: Vec<f64> = Vec::new();
    let mut total = 0usize;
    let mut center = half;
    while center + half < x.len() {
        total += 1;
        let buf = &x[center - half..center + half];
        if let Some((f0, clarity)) = detect_pitch(buf, srf) {
            if clarity >= CLARITY_MIN && (PITCH_FLOOR..=PITCH_CEILING).contains(&f0) {
                f0s.push(f0);
            }
        }
        center += hop;
    }
    let voiced_frac = if total > 0 { f0s.len() as f64 / total as f64 } else { 0.0 };
    let mut sorted = f0s.clone();
    PitchFeatures {
        duration,
        f0_median: median(&mut sorted),
        voiced_frac,
    }
}

fn median(v: &mut Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    })
}

// ── pitch (McLeod / NSDF) ────────────────────────────────────────────────────
fn detect_pitch(buf: &[f64], sr: f64) -> Option<(f64, f64)> {
    let n = buf.len();
    let max_tau = ((sr / PITCH_FLOOR) as usize).min(n - 1);
    let min_tau = (sr / PITCH_CEILING) as usize;
    if max_tau <= min_tau + 2 {
        return None;
    }
    let mut nsdf = vec![0.0f64; max_tau + 1];
    for tau in 0..=max_tau {
        let mut acf = 0.0;
        let mut norm = 0.0;
        for j in 0..(n - tau) {
            acf += buf[j] * buf[j + tau];
            norm += buf[j] * buf[j] + buf[j + tau] * buf[j + tau];
        }
        nsdf[tau] = if norm > 0.0 { 2.0 * acf / norm } else { 0.0 };
    }
    // key maxima: local maxima between positive-going zero crossings
    let mut maxima: Vec<usize> = Vec::new();
    let mut tau = min_tau.max(1);
    while tau < max_tau {
        if nsdf[tau] > 0.0 && nsdf[tau - 1] <= 0.0 {
            // rising edge; find the peak until it goes negative
            let mut peak = tau;
            let mut t = tau;
            while t < max_tau && nsdf[t] > 0.0 {
                if nsdf[t] > nsdf[peak] {
                    peak = t;
                }
                t += 1;
            }
            maxima.push(peak);
            tau = t;
        } else {
            tau += 1;
        }
    }
    if maxima.is_empty() {
        return None;
    }
    let global = maxima.iter().map(|&i| nsdf[i]).fold(0.0f64, f64::max);
    let thresh = 0.9 * global;
    let chosen = *maxima.iter().find(|&&i| nsdf[i] >= thresh)?;
    if chosen == 0 {
        return None;
    }
    // parabolic interpolation around the chosen lag
    let (period, clarity) = parabolic(&nsdf, chosen);
    if period <= 0.0 {
        return None;
    }
    Some((sr / period, clarity))
}

fn parabolic(y: &[f64], i: usize) -> (f64, f64) {
    if i == 0 || i + 1 >= y.len() {
        return (i as f64, y[i]);
    }
    let (a, b, c) = (y[i - 1], y[i], y[i + 1]);
    let denom = a - 2.0 * b + c;
    if denom.abs() < 1e-12 {
        return (i as f64, b);
    }
    let delta = 0.5 * (a - c) / denom;
    (i as f64 + delta, b - 0.25 * (a - c) * delta)
}

// ── formants + LPC cepstrum (Burg LPC → roots + cepstral recursion) ───────────
/// Returns (formant frequencies sorted ascending, LPC-cepstral coefficients).
fn lpc_frame_features(frame: &[f64], sr: f64) -> (Vec<f64>, Vec<f64>) {
    let n = frame.len();
    if n < LPC_ORDER + 4 {
        return (Vec::new(), Vec::new());
    }
    // pre-emphasis from 50 Hz, then Gaussian window
    let pre = (-2.0 * std::f64::consts::PI * 50.0 / sr).exp();
    let mut w = vec![0.0f64; n];
    let mid = (n - 1) as f64 / 2.0;
    let sigma = mid / 2.5;
    let mut prev = frame[0];
    for i in 0..n {
        let emph = frame[i] - pre * prev;
        prev = frame[i];
        let z = ((i as f64) - mid) / sigma;
        let g = (-0.5 * z * z).exp();
        w[i] = emph * g;
    }
    let d = burg_lpc(&w, LPC_ORDER);
    let lpcc = lpc_cepstrum(&d, N_LPCC);
    // AR prediction polynomial P(z) = z^m - d0 z^{m-1} - ... - d_{m-1}
    // (companion char. poly); its roots are the formant poles.
    let m = d.len();
    let mut coeffs = Vec::with_capacity(m + 1);
    coeffs.push(1.0);
    for v in &d {
        coeffs.push(-v);
    }
    let roots = durand_kerner(&coeffs);
    let mut fmts: Vec<f64> = Vec::new();
    for (re, im) in roots {
        if im <= 0.0 {
            continue; // take one of each conjugate pair
        }
        let mag = (re * re + im * im).sqrt();
        if mag >= 1.0 || mag <= 0.0 {
            continue;
        }
        let freq = im.atan2(re) * sr / (2.0 * std::f64::consts::PI);
        let bw = -(sr / std::f64::consts::PI) * mag.ln();
        if freq > 90.0 && freq < MAX_FORMANT && bw < FORMANT_BW_MAX {
            fmts.push(freq);
        }
    }
    fmts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (fmts, lpcc)
}

/// LPC cepstral coefficients from the predictor coefficients `d`
/// (x[i] = Σ d[k-1]·x[i-k]). Standard recursion for the all-pole cepstrum.
fn lpc_cepstrum(d: &[f64], n_cc: usize) -> Vec<f64> {
    let p = d.len();
    let a = |k: usize| d[k - 1]; // 1-indexed predictor coeff
    let mut c = vec![0.0f64; n_cc + 1];
    for n in 1..=n_cc {
        let mut s = 0.0;
        for k in 1..n {
            if n - k >= 1 && n - k <= p {
                s += (k as f64 / n as f64) * c[k] * a(n - k);
            }
        }
        let an = if n <= p { a(n) } else { 0.0 };
        c[n] = an + s;
    }
    c[1..=n_cc].to_vec()
}

struct SpectralFrame {
    centroid: f64,
    tilt: f64,
    rolloff: f64,
    h1h2: f64,
}

/// FFT-based source/shape features on a Hann-windowed frame (no pre-emphasis, so
/// spectral tilt reflects the glottal source). Freq band limited to 0..MAX_FORMANT.
fn spectral_frame_features(frame: &[f64], sr: f64, f0: f64, fft: &dyn rustfft::Fft<f64>) -> Option<SpectralFrame> {
    let n = frame.len().min(FFT_SIZE);
    let mut buf = vec![Complex::new(0.0, 0.0); FFT_SIZE];
    for i in 0..n {
        // Hann window
        let hann = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (n as f64 - 1.0)).cos();
        buf[i] = Complex::new(frame[i] * hann, 0.0);
    }
    fft.process(&mut buf);
    let half = FFT_SIZE / 2;
    let bin_hz = sr / FFT_SIZE as f64;
    let top_bin = ((MAX_FORMANT / bin_hz) as usize).min(half);
    let mag: Vec<f64> = (0..=top_bin).map(|i| buf[i].norm()).collect();

    // spectral centroid + rolloff (energy above 2 kHz) + tilt (dB/Hz slope)
    let mut num = 0.0;
    let mut den = 0.0;
    let mut e_total = 0.0;
    let mut e_high = 0.0;
    // linear regression of 20log10(mag) on freq
    let (mut sx, mut sy, mut sxx, mut sxy, mut cnt) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, &m) in mag.iter().enumerate() {
        let f = i as f64 * bin_hz;
        num += f * m;
        den += m;
        let e = m * m;
        e_total += e;
        if f > 2000.0 {
            e_high += e;
        }
        if f > 100.0 {
            let db = 20.0 * (m + 1e-12).log10();
            sx += f;
            sy += db;
            sxx += f * f;
            sxy += f * db;
            cnt += 1.0;
        }
    }
    if den <= 0.0 || cnt < 2.0 {
        return None;
    }
    let centroid = num / den;
    let rolloff = if e_total > 0.0 { e_high / e_total } else { 0.0 };
    let slope = (cnt * sxy - sx * sy) / (cnt * sxx - sx * sx).max(1e-9);
    let tilt = slope * 1000.0; // dB per kHz

    // H1-H2: magnitude at F0 vs 2*F0 (search ±2 bins for the harmonic peak)
    let peak = |target: f64| -> f64 {
        let b = (target / bin_hz).round() as i64;
        let lo = (b - 2).max(0) as usize;
        let hi = ((b + 2) as usize).min(top_bin);
        (lo..=hi).map(|i| mag[i]).fold(0.0, f64::max)
    };
    let h1 = peak(f0);
    let h2 = peak(2.0 * f0);
    let h1h2 = 20.0 * ((h1 + 1e-12) / (h2 + 1e-12)).log10();

    Some(SpectralFrame { centroid, tilt, rolloff, h1h2 })
}

#[derive(Clone, Copy)]
struct Sib {
    m1: f64,
    m2: f64,
    m3: f64,
    m4: f64,
    hi: f64,
    peak: f64,
}

fn sgn(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Sibilant-fricative spectral moments at native `sr`. Detects unvoiced,
/// high-frequency (fricative) frames and computes M1-M4 + >5 kHz energy fraction
/// + peak frequency over the 1-8 kHz band. Mirrors analysis/sibilant.py.
fn sibilant_features(x: &[f64], sr: f64) -> Option<Sib> {
    let w = (0.025 * sr).round() as usize;
    let hop = (0.010 * sr).round() as usize;
    if w < 32 || hop == 0 || x.len() < w + hop {
        return None;
    }
    let hann: Vec<f64> = (0..w)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (w as f64 - 1.0)).cos())
        .collect();
    let nfr = (x.len() - w) / hop + 1;
    let mut energies = vec![0.0f64; nfr];
    for (fr, e) in energies.iter_mut().enumerate() {
        let s = fr * hop;
        let mut acc = 0.0;
        for j in 0..w {
            let v = x[s + j] * hann[j];
            acc += v * v;
        }
        *e = acc;
    }
    let emax = energies.iter().cloned().fold(0.0, f64::max);
    if emax <= 0.0 {
        return None;
    }
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(w);
    let bin_hz = sr / w as f64;
    let nbins = w / 2 + 1;
    let lob = (1000.0 / bin_hz).ceil() as usize;
    let hib = ((8000.0 / bin_hz).floor() as usize).min(nbins - 1);
    if hib <= lob {
        return None;
    }

    let mut m1s = Vec::new();
    let mut m2s = Vec::new();
    let mut m3s = Vec::new();
    let mut m4s = Vec::new();
    let mut his = Vec::new();
    let mut pks = Vec::new();
    for fr in 0..nfr {
        let s = fr * hop;
        if energies[fr] < 0.02 * emax {
            continue;
        }
        // zero-crossing rate (unvoiced gate)
        let mut zc = 0.0;
        let mut prev = sgn(x[s]);
        for j in 1..w {
            let cur = sgn(x[s + j]);
            zc += (cur - prev).abs();
            prev = cur;
        }
        let zcr = zc / ((w - 1) as f64) / 2.0;
        if zcr <= 0.12 {
            continue;
        }
        // spectrum
        let mut buf: Vec<Complex<f64>> = (0..w).map(|j| Complex::new(x[s + j] * hann[j], 0.0)).collect();
        fft.process(&mut buf);
        let mut cog_num = 0.0;
        let mut cog_den = 0.0;
        let mut hi = 0.0;
        let mut lo = 0.0;
        for i in 0..nbins {
            let p = buf[i].norm_sqr();
            let f = i as f64 * bin_hz;
            cog_num += f * p;
            cog_den += p;
            if f >= 1000.0 {
                hi += p;
            } else {
                lo += p;
            }
        }
        if cog_den <= 0.0 || cog_num / cog_den <= 3000.0 || hi <= lo {
            continue; // not a high-frequency-dominant (sibilant) frame
        }
        // spectral moments over the 1-8 kHz band
        let mut psum = 0.0;
        let mut s1 = 0.0;
        for i in lob..=hib {
            let p = buf[i].norm_sqr();
            psum += p;
            s1 += (i as f64 * bin_hz) * p;
        }
        if psum <= 0.0 {
            continue;
        }
        let m1 = s1 / psum;
        let (mut s2, mut s3, mut s4, mut ph, mut pk, mut pf) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for i in lob..=hib {
            let f = i as f64 * bin_hz;
            let p = buf[i].norm_sqr();
            let d = f - m1;
            s2 += d * d * p;
            s3 += d * d * d * p;
            s4 += d * d * d * d * p;
            if f >= 5000.0 {
                ph += p;
            }
            if p > pk {
                pk = p;
                pf = f;
            }
        }
        let m2 = (s2 / psum).sqrt();
        m1s.push(m1);
        m2s.push(m2);
        m3s.push((s3 / psum) / (m2.powi(3) + 1e-12));
        m4s.push((s4 / psum) / (m2.powi(4) + 1e-12) - 3.0);
        his.push(ph / psum);
        pks.push(pf);
    }
    if m1s.len() < 5 {
        return None;
    }
    Some(Sib {
        m1: median(&mut m1s)?,
        m2: median(&mut m2s)?,
        m3: median(&mut m3s)?,
        m4: median(&mut m4s)?,
        hi: median(&mut his)?,
        peak: median(&mut pks)?,
    })
}

// ── complex helpers + Durand-Kerner root finding ─────────────────────────────
type Cx = (f64, f64);
fn cmul(a: Cx, b: Cx) -> Cx {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
fn cdiv(a: Cx, b: Cx) -> Cx {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}
fn csub(a: Cx, b: Cx) -> Cx {
    (a.0 - b.0, a.1 - b.1)
}

/// All roots of a monic polynomial given coeffs in descending order
/// (coeffs[0] == 1). Weierstrass/Durand-Kerner iteration.
fn durand_kerner(coeffs: &[f64]) -> Vec<Cx> {
    let n = coeffs.len() - 1; // degree
    if n == 0 {
        return Vec::new();
    }
    let eval = |z: Cx| -> Cx {
        let mut r = (coeffs[0], 0.0);
        for &c in &coeffs[1..] {
            r = cmul(r, z);
            r.0 += c;
        }
        r
    };
    // spread initial guesses around a circle (classic 0.4+0.9i seed powers)
    let seed = (0.4, 0.9);
    let mut roots: Vec<Cx> = Vec::with_capacity(n);
    let mut p = (1.0, 0.0);
    for _ in 0..n {
        p = cmul(p, seed);
        roots.push(p);
    }
    for _ in 0..60 {
        let mut max_delta = 0.0f64;
        for i in 0..n {
            let num = eval(roots[i]);
            let mut den = (1.0, 0.0);
            for j in 0..n {
                if i != j {
                    den = cmul(den, csub(roots[i], roots[j]));
                }
            }
            let delta = cdiv(num, den);
            roots[i] = csub(roots[i], delta);
            max_delta = max_delta.max(delta.0.abs() + delta.1.abs());
        }
        if max_delta < 1e-12 {
            break;
        }
    }
    roots
}

/// Burg's method (Numerical Recipes `memcof`). Returns `d[0..m]` such that the
/// predictor is x[i] = Σ d[j]·x[i-j]; these are also the companion top row for
/// the AR polynomial A(z)=1-Σ d[j] z^-j.
fn burg_lpc(data: &[f64], m: usize) -> Vec<f64> {
    let n = data.len();
    let mut d = vec![0.0f64; m];
    let mut wkm = vec![0.0f64; m];
    let mut wk1 = vec![0.0f64; n];
    let mut wk2 = vec![0.0f64; n];
    for j in 0..(n - 1) {
        wk1[j] = data[j];
        wk2[j] = data[j + 1];
    }
    for k in 1..=m {
        let mut num = 0.0;
        let mut den = 0.0;
        for j in 0..(n - k) {
            num += wk1[j] * wk2[j];
            den += wk1[j] * wk1[j] + wk2[j] * wk2[j];
        }
        let dk = if den != 0.0 { 2.0 * num / den } else { 0.0 };
        d[k - 1] = dk;
        for i in 1..k {
            d[i - 1] = wkm[i - 1] - dk * wkm[k - i - 1];
        }
        if k == m {
            break;
        }
        for i in 0..k {
            wkm[i] = d[i];
        }
        for j in 0..(n - k - 1) {
            wk1[j] -= wkm[k - 1] * wk2[j];
            wk2[j] = wk2[j + 1] - wkm[k - 1] * wk1[j + 1];
        }
    }
    d
}

// ── resampling (windowed-sinc, arbitrary ratio) ──────────────────────────────
fn resample(x: &[f64], from_sr: f64, to_sr: f64) -> Vec<f64> {
    if (from_sr - to_sr).abs() < 1.0 || x.is_empty() {
        return x.to_vec();
    }
    let ratio = to_sr / from_sr;
    let out_len = ((x.len() as f64) * ratio).floor() as usize;
    let cutoff = 0.5 * ratio.min(1.0); // normalized to input rate
    const TAPS: i64 = 16;
    let mut out = vec![0.0f64; out_len];
    for (i, o) in out.iter_mut().enumerate() {
        let center = (i as f64) / ratio; // position in input samples
        let c0 = center.floor() as i64;
        let mut acc = 0.0;
        let mut wsum = 0.0;
        for k in (c0 - TAPS + 1)..=(c0 + TAPS) {
            if k < 0 || k as usize >= x.len() {
                continue;
            }
            let t = center - k as f64;
            let s = sinc(2.0 * cutoff * t) * 2.0 * cutoff;
            // Hann taper over the tap window
            let win = 0.5 + 0.5 * ((std::f64::consts::PI * t / TAPS as f64).cos());
            let wv = s * win;
            acc += x[k as usize] * wv;
            wsum += wv;
        }
        *o = if wsum != 0.0 { acc / wsum } else { 0.0 };
    }
    out
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 {
        1.0
    } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}
