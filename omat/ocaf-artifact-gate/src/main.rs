use std::env;
use std::path::Path;
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "check-tree" {
        eprintln!("Usage: ocaf-artifact-gate check-tree REPOSITORY");
        std::process::exit(2);
    }
    let failures = ocaf_artifact_gate::validate_tree(Path::new(&args[2]));
    if failures.is_empty() {
        println!("OCAF STRUCTURE ACCEPTED; compilation, Zenoh runtime and certification require separate verified evidence");
    } else {
        for failure in failures { eprintln!("OCAF STRUCTURE REJECTED: {failure}"); }
        std::process::exit(1);
    }
}
