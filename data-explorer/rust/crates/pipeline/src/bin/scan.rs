//! Wide pitch scan of LibriSpeech subsets — Rust port of pipeline/scan.py.
//!
//! Streams a subset tarball over HTTP, decodes each flac in memory, computes
//! pitch-only features, and indexes every sample in samples.db. Audio is kept on
//! disk ONLY for pitch-crossover voices (female F0 < 180, male F0 > 140) — for
//! those the original flac bytes are written verbatim (no re-encode).
//!
//!   scan <subset> <data_dir> [cap_per_speaker]
//! e.g. scan train-clean-100 ../data 12

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use rusqlite::Connection;
use voice_analysis::{audio, pitch_features};

const FEMALE_MAX: f64 = 180.0;
const MALE_MIN: f64 = 140.0;

fn url_for(subset: &str) -> Option<&'static str> {
    Some(match subset {
        "dev-clean" => "https://www.openslr.org/resources/12/dev-clean.tar.gz",
        "test-clean" => "https://www.openslr.org/resources/12/test-clean.tar.gz",
        "dev-other" => "https://www.openslr.org/resources/12/dev-other.tar.gz",
        "test-other" => "https://www.openslr.org/resources/12/test-other.tar.gz",
        "train-clean-100" => "https://www.openslr.org/resources/12/train-clean-100.tar.gz",
        "train-clean-360" => "https://www.openslr.org/resources/12/train-clean-360.tar.gz",
        "train-other-500" => "https://www.openslr.org/resources/12/train-other-500.tar.gz",
        _ => return None,
    })
}

fn keep_audio(gender: Option<&str>, f0: Option<f64>) -> bool {
    match (gender, f0) {
        (Some("female"), Some(f)) => f < FEMALE_MAX,
        (Some("male"), Some(f)) => f > MALE_MIN,
        _ => false,
    }
}

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 2 {
        bail!("usage: scan <subset> <data_dir> [cap_per_speaker]");
    }
    let subset = &a[0];
    let data_dir = PathBuf::from(&a[1]);
    let cap: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(12);
    let url = url_for(subset).ok_or_else(|| anyhow::anyhow!("unknown subset {subset}"))?;

    let kept_dir = data_dir.join("kept");
    std::fs::create_dir_all(&kept_dir)?;
    let speakers = data_dir.join("raw/LibriSpeech/SPEAKERS.TXT");
    if !speakers.exists() {
        bail!(
            "SPEAKERS.TXT missing at {} — ingest a small subset first",
            speakers.display()
        );
    }
    let gmap = gender_map(&speakers)?;
    let mut con = Connection::open(data_dir.join("samples.db"))?;
    con.execute_batch(SCHEMA)?;
    con.execute_batch("PRAGMA busy_timeout=60000;")?;

    eprintln!("Streaming {subset} …");
    let resp = ureq::get(url).call().context("http get")?;
    let gz = GzDecoder::new(resp.into_reader());
    let mut ar = tar::Archive::new(gz);

    let mut per_spk: HashMap<String, usize> = HashMap::new();
    let (mut n, mut kept) = (0usize, 0usize);
    let tx = con.transaction()?;
    for entry in ar.entries()? {
        let mut entry = entry?;
        let name = entry.path()?.to_string_lossy().into_owned();
        if !name.ends_with(".flac") {
            continue;
        }
        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() < 4 {
            continue;
        }
        let spk = parts[2].to_string();
        if per_spk.get(&spk).copied().unwrap_or(0) >= cap {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let (samples, sr) = match audio::decode_flac_bytes(&bytes) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let feat = pitch_features(&samples, sr);
        *per_spk.entry(spk.clone()).or_insert(0) += 1;
        let gender = gmap.get(&spk).map(|s| s.as_str());
        let base = Path::new(&name).file_name().unwrap().to_string_lossy();
        let keep = keep_audio(gender, feat.f0_median);
        let path = if keep {
            let dst = kept_dir.join(format!("ls-{spk}-{base}"));
            std::fs::write(&dst, &bytes)?; // original flac bytes, verbatim
            kept += 1;
            std::fs::canonicalize(&dst)?.to_string_lossy().into_owned()
        } else {
            format!("discarded://librispeech/{base}")
        };
        tx.execute(
            "INSERT OR IGNORE INTO samples
             (dataset,speaker,gender,path,duration,f0_median,voiced_frac,f1_median,f2_median,kept)
             VALUES ('librispeech',?,?,?,?,?,?,NULL,NULL,?)",
            rusqlite::params![
                format!("ls-{spk}"),
                gender,
                path,
                feat.duration,
                feat.f0_median,
                feat.voiced_frac,
                keep as i64
            ],
        )?;
        n += 1;
        if n % 1000 == 0 {
            eprintln!("  {n} scanned · {kept} kept · {} speakers", per_spk.len());
        }
    }
    tx.commit()?;
    eprintln!(
        "Done {subset}: {n} samples indexed, {kept} kept ({} speakers).",
        per_spk.len()
    );
    Ok(())
}

fn gender_map(path: &Path) -> Result<HashMap<String, String>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = HashMap::new();
    for line in text.lines() {
        if line.starts_with(';') {
            continue;
        }
        let p: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        if p.len() >= 2 && !p[0].is_empty() && p[0].chars().all(|c| c.is_ascii_digit()) {
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

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS samples (
    id integer primary key autoincrement,
    dataset text not null, speaker text not null, gender text, path text not null,
    duration real, f0_median real, voiced_frac real, f1_median real, f2_median real,
    kept integer default 1,
    unique(dataset, path));
";
