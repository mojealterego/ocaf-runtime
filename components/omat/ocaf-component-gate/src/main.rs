use std::env;
use std::path::PathBuf;

use ocaf_component_gate::audit;

fn main() {
    let mut repo = PathBuf::from(".");
    let mut strict = false;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--repo" => match arguments.next() {
                Some(path) => repo = PathBuf::from(path),
                None => {
                    eprintln!("--repo requires a path");
                    std::process::exit(2);
                }
            },
            "--strict-targets" => strict = true,
            "--help" => {
                println!("Usage: ocaf-component-gate [--repo PATH] [--strict-targets]");
                return;
            }
            _ => {
                eprintln!("unknown argument: {argument}");
                std::process::exit(2);
            }
        }
    }
    match audit(&repo, strict) {
        Ok(report) => {
            println!(
                "OCAF component gate: inspected={}, pending_targets={}, violations={}",
                report.inspected,
                report.missing_targets.len(),
                report.errors.len()
            );
            for target in &report.missing_targets {
                println!("PENDING (not implemented): {target}");
            }
            for error in &report.errors {
                eprintln!("INVALID: {error}");
            }
            if !report.accepted() {
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("cannot audit OCAF component tree: {err}");
            std::process::exit(2);
        }
    }
}
