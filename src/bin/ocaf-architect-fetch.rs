//! Trusted operator CLI for one pinned-IP HTTPS source read.
//! This is NOT an LLM-accessible OMAt executor. The trusted service manager
//! owns route configuration and must not permit agent-controlled environment
//! variables or output paths.

use ocaf_runtime::architect_https_source_fetch::{fetch_https, PinnedSourceRoute};
use std::{
    env,
    error::Error,
    fs::OpenOptions,
    io::{self, Write},
    net::Ipv4Addr,
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

fn invalid(msg: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg)
}

fn required(key: &'static str) -> io::Result<String> {
    env::var(key).map_err(|_| invalid("missing required OCAF_SOURCE_* configuration"))
}

fn config() -> io::Result<(PinnedSourceRoute, PathBuf)> {
    if env::args().skip(1).collect::<Vec<_>>() != ["--once"] {
        return Err(invalid("only --once is supported"));
    }
    let ip: Ipv4Addr = required("OCAF_SOURCE_PINNED_IPV4")?
        .parse()
        .map_err(|_| invalid("invalid pinned public IPv4"))?;
    let route = PinnedSourceRoute {
        tenant: required("OCAF_SOURCE_TENANT")?,
        hostname: required("OCAF_SOURCE_HOSTNAME")?,
        pinned_ipv4: ip,
        path: required("OCAF_SOURCE_PATH")?,
        max_body_bytes: required("OCAF_SOURCE_MAX_BYTES")?
            .parse()
            .map_err(|_| invalid("invalid byte limit"))?,
        timeout_seconds: required("OCAF_SOURCE_TIMEOUT_SECONDS")?
            .parse()
            .map_err(|_| invalid("invalid timeout"))?,
    };
    route
        .validate()
        .map_err(|_| invalid("unsafe or invalid source route"))?;
    let output = PathBuf::from(required("OCAF_SOURCE_OUTPUT")?);
    if !output.is_absolute() || output.file_name().is_none() {
        return Err(invalid("output must be an absolute new file"));
    }
    Ok((route, output))
}

fn run() -> Result<(), Box<dyn Error>> {
    let (route, output) = config()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let result = fetch_https(&route, &route.tenant, now)?;
    // Write only after complete TLS verification, HTTP validation and hashing.
    // create_new never overwrites an existing path, including a symlink.
    let mut file_opts = OpenOptions::new();
    file_opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        file_opts.mode(0o600);
    }
    let mut file = file_opts.open(&output)?;
    if let Err(err) = file.write_all(&result.body).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = std::fs::remove_file(&output);
        return Err(Box::new(err));
    }
    println!("{}", serde_json::to_string(&result.source)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("source_fetch_rejected: {error}");
        process::exit(1);
    }
}
