use wasm_bindgen::prelude::*;

const MIN_HZ: f32 = 80.0;
const MAX_HZ: f32 = 350.0;
const CLARITY_THRESHOLD: f32 = 0.4;
const SILENCE_RMS: f64 = 0.01;

/// Normalized Square Difference Function at lag `tau`.
fn nsdf(buf: &[f32], tau: usize) -> f32 {
    let n = buf.len().saturating_sub(tau);
    if n == 0 {
        return 0.0;
    }
    let mut num = 0.0f64;
    let mut den = 0.0f64;
    for i in 0..n {
        let x = buf[i] as f64;
        let y = buf[i + tau] as f64;
        num += x * y;
        den += x * x + y * y;
    }
    if den == 0.0 {
        0.0
    } else {
        (2.0 * num / den) as f32
    }
}

/// Detect the fundamental pitch of `samples` at `sample_rate` Hz.
///
/// Uses NSDF (McLeod pitch method) with parabolic interpolation for
/// sub-sample accuracy. Returns `0.0` when no pitch is detected
/// (silence or unclear signal); otherwise returns the frequency in Hz.
#[wasm_bindgen]
pub fn detect_pitch(samples: &[f32], sample_rate: f32) -> f32 {
    // Silence gate
    let rms = (samples.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>()
        / samples.len() as f64)
        .sqrt();
    if rms < SILENCE_RMS {
        return 0.0;
    }

    let min_period = (sample_rate / MAX_HZ).floor() as usize;
    let max_period = ((sample_rate / MIN_HZ).ceil() as usize).min(samples.len() / 2);

    let mut best_tau = 0usize;
    let mut best_val = f32::NEG_INFINITY;

    for tau in min_period..=max_period {
        let r = nsdf(samples, tau);
        if r > best_val {
            best_val = r;
            best_tau = tau;
        }
    }

    if best_val < CLARITY_THRESHOLD || best_tau == 0 {
        return 0.0;
    }

    // Parabolic interpolation for sub-sample period accuracy
    let a = if best_tau > 0 { nsdf(samples, best_tau - 1) } else { 0.0 };
    let b = best_val;
    let c = if best_tau < samples.len() / 2 {
        nsdf(samples, best_tau + 1)
    } else {
        0.0
    };

    let denom = a - 2.0 * b + c;
    let refined = if denom.abs() > 1e-6 {
        best_tau as f32 - 0.5 * (a - c) / denom
    } else {
        best_tau as f32
    };

    sample_rate / refined
}
