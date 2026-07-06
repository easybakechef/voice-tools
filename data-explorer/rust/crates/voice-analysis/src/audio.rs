//! Minimal audio decode → mono f32 + sample rate. flac (claxon), wav (hound).

use anyhow::{bail, Context, Result};
use std::path::Path;

pub fn load(path: &Path) -> Result<(Vec<f32>, u32)> {
    match path.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()) {
        Some(ref e) if e == "flac" => load_flac(path),
        Some(ref e) if e == "wav" => load_wav(path),
        other => bail!("unsupported audio extension: {:?}", other),
    }
}

fn load_flac(path: &Path) -> Result<(Vec<f32>, u32)> {
    let reader = claxon::FlacReader::open(path)
        .with_context(|| format!("open flac {}", path.display()))?;
    decode_flac(reader)
}

/// Decode FLAC from an in-memory byte slice (for streamed tarball scanning).
pub fn decode_flac_bytes(bytes: &[u8]) -> Result<(Vec<f32>, u32)> {
    let reader = claxon::FlacReader::new(std::io::Cursor::new(bytes))?;
    decode_flac(reader)
}

fn decode_flac<R: std::io::Read>(mut reader: claxon::FlacReader<R>) -> Result<(Vec<f32>, u32)> {
    let info = reader.streaminfo();
    let ch = info.channels as usize;
    let sr = info.sample_rate;
    let scale = 1.0f32 / ((1i64 << (info.bits_per_sample - 1)) as f32);
    let raw: Vec<i32> = reader.samples().collect::<std::result::Result<_, _>>()?;
    Ok((downmix_i32(&raw, ch, scale), sr))
}

fn load_wav(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("open wav {}", path.display()))?;
    let spec = reader.spec();
    let ch = spec.channels as usize;
    let sr = spec.sample_rate;
    let mono: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            let raw: Vec<f32> = reader.samples::<f32>().collect::<std::result::Result<_, _>>()?;
            downmix_f32(&raw, ch)
        }
        hound::SampleFormat::Int => {
            let scale = 1.0f32 / ((1i64 << (spec.bits_per_sample - 1)) as f32);
            let raw: Vec<i32> = reader.samples::<i32>().collect::<std::result::Result<_, _>>()?;
            downmix_i32(&raw, ch, scale)
        }
    };
    Ok((mono, sr))
}

fn downmix_i32(raw: &[i32], ch: usize, scale: f32) -> Vec<f32> {
    if ch <= 1 {
        return raw.iter().map(|&s| s as f32 * scale).collect();
    }
    raw.chunks(ch)
        .map(|c| c.iter().map(|&s| s as f32).sum::<f32>() / ch as f32 * scale)
        .collect()
}

fn downmix_f32(raw: &[f32], ch: usize) -> Vec<f32> {
    if ch <= 1 {
        return raw.to_vec();
    }
    raw.chunks(ch).map(|c| c.iter().sum::<f32>() / ch as f32).collect()
}
