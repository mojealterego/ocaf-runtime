#!/usr/bin/env python3
"""Opt out of deprecated Zenoh YAML decoder and migrate PEM handling upstream.

Only three selected Cargo-verified crates from the same Zenoh 1.10.1
release are locally patched; other registry crates remain untouched.
"""
from pathlib import Path
import argparse
import re
import shutil
import tomllib

NAMES = ("zenoh-config", "zenoh-link-commons", "zenoh-link-tls")
REPLACEMENTS = (
    ("rustls_pemfile::certs(", "CertificateDer::pem_reader_iter("),
    ("rustls_pemfile::rsa_private_keys(", "rustls_pki_types::PrivatePkcs1KeyDer::pem_reader_iter("),
    ("rustls_pemfile::pkcs8_private_keys(", "rustls_pki_types::PrivatePkcs8KeyDer::pem_reader_iter("),
    ("rustls_pemfile::ec_private_keys(", "rustls_pki_types::PrivateSec1KeyDer::pem_reader_iter("),
)


def replace_normalized_dep(manifest: Path, dependency: str) -> None:
    content = manifest.read_text()
    match = re.search(
        r"(?ms)^\[dependencies\." + re.escape(dependency) +
        r"\]\s*\n.*?(?=^\[|\Z)", content
    )
    assert match is not None, f"expected normalized {dependency} dependency"
    manifest.write_text(content[:match.start()] + content[match.end():])


def remove_pem_call_sites(source_file: Path, anchor: str) -> None:
    text = source_file.read_text()
    assert anchor in text, "PEM decoder import location changed"
    total = 0
    for old, new in REPLACEMENTS:
        occurrences = text.count(old)
        assert occurrences >= 1, f"expected PEM usage not found: {old}"
        total += occurrences
        text = text.replace(old, new)
    assert total >= 8 and "rustls_pemfile::" not in text
    text = text.replace(anchor, "use rustls_pki_types::pem::PemObject;\n" + anchor, 1)
    source_file.write_text(text)


def patch(repo: Path, cargo_home: Path) -> None:
    repo = repo.resolve()
    data = tomllib.loads((repo / "Cargo.toml").read_text())
    assert data["patch"]["crates-io"] == {
        "zenoh-transport": {"path": "vendor/zenoh-transport"},
        "token-cell": {"path": "vendor/token-cell"},
    }, "unrecognized existing vendor overlay"
    lock = tomllib.loads((repo / "Cargo.lock").read_text())["package"]
    for name in NAMES:
        matching = [p for p in lock if p["name"] == name]
        assert len(matching) == 1 and matching[0]["version"] == "1.10.1"
        assert matching[0]["source"] == "registry+https://github.com/rust-lang/crates.io-index"
        assert re.fullmatch(r"[0-9a-f]{64}", matching[0]["checksum"])
        paths = list((cargo_home / "registry/src").glob(f"*/{name}-1.10.1"))
        assert len(paths) == 1, f"unable to locate verified {name} source"
        target = repo / "vendor" / name
        assert not target.exists(), f"will not overwrite existing {name} source"
        assert all(not file.is_symlink() for file in paths[0].rglob("*"))
        shutil.copytree(paths[0], target)

    config = repo / "vendor/zenoh-config"
    config_src = config / "src/lib.rs"
    source = config_src.read_text()
    pattern = re.compile(
        r'(?s)Some\("yaml"\) \| Some\("yml"\) => '
        r'Config::from_deserializer\(serde_yaml::Deserializer::from_str\(&content\)\)'
        r'\.map_err\(\|e\| match e \{.*?\}\),'
    )
    new, matches = pattern.subn(
        'Some("yaml") | Some("yml") => bail!("YAML configuration disabled by OCAF; use JSON5"),',
        source
    )
    assert matches == 1, "Zenoh YAML source API differs from audited release"
    assert "serde_yaml::" not in new
    config_src.write_text(new)
    replace_normalized_dep(config / "Cargo.toml", "serde_yaml")

    tls = repo / "vendor/zenoh-link-tls"
    remove_pem_call_sites(
        tls / "src/utils.rs", "use rustls_pki_types::ServerName;"
    )
    replace_normalized_dep(tls / "Cargo.toml", "rustls-pemfile")

    commons = repo / "vendor/zenoh-link-commons"
    remove_pem_call_sites(
        commons / "src/quic/utils.rs", "use secrecy::ExposeSecret;"
    )
    manifest = commons / "Cargo.toml"
    replace_normalized_dep(manifest, "rustls-pemfile")
    manifest_text = manifest.read_text()
    manifest_text, count = re.subn(
        r'(?m)^[ \t]*"dep:rustls-pemfile",[ \t]*\n', "", manifest_text
    )
    assert count == 1, "QUIC cannot retain the removed optional PEM dependency"
    manifest.write_text(manifest_text)

    root = repo / "Cargo.toml"
    root.write_text(root.read_text().rstrip() +
        '\n# Reviewed Zenoh JSON5-only and rustls-pki-types security backports.\n'
        + "".join(
            f'{name} = {{ path = "vendor/{name}" }}\n' for name in NAMES
        ))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--source-home", type=Path, required=True)
    args = parser.parse_args()
    patch(args.repo, args.source_home)
