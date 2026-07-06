//! Resonance metric + hard-case computation, written into samples.db.
//!
//! Reads per-speaker features (speaker,gender,n_clips,f0,f1,f2,f3,f4), computes
//! VTL, builds the pitch-matched set, fits a logistic model on VTL with 5-fold
//! CV, and writes:
//!   resonance(speaker, gender, f0..f4, vtl, prob_female, pred, correct, margin, in_matched)
//!   resonance_meta(auc, full_auc, threshold, matched_acc)
//!
//!   metric <features.csv> <samples.db>

use anyhow::{bail, Result};
use rusqlite::Connection;

const C_CM: f64 = 35000.0;
const TOL: f64 = 6.0;

#[derive(Clone)]
struct Spk {
    speaker: String,
    gender: String,
    f0: f64,
    f1: f64,
    f2: f64,
    f3: f64,
    f4: f64,
    vtl: f64,
    y: f64,
    in_matched: bool,
}

fn vtl(f1: f64, f2: f64, f3: f64, f4: f64) -> f64 {
    let f = [f1, f2, f3, f4];
    let ns = [1.0, 3.0, 5.0, 7.0];
    let num: f64 = ns.iter().zip(f).map(|(n, x)| n * x).sum();
    let den: f64 = ns.iter().map(|n| n * n).sum();
    C_CM / (4.0 * (num / den))
}

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 2 {
        bail!("usage: metric <features.csv> <samples.db>");
    }
    let mut rows = load_csv(&a[0])?;
    pitch_match(&mut rows);

    let n = rows.len();
    let nm = rows.iter().filter(|r| r.y == 0.0).count();
    let nf = n - nm;
    let matched: Vec<&Spk> = rows.iter().filter(|r| r.in_matched).collect();
    eprintln!(
        "plausible speakers: {n} ({nm}M / {nf}F); pitch-matched: {} ({}M / {}F)",
        matched.len(),
        matched.iter().filter(|r| r.y == 0.0).count(),
        matched.iter().filter(|r| r.y == 1.0).count()
    );

    // features: standardized VTL
    let x: Vec<f64> = rows.iter().map(|r| r.vtl).collect();
    let y: Vec<f64> = rows.iter().map(|r| r.y).collect();
    let (mean, std) = mean_std(&x);
    let z: Vec<Vec<f64>> = x.iter().map(|v| vec![(v - mean) / std]).collect();

    // full-pool model (for threshold + storing per-speaker probs via CV)
    let w = logistic_irls(&z, &y);
    let probs = cv_probs(&z, &y, 5);

    // AUC on matched subset and full pool (VTL, cross-validated)
    let midx: Vec<usize> = (0..n).filter(|&i| rows[i].in_matched).collect();
    let matched_probs: Vec<f64> = midx.iter().map(|&i| probs[i]).collect();
    let matched_y: Vec<f64> = midx.iter().map(|&i| y[i]).collect();
    let auc_matched = auc(&matched_probs, &matched_y);
    let full_auc = auc(&probs, &y);
    let matched_acc = midx
        .iter()
        .filter(|&&i| (probs[i] >= 0.5) == (y[i] == 1.0))
        .count() as f64
        / midx.len().max(1) as f64;

    // VTL threshold where full-model prob = 0.5:  w0 + w1*z = 0
    let threshold = if w[1].abs() > 1e-9 {
        mean + (-w[0] / w[1]) * std
    } else {
        mean
    };

    eprintln!(
        "VTL AUC matched={auc_matched:.3} full={full_auc:.3} matched_acc={:.1}% threshold={threshold:.2}cm",
        matched_acc * 100.0
    );

    write_db(&a[1], &rows, &probs, auc_matched, full_auc, threshold, matched_acc)?;
    eprintln!("wrote resonance table for {n} speakers → {}", a[1]);
    Ok(())
}

