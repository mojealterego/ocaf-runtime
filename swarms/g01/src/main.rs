use ocaf_g01_swarm::{
    decode_hex, encode_hex, ClaimRequest, ReceiptPayload, SignedReceipt, SwarmStore,
};
use ocaf_stage_ledger::StageRecord;
use openssl::pkey::PKey;
use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

fn usage() -> &'static str {
    "usage:\n  ocaf-g01-swarm keygen PUBLIC_HEX_FILE PRIVATE_HEX_FILE\n  ocaf-g01-swarm init STORE_DIR PLAN_JSON PUBLIC_HEX_FILE\n  ocaf-g01-swarm next STORE_DIR\n  ocaf-g01-swarm status STORE_DIR\n  ocaf-g01-swarm claim STORE_DIR CLAIM_REQUEST_JSON\n  ocaf-g01-swarm prepare STORE_DIR ARTIFACT COMMIT TEST_COMMAND TEST_RESULT RECEIPT_ID [LIMITS_JSON]\n  ocaf-g01-swarm operator-sign STORE_DIR ARTIFACT PAYLOAD_JSON PRIVATE_HEX_FILE\n  ocaf-g01-swarm handoff STORE_DIR ARTIFACT SIGNED_RECEIPT_JSON"
}

fn read_hex_file<const N: usize>(path: &str) -> Result<[u8; N], Box<dyn Error>> {
    Ok(decode_hex::<N>(fs::read_to_string(path)?.trim())?)
}

fn write_new(path: &str, contents: &str, private: bool) -> Result<(), Box<dyn Error>> {
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .mode(if private { 0o600 } else { 0o644 });
    let mut file = options.open(path)?;
    file.write_all(contents.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    match args.get(1).map(String::as_str) {
        Some("keygen") if args.len() == 4 => {
            let key = PKey::generate_ed25519()?;
            write_new(&args[3], &encode_hex(&key.raw_private_key()?), true)?;
            write_new(&args[2], &encode_hex(&key.raw_public_key()?), false)?;
            println!("operator key pair created; protect the private file");
        }
        Some("init") if args.len() == 5 => {
            let plan: Vec<StageRecord> = serde_json::from_slice(&fs::read(&args[3])?)?;
            let public = fs::read_to_string(&args[4])?;
            SwarmStore::new(&args[2]).initialize(plan, public.trim())?;
            println!("G01 local store initialized");
        }
        Some("next") if args.len() == 3 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&SwarmStore::new(&args[2]).next()?)?
            );
        }
        Some("status") if args.len() == 3 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&SwarmStore::new(&args[2]).status()?)?
            );
        }
        Some("claim") if args.len() == 4 => {
            let request: ClaimRequest = serde_json::from_slice(&fs::read(&args[3])?)?;
            println!("{}", SwarmStore::new(&args[2]).claim(request)?);
        }
        Some("prepare") if args.len() == 8 || args.len() == 9 => {
            let limits: Vec<String> = if args.len() == 9 {
                serde_json::from_slice(&fs::read(&args[8])?)?
            } else {
                Vec::new()
            };
            let payload = SwarmStore::new(&args[2]).prepare(
                Path::new(&args[3]),
                args[4].clone(),
                args[5].clone(),
                args[6].clone(),
                args[7].clone(),
                limits,
            )?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        Some("operator-sign") if args.len() == 6 => {
            if fs::metadata(&args[5])?.permissions().mode() & 0o077 != 0 {
                return Err("private key file must be owner-only (chmod 600)".into());
            }
            let key = read_hex_file::<32>(&args[5])?;
            let payload: ReceiptPayload = serde_json::from_slice(&fs::read(&args[4])?)?;
            let signed =
                SwarmStore::new(&args[2]).operator_sign(Path::new(&args[3]), payload, &key)?;
            println!("{}", serde_json::to_string_pretty(&signed)?);
        }
        Some("handoff") if args.len() == 5 => {
            let signed: SignedReceipt = serde_json::from_slice(&fs::read(&args[4])?)?;
            println!(
                "{}",
                SwarmStore::new(&args[2]).handoff(Path::new(&args[3]), signed)?
            );
        }
        _ => return Err(usage().into()),
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Err(error) = run(&args) {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
