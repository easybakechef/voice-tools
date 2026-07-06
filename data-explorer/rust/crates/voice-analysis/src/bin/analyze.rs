//! Analyze one or more audio files, print F0/F1-F4/VTL. JSON with `--json`.
//!   cargo run --release --bin analyze -- [--json] <file>...

use std::path::Path;
use voice_analysis::{analyze, audio};

fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    args.retain(|a| a != "--json");
    if args.is_empty() {
        eprintln!("usage: analyze [--json] <file>...");
        std::process::exit(2);
    }
    if json {
        print!("[");
    }
    for (i, arg) in args.iter().enumerate() {
        let (samples, sr) = audio::load(Path::new(arg))?;
        let f = analyze(&samples, sr);
        let vtl = f.vtl_cm();
        if json {
            if i > 0 {
                print!(",");
            }
            print!(
                "{{\"path\":{:?},\"f0\":{},\"f1\":{},\"f2\":{},\"f3\":{},\"f4\":{},\"vtl_cm\":{},\"voiced\":{}}}",
                arg,
                onull(f.f0), onull(f.f1), onull(f.f2), onull(f.f3), onull(f.f4), onull(vtl),
                f.voiced_frames
            );
        } else {
            println!("{}", arg);
            println!(
                "  F0 {}  F1-F4 {} / {} / {} / {}  VTL {} cm  ({} voiced)",
                fmt(f.f0), fmt(f.f1), fmt(f.f2), fmt(f.f3), fmt(f.f4), fmt1(vtl), f.voiced_frames
            );
        }
    }
    if json {
        println!("]");
    }
    Ok(())
}

fn fmt(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.0}")).unwrap_or_else(|| "-".into())
}
fn fmt1(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.1}")).unwrap_or_else(|| "-".into())
}
fn onull(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.3}")).unwrap_or_else(|| "null".into())
}