fn load_csv(path: &str) -> Result<Vec<Spk>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            continue; // header
        }
        let c: Vec<&str> = line.split(',').collect();
        if c.len() < 8 {
            continue;
        }
        let g = c[1];
        if g != "male" && g != "female" {
            continue;
        }
        let p = |s: &str| s.trim().parse::<f64>().ok();
        let (f0, f1, f2, f3, f4) = match (p(c[3]), p(c[4]), p(c[5]), p(c[6]), p(c[7])) {
            (Some(a), Some(b), Some(cc), Some(d), Some(e)) => (a, b, cc, d, e),
            _ => continue,
        };
        // plausibility filter (mirrors resonance_metric.load)
        if !(200.0..=1100.0).contains(&f1) || !(f2 > f1 && f3 > f2 && f4 > f3) {
            continue;
        }
        if !(80.0..=320.0).contains(&f0) {
            continue;
        }
        let v = vtl(f1, f2, f3, f4);
        if !(8.0..=22.0).contains(&v) {
            continue;
        }
        out.push(Spk {
            speaker: c[0].to_string(),
            gender: g.to_string(),
            f0,
            f1,
            f2,
            f3,
            f4,
            vtl: v,
            y: if g == "female" { 1.0 } else { 0.0 },
            in_matched: false,
        });
    }
    Ok(out)
}

/// Greedy nearest-pitch M↔F matching within TOL Hz (mirrors pitch_match).
fn pitch_match(rows: &mut [Spk]) {
    let mut men: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].y == 0.0).collect();
    let mut women: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].y == 1.0).collect();
    men.sort_by(|&a, &b| rows[a].f0.partial_cmp(&rows[b].f0).unwrap());
    women.sort_by(|&a, &b| rows[a].f0.partial_cmp(&rows[b].f0).unwrap());
    let mut used = vec![false; rows.len()];
    for &m in &men {
        let mf0 = rows[m].f0;
        let mut best: Option<usize> = None;
        let mut bestd = f64::MAX;
        for &wi in &women {
            if used[wi] {
                continue;
            }
            let d = (rows[wi].f0 - mf0).abs();
            if d < bestd {
                bestd = d;
                best = Some(wi);
            }
        }
        if let Some(wi) = best {
            if bestd <= TOL {
                used[wi] = true;
                rows[m].in_matched = true;
                rows[wi].in_matched = true;
            }
        }
    }
}

fn mean_std(x: &[f64]) -> (f64, f64) {
    let m = x.iter().sum::<f64>() / x.len() as f64;
    let var = x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / x.len() as f64;
    (m, var.sqrt().max(1e-9))
}

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

/// Logistic regression via IRLS. Returns weights [bias, w1..wp].
fn logistic_irls(feats: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let n = feats.len();
    let p = feats[0].len();
    let dim = p + 1;
    let mut w = vec![0.0; dim];
    for _ in 0..25 {
        // build X^T W X (dim x dim) and X^T (y-mu) (dim)
        let mut ata = vec![vec![0.0; dim]; dim];
        let mut atb = vec![0.0; dim];
        for i in 0..n {
            let mut xi = Vec::with_capacity(dim);
            xi.push(1.0);
            xi.extend_from_slice(&feats[i]);
            let eta: f64 = xi.iter().zip(&w).map(|(a, b)| a * b).sum();
            let mu = sigmoid(eta);
            let wt = (mu * (1.0 - mu)).max(1e-6);
            for r in 0..dim {
                atb[r] += xi[r] * (y[i] - mu);
                for c in 0..dim {
                    ata[r][c] += xi[r] * wt * xi[c];
                }
            }
        }
        // ridge for stability
        for d in 0..dim {
            ata[d][d] += 1e-6;
        }
        let delta = solve(&ata, &atb);
        let mut change = 0.0;
        for d in 0..dim {
            w[d] += delta[d];
            change += delta[d].abs();
        }
        if change < 1e-8 {
            break;
        }
    }
    w
}

/// Gaussian elimination solve A x = b (A small, square).
fn solve(a: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut m: Vec<Vec<f64>> = a.iter().map(|r| r.clone()).collect();
    let mut x = b.to_vec();
    for col in 0..n {
        let mut piv = col;
        for r in (col + 1)..n {
            if m[r][col].abs() > m[piv][col].abs() {
                piv = r;
            }
        }
        m.swap(col, piv);
        x.swap(col, piv);
        let d = m[col][col];
        if d.abs() < 1e-12 {
            continue;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let f = m[r][col] / d;
            for c in col..n {
                m[r][c] -= f * m[col][c];
            }
            x[r] -= f * x[col];
        }
    }
    for i in 0..n {
        if m[i][i].abs() > 1e-12 {
            x[i] /= m[i][i];
        }
    }
    x
}

