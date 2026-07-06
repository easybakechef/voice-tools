//! Resonance metric + hard-case computation, written into samples.db.
//!
//! Reads the wide per-speaker feature CSV (f0,f1..f5,vtl,c1..c12,centroid,tilt,
//! rolloff,h1h2), builds the pitch-matched set, and fits logistic models:
//!   - VTL only              (interpretable single-number resonance, a threshold)
//!   - rich resonance        (F1-F5 + VTL + LPC-cepstrum + spectral; NO pitch)
//!   - combo                 (F0 + rich resonance)
//! Writes per-speaker predictions for the rich-resonance (primary) and combo
//! models into the `resonance` table, plus AUCs into `resonance_meta`.
//!
//!   metric <rich_features.csv> <samples.db>

use std::collections::HashMap;

use anyhow::{bail, Result};
use rusqlite::Connection;

const TOL: f64 = 6.0;

// feature groups
const LPCC: [&str; 12] = ["c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9", "c10", "c11", "c12"];
const SPEC: [&str; 4] = ["centroid", "tilt", "rolloff", "h1h2"];
// pitch-independent formant-dynamics (VISC / movement over time)
const DYN: [&str; 3] = ["traj_f12", "spec_rate", "f2_range"];
// sibilant spectral moments — optional (mean-imputed where fricatives too sparse)
const SIB: [&str; 6] = ["sib_m1", "sib_m2", "sib_m3", "sib_m4", "sib_hi", "sib_peak"];

struct Row {
    speaker: String,
    gender: String,
    y: f64,
    in_matched: bool,
    f: HashMap<String, f64>,
}

fn resonance_feats() -> Vec<String> {
    let mut v: Vec<String> = ["f1", "f2", "f3", "f4", "f5", "vtl"].iter().map(|s| s.to_string()).collect();
    v.extend(LPCC.iter().map(|s| s.to_string()));
    v.extend(SPEC.iter().map(|s| s.to_string()));
    v.extend(DYN.iter().map(|s| s.to_string())); // formant-dynamics (pitch-independent)
    v
}

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 2 {
        bail!("usage: metric <rich_features.csv> <samples.db>");
    }
    let mut rows = load_wide(&a[0])?;
    pitch_match(&mut rows);

    let n = rows.len();
    let nm = rows.iter().filter(|r| r.y == 0.0).count();
    let midx: Vec<usize> = (0..n).filter(|&i| rows[i].in_matched).collect();
    eprintln!(
        "plausible speakers: {n} ({nm}M / {}F); pitch-matched: {}",
        n - nm,
        midx.len()
    );

    let y: Vec<f64> = rows.iter().map(|r| r.y).collect();
    let mut cols: HashMap<String, Vec<f64>> = {
        let mut m = HashMap::new();
        let mut names = resonance_feats();
        names.push("f0".into());
        names.push("f0_range".into());
        for name in names {
            let v: Vec<f64> = rows.iter().map(|r| r.f[&name]).collect();
            m.insert(name, v);
        }
        m
    };
    // sibilants: optional, mean-imputed where fricatives were too sparse
    let mut sib_present = 0usize;
    for (k, name) in SIB.iter().enumerate() {
        let vals: Vec<f64> = rows.iter().filter_map(|r| r.f.get(*name).copied()).collect();
        let mean = if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 };
        if k == 0 {
            sib_present = vals.len();
        }
        let col: Vec<f64> = rows.iter().map(|r| r.f.get(*name).copied().unwrap_or(mean)).collect();
        cols.insert(name.to_string(), col);
    }
    eprintln!("sibilant coverage: {sib_present}/{n} speakers ({} imputed)", n - sib_present);

    // resonance model = pitch-independent envelope + dynamics + sibilants
    let res: Vec<String> = resonance_feats()
        .into_iter()
        .chain(SIB.iter().map(|s| s.to_string()))
        .collect();
    // combo = pitch (level + dynamic range) + the resonance model
    let combo: Vec<String> = ["f0", "f0_range"]
        .iter()
        .map(|s| s.to_string())
        .chain(res.clone())
        .collect();

    // comparison table
    let sets: Vec<(&str, Vec<String>)> = vec![
        ("pitch F0", vec!["f0".into()]),
        ("VTL only", vec!["vtl".into()]),
        ("F1-F5", vec!["f1".into(), "f2".into(), "f3".into(), "f4".into(), "f5".into()]),
        ("LPC-cepstrum", LPCC.iter().map(|s| s.to_string()).collect()),
        ("spectral", SPEC.iter().map(|s| s.to_string()).collect()),
        ("resonance (rich)", res.clone()),
        ("combo (F0+rich)", combo.clone()),
    ];
    eprintln!("\n{:22} {:>11} {:>9}", "feature set", "matchedAUC", "fullAUC");
    for (name, feats) in &sets {
        let probs = cv_probs(&build_matrix(&cols, feats), &y, 5);
        let (mauc, _) = auc_acc(&probs, &y, &midx);
        let (fauc, _) = auc_acc(&probs, &y, &(0..n).collect::<Vec<_>>());
        eprintln!("{name:22} {mauc:>11.3} {fauc:>9.3}");
    }

    // stored models
    let vtl_probs = cv_probs(&build_matrix(&cols, &["vtl".to_string()]), &y, 5);
    let (vtl_mauc, _) = auc_acc(&vtl_probs, &y, &midx);
    let threshold = vtl_threshold(&cols["vtl"], &y);

    let res_probs = cv_probs(&build_matrix(&cols, &res), &y, 5);
    let (res_mauc, res_macc) = auc_acc(&res_probs, &y, &midx);
    let (res_fauc, _) = auc_acc(&res_probs, &y, &(0..n).collect::<Vec<_>>());

    let combo_probs = cv_probs(&build_matrix(&cols, &combo), &y, 5);
    let (combo_mauc, combo_macc) = auc_acc(&combo_probs, &y, &midx);
    let (combo_fauc, _) = auc_acc(&combo_probs, &y, &(0..n).collect::<Vec<_>>());

    eprintln!(
        "\nstored: resonance(rich) matchedAUC {res_mauc:.3} fullAUC {res_fauc:.3} acc {:.1}% | \
         combo matchedAUC {combo_mauc:.3} fullAUC {combo_fauc:.3} | (old VTL matchedAUC {vtl_mauc:.3})",
        res_macc * 100.0
    );

    let meta = Meta {
        auc: res_mauc,
        full_auc: res_fauc,
        matched_acc: res_macc,
        combo_auc: combo_mauc,
        combo_full_auc: combo_fauc,
        combo_matched_acc: combo_macc,
        vtl_auc: vtl_mauc,
        threshold,
    };
    write_db(&a[1], &rows, &res_probs, &combo_probs, &meta)?;
    eprintln!("wrote resonance table (rich + combo) for {n} speakers → {}", a[1]);
    Ok(())
}

