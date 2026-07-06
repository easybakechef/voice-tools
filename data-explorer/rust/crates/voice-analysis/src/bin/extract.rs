//! Batch-extract F0+F1-F4 per kept clip, aggregate to per-speaker medians.
//! Mirrors analysis/extract_kept_formants.py (cap 4 clips/speaker).
//!   cargo run --release --bin extract -- <kept_dir> <speakers.txt> <out.csv> [cap]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use voice_analysis::{analyze, audio, Features};

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 3 {
        eprintln!("usage: extract <kept_dir> <speakers.txt> <out.csv> [cap]");
        std::process::exit(2);
    }
    let (kept_dir, speakers_txt, out_csv) = (&a[0], &a[1], &a[2]);
    let cap: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);

    let gmap = gender_map(Path::new(speakers_txt))?;

    // group flac clips by speaker id (filename: ls-<spk>-<chap>-<utt>-<idx>.flac)
    let mut by_spk: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for entry in std::fs::read_dir(kept_dir)? {
        let p = entry?.path();
        if p.extension().and_then(|e| e.to_str()) != Some("flac") {
            continue;
        }
        let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parts: Vec<&str> = name.split('-').collect();
        if parts.len() < 2 {
            continue;
        }
        by_spk.entry(parts[1].to_string()).or_default().push(p);
    }

    let speakers: Vec<(String, Vec<PathBuf>)> = by_spk
        .into_iter()
        .map(|(spk, mut clips)| {
            clips.sort();
            clips.truncate(cap);
            (spk, clips)
        })
        .collect();
    let total = speakers.len();
    eprintln!("extracting {total} speakers ({} threads) …", rayon::current_num_threads());

    let done = std::sync::atomic::AtomicUsize::new(0);
    let mut rows: Vec<String> = speakers
        .par_iter()
        .filter_map(|(spk, clips)| {
            let feats: Vec<Features> = clips
                .iter()
                .filter_map(|c| audio::load(c).ok().map(|(s, sr)| analyze(&s, sr)))
                .filter(|f| f.f0.is_some())
                .collect();
            let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if n % 50 == 0 {
                eprintln!("  {n}/{total}");
            }
            if feats.is_empty() {
                return None;
            }
            let gender = gmap.get(spk).cloned().unwrap_or_default();
            let f0 = agg(&feats, |f| f.f0);
            let f1 = agg(&feats, |f| f.f1);
            let f2 = agg(&feats, |f| f.f2);
            let f3 = agg(&feats, |f| f.f3);
            let f4 = agg(&feats, |f| f.f4);
            Some(format!(
                "ls-{spk},{gender},{},{},{},{},{},{}",
                feats.len(), cell(f0), cell(f1), cell(f2), cell(f3), cell(f4)
            ))
        })
        .collect();
    rows.sort();

    let mut buf = String::from("speaker,gender,n_clips,f0,f1,f2,f3,f4\n");
    for r in &rows {
        buf.push_str(r);
        buf.push('\n');
    }
    std::fs::write(out_csv, buf)?;
    eprintln!("wrote {} speakers → {out_csv}", rows.len());
    Ok(())
}

fn agg(feats: &[Features], get: impl Fn(&Features) -> Option<f64>) -> Option<f64> {
    let mut v: Vec<f64> = feats.iter().filter_map(&get).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    Some(if n % 2 == 1 { v[n / 2] } else { 0.5 * (v[n / 2 - 1] + v[n / 2]) })
}

fn cell(v: Option<f64>) -> String {
    v.map(|x| format!("{x}")).unwrap_or_default()
}

fn gender_map(path: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with(';') {
            continue;
        }
        let p: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        if p.len() >= 2 && p[0].chars().all(|c| c.is_ascii_digit()) && !p[0].is_empty() {
            let g = match p[1] {
                "F" => "female",
                "M" => "male",
                other => other,
            };
            out.insert(p[0].to_string(), g.to_string());
        }
    }
    Ok(out)
}
