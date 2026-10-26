#!/usr/bin/env python3
"""Disposable PostgreSQL 16 TLS/role admission lab for stage 0.40.10.

Runs on one Linux kernel. It checks real TCP TLS 1.3, distinct certificate
principals, minimal replication/admin roles and negative downgrade attempts.
It cannot prove independent-host TLS, PQC, witness fencing or stage closure.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import pwd
import shutil
import socket
import stat
import subprocess
import tempfile


class PgTlsLabError(ValueError):
    pass


def hba_policy() -> str:
    """No TCP trust/password fallback; replication and admin certs are distinct."""
    return ("local all postgres trust\n"
            "hostnossl all all 127.0.0.1/32 reject\n"
            "hostnossl replication all 127.0.0.1/32 reject\n"
            "hostssl replication ocaf_repl 127.0.0.1/32 cert\n"
            "hostssl all ocaf_admin 127.0.0.1/32 cert\n"
            "hostssl all all 127.0.0.1/32 reject\n"
            "hostssl replication all 127.0.0.1/32 reject\n"
            "host all all 0.0.0.0/0 reject\n")


def validate_report(report: dict) -> dict:
    required = ("tls13_admin_tcp_verified", "admin_client_certificate_verified",
                "replication_client_certificate_verified", "distinct_role_certificates",
                "replication_role_not_superuser", "admin_role_not_superuser",
                "admin_role_cannot_replicate", "replication_role_cannot_admin",
                "non_tls_tcp_rejected", "rogue_client_certificate_rejected",
                "wrong_ca_rejected")
    if (not isinstance(report, dict)
            or report.get("status") != "ONE_KERNEL_PG_TLS13_SEPARATE_ROLES_LAB_PASS"
            or any(report.get(key) is not True for key in required)
            or report.get("one_underlying_linux_kernel") is not True
            or report.get("independent_host_tls_tested") is not False
            or report.get("pqc_hybrid_tested") is not False
            or report.get("stage_040_complete") is not False
            or report.get("production_approved") is not False):
        raise PgTlsLabError("TLS role proof incomplete or scope overclaimed")
    return report


def command(argv: list[str], *, timeout: int = 30, ok: bool = True) -> subprocess.CompletedProcess:
    try:
        result = subprocess.run(argv, capture_output=True, text=True, timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise PgTlsLabError("PostgreSQL TLS lab command unavailable: " + argv[0]) from exc
    if ok and result.returncode:
        raise PgTlsLabError("PostgreSQL TLS lab command failed: " + " ".join(argv[:3])
                            + ": " + result.stderr[-500:])
    return result


def run_lab(pg_bin: Path, report: Path) -> dict:
    if (os.geteuid() != 0 or os.uname().sysname != "Linux"
            or not pg_bin.is_absolute() or pg_bin.is_symlink() or not pg_bin.is_dir()
            or not report.is_absolute() or report.exists() or report.is_symlink()
            or not report.parent.is_dir()):
        raise PgTlsLabError("privileged disposable Linux PostgreSQL lab required")
    for name in ("initdb", "pg_ctl", "psql", "pg_basebackup"):
        if not (pg_bin / name).is_file():
            raise PgTlsLabError("PostgreSQL server binary missing: " + name)
    postgres = pwd.getpwnam("postgres")
    root = Path(tempfile.mkdtemp(prefix="ocaf40-pg-tls-", dir="/tmp"))
    os.chown(root, postgres.pw_uid, postgres.pw_gid)
    cluster = root / "cluster"
    sockdir = root / "socket"
    sockdir.mkdir(mode=0o700)
    os.chown(sockdir, postgres.pw_uid, postgres.pw_gid)
    certdir = root / "cert"
    certdir.mkdir(mode=0o700)
    os.chown(certdir, postgres.pw_uid, postgres.pw_gid)
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]

    def pg(argv: list[str], *, timeout: int = 30, ok: bool = True) -> subprocess.CompletedProcess:
        return command(["runuser", "-u", "postgres", "--", *argv], timeout=timeout, ok=ok)

    def openssl(argv: list[str]) -> None:
        command(["openssl", *argv])

    def issue(name: str, subject: str, ext: str) -> tuple[Path, Path]:
        key, csr, cert = (certdir / f"{name}.{suffix}" for suffix in ("key", "csr", "crt"))
        openssl(["req", "-new", "-newkey", "rsa:2048", "-nodes", "-subj", subject,
                 "-keyout", str(key), "-out", str(csr)])
        extfile = certdir / f"{name}.ext"
        extfile.write_text(ext)
        ca_serial = certdir / "ca.srl"
        openssl(["x509", "-req", "-in", str(csr), "-CA", str(certdir / "ca.crt"),
                 "-CAkey", str(certdir / "ca.key"),
                 *( ["-CAserial", str(ca_serial)] if ca_serial.exists() else ["-CAcreateserial"] ),
                 "-out", str(cert), "-days", "1", "-extfile", str(extfile)])
        os.chown(key, postgres.pw_uid, postgres.pw_gid)
        os.chmod(key, stat.S_IRUSR | stat.S_IWUSR)
        os.chown(cert, postgres.pw_uid, postgres.pw_gid)
        return cert, key

    running = False
    try:
        openssl(["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
                 "-subj", "/CN=OCAF 0.40 TLS lab CA", "-addext", "basicConstraints=critical,CA:TRUE",
                 "-addext", "keyUsage=critical,keyCertSign,cRLSign",
                 "-keyout", str(certdir / "ca.key"), "-out", str(certdir / "ca.crt")])
        server_cert, server_key = issue("server", "/CN=127.0.0.1",
                                        "subjectAltName=IP:127.0.0.1\nextendedKeyUsage=serverAuth\n")
        admin_cert, admin_key = issue("admin", "/CN=ocaf_admin", "extendedKeyUsage=clientAuth\n")
        repl_cert, repl_key = issue("repl", "/CN=ocaf_repl", "extendedKeyUsage=clientAuth\n")
        rogue_cert, rogue_key = issue("rogue", "/CN=ocaf_rogue", "extendedKeyUsage=clientAuth\n")
        openssl(["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
                 "-subj", "/CN=Untrusted lab CA", "-keyout", str(certdir / "wrong-ca.key"),
                 "-out", str(certdir / "wrong-ca.crt")])
        for path in certdir.iterdir():
            os.chown(path, postgres.pw_uid, postgres.pw_gid)
        pg([str(pg_bin / "initdb"), "-D", str(cluster), "--auth-local=trust",
            "--auth-host=reject", "--no-instructions"], timeout=60)
        config = cluster / "postgresql.conf"
        config.write_text(config.read_text() +
                          f"\nlisten_addresses = '127.0.0.1'\nport = {port}\n"
                          f"unix_socket_directories = '{sockdir}'\n"
                          f"ssl = on\nssl_cert_file = '{server_cert}'\n"
                          f"ssl_key_file = '{server_key}'\nssl_ca_file = '{certdir / 'ca.crt'}'\n"
                          "ssl_min_protocol_version = 'TLSv1.3'\n"
                          "wal_level = replica\nmax_wal_senders = 4\n")
        (cluster / "pg_hba.conf").write_text(hba_policy())
        pg([str(pg_bin / "pg_ctl"), "-D", str(cluster), "-l", str(root / "server.log"),
            "-w", "start"], timeout=35)
        running = True

        def local(query: str) -> str:
            return pg([str(pg_bin / "psql"), "-X", "-A", "-t", "-q", "-v", "ON_ERROR_STOP=1",
                       "-h", str(sockdir), "-p", str(port), "-U", "postgres", "-d", "postgres",
                       "-c", query]).stdout.strip()

        local("CREATE ROLE ocaf_admin LOGIN")
        local("CREATE ROLE ocaf_repl LOGIN REPLICATION")
        roles = local("SELECT rolname || ':' || rolsuper::text || ':' || rolreplication::text || ':' || "
                      "rolcreatedb::text || ':' || rolcreaterole::text FROM pg_roles "
                      "WHERE rolname IN ('ocaf_admin','ocaf_repl') ORDER BY rolname").splitlines()
        if roles != ["ocaf_admin:false:false:false:false", "ocaf_repl:false:true:false:false"]:
            raise PgTlsLabError("separate least privilege PostgreSQL roles not observed")

        def conn(role: str, cert: Path, key: Path, *, ca: Path | None = None,
                 sslmode: str = "verify-full") -> str:
            return (f"host=127.0.0.1 port={port} dbname=postgres user={role} "
                    f"sslmode={sslmode} sslrootcert={ca or certdir / 'ca.crt'} "
                    f"sslcert={cert} sslkey={key} connect_timeout=3")

        def tcp(connection: str, query: str, *, ok: bool = True) -> subprocess.CompletedProcess:
            return pg([str(pg_bin / "psql"), "-X", "-A", "-t", "-q", "-v", "ON_ERROR_STOP=1",
                       connection, "-c", query], timeout=10, ok=ok)

        admin_conn = conn("ocaf_admin", admin_cert, admin_key)
        tls = tcp(admin_conn, "SELECT ssl::text || ':' || version FROM pg_stat_ssl "
                              "WHERE pid=pg_backend_pid()").stdout.strip()
        if tls != "true:TLSv1.3":
            raise PgTlsLabError("verified admin TCP connection did not negotiate TLS 1.3")
        bad_clear = tcp(conn("ocaf_admin", admin_cert, admin_key, sslmode="disable"),
                        "SELECT 1", ok=False)
        bad_rogue = tcp(conn("ocaf_admin", rogue_cert, rogue_key), "SELECT 1", ok=False)
        bad_ca = tcp(conn("ocaf_admin", admin_cert, admin_key, ca=certdir / "wrong-ca.crt"),
                     "SELECT 1", ok=False)
        bad_admin = tcp(admin_conn, "CREATE ROLE ocaf_unexpected_role", ok=False)
        bad_repl_sql = tcp(conn("ocaf_repl", repl_cert, repl_key), "SELECT 1", ok=False)
        if any(result.returncode == 0 for result in
               (bad_clear, bad_rogue, bad_ca, bad_admin, bad_repl_sql)):
            raise PgTlsLabError("PostgreSQL certificate, downgrade or role isolation bypassed")

        backup = root / "repl-backup"
        basebackup = pg([str(pg_bin / "pg_basebackup"), "-d", conn("ocaf_repl", repl_cert, repl_key),
                         "-D", str(backup), "-X", "none", "--no-sync"], timeout=80)
        if not (backup / "PG_VERSION").is_file() or basebackup.returncode:
            raise PgTlsLabError("replication-only certificate did not perform real base backup")
        bad_admin_repl = pg([str(pg_bin / "pg_basebackup"), "-d", admin_conn,
                             "-D", str(root / "admin-denied-backup"), "-X", "none", "--no-sync"],
                            timeout=20, ok=False)
        if bad_admin_repl.returncode == 0:
            raise PgTlsLabError("admin certificate unexpectedly performed replication")

        output = validate_report({
            "schema_version": 1, "status": "ONE_KERNEL_PG_TLS13_SEPARATE_ROLES_LAB_PASS",
            "observed_at_utc": datetime.now(timezone.utc).isoformat(),
            "postgresql_version": local("SHOW server_version"),
            "tls13_admin_tcp_verified": True, "admin_client_certificate_verified": True,
            "replication_client_certificate_verified": True, "distinct_role_certificates": True,
            "replication_role_not_superuser": True, "admin_role_not_superuser": True,
            "admin_role_cannot_replicate": True, "replication_role_cannot_admin": True,
            "non_tls_tcp_rejected": True, "rogue_client_certificate_rejected": True,
            "wrong_ca_rejected": True, "one_underlying_linux_kernel": True,
            "independent_host_tls_tested": False, "pqc_hybrid_tested": False,
            "stage_040_complete": False, "production_approved": False,
        })
        fd = os.open(report, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            json.dump(output, stream, indent=2, sort_keys=True)
            stream.write("\n")
        return output
    finally:
        if running:
            pg([str(pg_bin / "pg_ctl"), "-D", str(cluster), "-m", "immediate",
                "-w", "stop"], timeout=20, ok=False)
        shutil.rmtree(root, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pg-bin", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = run_lab(args.pg_bin, args.report)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    print(result["status"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