fn load_wide(path: &str) -> Result<Vec<Row>> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines();
    let header: Vec<String> = lines.next().unwrap_or("").split(',').map(|s| s.to_string()).collect();
    let idx = |name: &str| header.iter().position(|h| h == name);
    let need = resonance_feats();
    let mut out = Vec::new();
    for line in lines {
        let c: Vec<&str> = line.split(',').collect();
        if c.len() != header.len() {
            continue;
        }
        let gender = c[idx("gender").unwrap()];
        if gender != "male" && gender != "female" {
            continue;
        }
        let get = |name: &str| idx(name).and_then(|i| c[i].trim().parse::<f64>().ok());
        // all needed features + f0 (level) + f0_range must be present
        let extra = ["f0".to_string(), "f0_range".to_string()];
        let mut f: HashMap<String, f64> = HashMap::new();
        let mut ok = true;
        for name in need.iter().chain(extra.iter()) {
            match get(name) {
                Some(v) => {
                    f.insert(name.clone(), v);
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        // sibilants are optional — store if present, imputed later if not
        for name in SIB {
            if let Some(v) = get(name) {
                f.insert(name.to_string(), v);
            }
        }
        // plausibility filter (mirrors the analysis load)
        let (f0, f1, f2, f3, f4, vtl) = (f["f0"], f["f1"], f["f2"], f["f3"], f["f4"], f["vtl"]);
        if !(200.0..=1100.0).contains(&f1) || !(f2 > f1 && f3 > f2 && f4 > f3) {
            continue;
        }
        if !(80.0..=320.0).contains(&f0) || !(8.0..=22.0).contains(&vtl) {
            continue;
        }
        out.push(Row {
            speaker: c[idx("speaker").unwrap()].to_string(),
            gender: gender.to_string(),
            y: if gender == "female" { 1.0 } else { 0.0 },
            in_matched: false,
            f,
        });
    }
    Ok(out)
}

/// Greedy nearest-pitch M↔F matching within TOL Hz.
fn pitch_match(rows: &mut [Row]) {
    let f0 = |r: &Row| r.f["f0"];
    let mut men: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].y == 0.0).collect();
    let women: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].y == 1.0).collect();
    men.sort_by(|&a, &b| f0(&rows[a]).partial_cmp(&f0(&rows[b])).unwrap());
    let mut used = vec![false; rows.len()];
    for &m in &men {
        let mf0 = f0(&rows[m]);
        let mut best = None;
        let mut bestd = f64::MAX;
        for &wi in &women {
            if used[wi] {
                continue;
            }
            let dd = (f0(&rows[wi]) - mf0).abs();
            if dd < bestd {
                bestd = dd;
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

/// Standardize named columns → row-major feature vectors.
fn build_matrix(cols: &HashMap<String, Vec<f64>>, names: &[String]) -> Vec<Vec<f64>>
where
{
    let n = cols[&names[0]].len();
    let stats: Vec<(f64, f64)> = names.iter().map(|k| mean_std(&cols[k])).collect();
    (0..n)
        .map(|i| {
            names
                .iter()
                .enumerate()
                .map(|(j, k)| (cols[k][i] - stats[j].0) / stats[j].1)
                .collect()
        })
        .collect()
}

fn vtl_threshold(vtl: &[f64], y: &[f64]) -> f64 {
    let (mean, std) = mean_std(vtl);
    let z: Vec<Vec<f64>> = vtl.iter().map(|v| vec![(v - mean) / std]).collect();
    let w = logistic_irls(&z, y);
    if w[1].abs() > 1e-9 {
        mean + (-w[0] / w[1]) * std
    } else {
        mean
    }
}

fn auc_acc(probs: &[f64], y: &[f64], idx: &[usize]) -> (f64, f64) {
    let p: Vec<f64> = idx.iter().map(|&i| probs[i]).collect();
    let yy: Vec<f64> = idx.iter().map(|&i| y[i]).collect();
    let acc = idx.iter().filter(|&&i| (probs[i] >= 0.5) == (y[i] == 1.0)).count() as f64
        / idx.len().max(1) as f64;
    (auc(&p, &yy), acc)
}

struct Meta {
    auc: f64,
    full_auc: f64,
    matched_acc: f64,
    combo_auc: f64,
    combo_full_auc: f64,
    combo_matched_acc: f64,
    vtl_auc: f64,
    threshold: f64,
}

fn write_db(db: &str, rows: &[Row], res_probs: &[f64], combo_probs: &[f64], meta: &Meta) -> Result<()> {
    let mut conn = Connection::open(db)?;
    conn.execute_batch(
        "DROP TABLE IF EXISTS resonance;
         DROP TABLE IF EXISTS resonance_meta;
         CREATE TABLE resonance (
            speaker text primary key, gender text,
            f0 real, f1 real, f2 real, f3 real, f4 real, f5 real, vtl real,
            centroid real, tilt real, rolloff real, h1h2 real,
            prob_female real, pred text, correct integer, margin real, in_matched integer,
            combo_prob real, combo_pred text, combo_correct integer, combo_margin real);
         CREATE TABLE resonance_meta (
            auc real, full_auc real, threshold real, matched_acc real,
            combo_auc real, combo_full_auc real, combo_matched_acc real, vtl_auc real);
         CREATE INDEX idx_res_margin ON resonance(margin);
         CREATE INDEX idx_res_correct ON resonance(correct);
         CREATE INDEX idx_res_combo_margin ON resonance(combo_margin);
         CREATE INDEX idx_res_combo_correct ON resonance(combo_correct);",
    )?;
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO resonance VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )?;
        let g = |r: &Row, k: &str| r.f.get(k).copied().unwrap_or(f64::NAN);
        for (i, r) in rows.iter().enumerate() {
            let prob = res_probs[i];
            let pred = if prob >= 0.5 { "female" } else { "male" };
            let correct = ((prob >= 0.5) == (r.y == 1.0)) as i64;
            let margin = (prob - 0.5).abs();
            let cprob = combo_probs[i];
            let cpred = if cprob >= 0.5 { "female" } else { "male" };
            let ccorrect = ((cprob >= 0.5) == (r.y == 1.0)) as i64;
            let cmargin = (cprob - 0.5).abs();
            stmt.execute(rusqlite::params![
                r.speaker, r.gender,
                g(r, "f0"), g(r, "f1"), g(r, "f2"), g(r, "f3"), g(r, "f4"), g(r, "f5"), g(r, "vtl"),
                g(r, "centroid"), g(r, "tilt"), g(r, "rolloff"), g(r, "h1h2"),
                prob, pred, correct, margin, r.in_matched as i64,
                cprob, cpred, ccorrect, cmargin
            ])?;
        }
        tx.execute(
            "INSERT INTO resonance_meta VALUES (?,?,?,?,?,?,?,?)",
            rusqlite::params![
                meta.auc, meta.full_auc, meta.threshold, meta.matched_acc,
                meta.combo_auc, meta.combo_full_auc, meta.combo_matched_acc, meta.vtl_auc
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

// ── logistic regression (IRLS) + CV + AUC ────────────────────────────────────
fn mean_std(x: &[f64]) -> (f64, f64) {
    let m = x.iter().sum::<f64>() / x.len() as f64;
    let var = x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / x.len() as f64;
    (m, var.sqrt().max(1e-9))
}

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

fn logistic_irls(feats: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let n = feats.len();
    let p = feats[0].len();
    let dim = p + 1;
    let mut w = vec![0.0; dim];
    for _ in 0..40 {
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
        for d in 0..dim {
            ata[d][d] += 1e-4; // ridge (stabilizes the ~23-feature fit)
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
            let fct = m[r][col] / d;
            for c in col..n {
                m[r][c] -= fct * m[col][c];
            }
            x[r] -= fct * x[col];
        }
    }
    for i in 0..n {
        if m[i][i].abs() > 1e-12 {
            x[i] /= m[i][i];
        }
    }
    x
}

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

fn auc(scores: &[f64], y: &[f64]) -> f64 {
    let mut idx: Vec<usize> = (0..scores.len()).collect();
    idx.sort_by(|&a, &b| scores[a].partial_cmp(&scores[b]).unwrap());
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