/// Out-of-fold probabilities via k-fold CV (deterministic striped folds).
fn cv_probs(feats: &[Vec<f64>], y: &[f64], k: usize) -> Vec<f64> {
    let n = feats.len();
    let fold: Vec<usize> = (0..n).map(|i| i % k).collect();
    let mut probs = vec![0.5; n];
    for f in 0..k {
        let tr_x: Vec<Vec<f64>> = (0..n).filter(|&i| fold[i] != f).map(|i| feats[i].clone()).collect();
        let tr_y: Vec<f64> = (0..n).filter(|&i| fold[i] != f).map(|i| y[i]).collect();
        if tr_y.iter().all(|&v| v == tr_y[0]) {
            continue;
        }
        let w = logistic_irls(&tr_x, &tr_y);
        for i in 0..n {
            if fold[i] == f {
                let mut eta = w[0];
                for (j, v) in feats[i].iter().enumerate() {
                    eta += w[j + 1] * v;
                }
                probs[i] = sigmoid(eta);
            }
        }
    }
    probs
}

/// ROC AUC via the Mann-Whitney U statistic (rank-based).
fn auc(scores: &[f64], y: &[f64]) -> f64 {
    let mut idx: Vec<usize> = (0..scores.len()).collect();
    idx.sort_by(|&a, &b| scores[a].partial_cmp(&scores[b]).unwrap());
    // average ranks (handle ties)
    let mut rank = vec![0.0; scores.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && scores[idx[j + 1]] == scores[idx[i]] {
            j += 1;
        }
        let r = (i + j) as f64 / 2.0 + 1.0;
        for t in i..=j {
            rank[idx[t]] = r;
        }
        i = j + 1;
    }
    let npos: f64 = y.iter().filter(|&&v| v == 1.0).count() as f64;
    let nneg = scores.len() as f64 - npos;
    if npos == 0.0 || nneg == 0.0 {
        return 0.5;
    }
    let sum_pos: f64 = (0..scores.len()).filter(|&i| y[i] == 1.0).map(|i| rank[i]).sum();
    (sum_pos - npos * (npos + 1.0) / 2.0) / (npos * nneg)
}

#[allow(clippy::too_many_arguments)]
fn write_db(
    db: &str,
    rows: &[Spk],
    probs: &[f64],
    auc_matched: f64,
    full_auc: f64,
    threshold: f64,
    matched_acc: f64,
) -> Result<()> {
    let mut conn = Connection::open(db)?;
    conn.execute_batch(
        "DROP TABLE IF EXISTS resonance;
         DROP TABLE IF EXISTS resonance_meta;
         CREATE TABLE resonance (
            speaker text primary key, gender text,
            f0 real, f1 real, f2 real, f3 real, f4 real, vtl real,
            prob_female real, pred text, correct integer, margin real, in_matched integer);
         CREATE TABLE resonance_meta (auc real, full_auc real, threshold real, matched_acc real);
         CREATE INDEX idx_res_margin ON resonance(margin);
         CREATE INDEX idx_res_correct ON resonance(correct);",
    )?;
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO resonance VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )?;
        for (i, r) in rows.iter().enumerate() {
            let prob = probs[i];
            let pred = if prob >= 0.5 { "female" } else { "male" };
            let correct = ((prob >= 0.5) == (r.y == 1.0)) as i64;
            let margin = (prob - 0.5).abs();
            stmt.execute(rusqlite::params![
                r.speaker, r.gender, r.f0, r.f1, r.f2, r.f3, r.f4, r.vtl,
                prob, pred, correct, margin, r.in_matched as i64
            ])?;
        }
        tx.execute(
            "INSERT INTO resonance_meta VALUES (?,?,?,?)",
            rusqlite::params![auc_matched, full_auc, threshold, matched_acc],
        )?;
    }
    tx.commit()?;
    Ok(())
}
