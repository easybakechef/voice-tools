//! Voice analysis DSP: audio decode, NSDF pitch, Burg-LPC formants, VTL metric.
//!
//! Formant extraction mirrors Praat's `to_formant_burg`: resample to
//! 2*max_formant, pre-emphasis, short Gaussian window, Burg LPC of order
//! 2*n_formants, then formants from the roots of the prediction polynomial.
//! Aggregation matches `analysis/extract_kept_formants.py`: median of each
//! formant over voiced frames.

pub mod audio;

pub const C_CM: f64 = 35000.0; // speed of sound, cm/s (Fitch convention)
pub const MAX_FORMANT: f64 = 5500.0;
const RESAMPLE_SR: f64 = 2.0 * MAX_FORMANT; // 11025-ish target; we use exactly 2*Fmax
const N_FORMANTS: usize = 5;
const LPC_ORDER: usize = 2 * N_FORMANTS; // 10
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

    let mut f0s: Vec<f64> = Vec::new();
    let mut cols: [Vec<f64>; 4] = Default::default();

    let mut center = half_p.max(half_f);
    let end = rs.len().saturating_sub(half_p.max(half_f) + 1);
    while center < end {
        let pbuf = &rs[center - half_p..center + half_p];
        if let Some((f0, clarity)) = detect_pitch(pbuf, fsr) {
            if clarity >= CLARITY_MIN && (PITCH_FLOOR..=PITCH_CEILING).contains(&f0) {
                f0s.push(f0);
                let fbuf = &rs[center - half_f..center + half_f];
                let fmts = formants_for_frame(fbuf, fsr);
                for (k, col) in cols.iter_mut().enumerate() {
                    if let Some(&v) = fmts.get(k) {
                        col.push(v);
                    }
                }
            }
        }
        center += hop;
    }

    if f0s.len() < 5 {
        return Features::default();
    }
    Features {
        f0: median(&mut f0s.clone()),
        f1: median(&mut cols[0].clone()),
        f2: median(&mut cols[1].clone()),
        f3: median(&mut cols[2].clone()),
        f4: median(&mut cols[3].clone()),
        voiced_frames: f0s.len(),
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

// ── formants (Burg LPC → polynomial roots) ───────────────────────────────────
fn formants_for_frame(frame: &[f64], sr: f64) -> Vec<f64> {
    let n = frame.len();
    if n < LPC_ORDER + 4 {
        return Vec::new();
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
    fmts
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
