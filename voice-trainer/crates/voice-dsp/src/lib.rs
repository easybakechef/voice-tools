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
    if max_period < min_period + 2 {
        return 0.0;
    }

    // NSDF across the search range.
    let n = max_period - min_period + 1;
    let mut r = vec![0f32; n];
    for t in min_period..=max_period {
        r[t - min_period] = nsdf(samples, t);
    }

    // Proper McLeod method: octave errors come from picking the *tallest* peak
    // (often the period-doubled one). Instead pick the FIRST local maximum that
    // is within 88% of the tallest.
    let mut max_val = f32::NEG_INFINITY;
    let mut peaks: Vec<usize> = Vec::new();
    for i in 1..n - 1 {
        if r[i] > r[i - 1] && r[i] >= r[i + 1] {
            peaks.push(i);
            if r[i] > max_val {
                max_val = r[i];
            }
        }
    }
    if peaks.is_empty() || max_val < CLARITY_THRESHOLD {
        return 0.0;
    }

    let thresh = 0.88 * max_val;
    let mut chosen = peaks[0];
    for &p in &peaks {
        if r[p] >= thresh {
            chosen = p;
            break;
        }
    }

    // Parabolic interpolation around the chosen peak.
    let a = if chosen > 0 { r[chosen - 1] } else { r[chosen] };
    let b = r[chosen];
    let c = if chosen < n - 1 { r[chosen + 1] } else { r[chosen] };
    let denom = a - 2.0 * b + c;
    let offset = if denom.abs() > 1e-6 { -0.5 * (a - c) / denom } else { 0.0 };
    let tau = (min_period + chosen) as f32 + offset;

    if tau > 0.0 {
        sample_rate / tau
    } else {
        0.0
    }
}
