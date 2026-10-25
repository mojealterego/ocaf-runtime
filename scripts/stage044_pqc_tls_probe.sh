#!/usr/bin/env bash
# A separately provisioned, REAL TLS 1.3 server MUST require X25519MLKEM768.
# This script does not enable PQC, provision keys, or pretend a handshake passed.
set -euo pipefail
: "${OCAF_PQC_HOST:?must set real test hostname}"
: "${OCAF_PQC_PORT:?must set real test port}"
: "${OCAF_PQC_CA_FILE:?must set real test CA bundle path}"
command -v openssl >/dev/null
[[ -f "$OCAF_PQC_CA_FILE" ]]
[[ "$OCAF_PQC_HOST" =~ ^[A-Za-z0-9.-]+$ ]]
[[ "$OCAF_PQC_PORT" =~ ^[0-9]{1,5}$ ]]
[[ "$OCAF_PQC_PORT" -ge 1 && "$OCAF_PQC_PORT" -le 65535 ]]
workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT
openssl version > "$workdir/openssl-version.txt"
# Positive hybrid handshake with actual server and verified certificate.
if ! timeout 15 openssl s_client -connect "${OCAF_PQC_HOST}:${OCAF_PQC_PORT}" \
  -servername "$OCAF_PQC_HOST" -tls1_3 -groups X25519MLKEM768 \
  -verify_return_error -verify_hostname "$OCAF_PQC_HOST" -CAfile "$OCAF_PQC_CA_FILE" \
  </dev/null >"$workdir/hybrid.log" 2>&1; then
  echo 'FAIL: hybrid TLS handshake' >&2; exit 1
fi
if ! grep -Eq 'Server Temp Key: X25519MLKEM768|Negotiated TLS1\.3 group: X25519MLKEM768|Peer Temp Key: X25519MLKEM768' "$workdir/hybrid.log"; then
  echo 'FAIL: hybrid key agreement not proven by OpenSSL output' >&2; exit 1
fi
if ! grep -Eq 'Verify return code: 0 \(ok\)|Verification: OK' "$workdir/hybrid.log"; then
  echo 'FAIL: peer certificate verification not proven' >&2; exit 1
fi
# Negative downgrade: a classical-only client must not complete TLS.
timeout 15 openssl s_client -connect "${OCAF_PQC_HOST}:${OCAF_PQC_PORT}" \
  -servername "$OCAF_PQC_HOST" -tls1_3 -groups X25519 \
  -verify_return_error -verify_hostname "$OCAF_PQC_HOST" -CAfile "$OCAF_PQC_CA_FILE" \
  </dev/null >"$workdir/classical.log" 2>&1 || true
if grep -Eq 'New, TLSv1\.3, Cipher is|Cipher[[:space:]]*:[[:space:]]*TLS_|Ciphersuite:[[:space:]]*TLS_' "$workdir/classical.log"; then
  echo 'FAIL: classical-only downgrade succeeded' >&2; exit 1
fi
# TLS 1.2 must not be negotiated by the hybrid-only endpoint.
timeout 15 openssl s_client -connect "${OCAF_PQC_HOST}:${OCAF_PQC_PORT}" \
  -servername "$OCAF_PQC_HOST" -tls1_2 \
  -verify_return_error -verify_hostname "$OCAF_PQC_HOST" -CAfile "$OCAF_PQC_CA_FILE" \
  </dev/null >"$workdir/tls12.log" 2>&1 || true
if grep -Eq 'New, TLSv1\.2, Cipher is|Protocol[[:space:]]*:[[:space:]]*TLSv1\.2' "$workdir/tls12.log"; then
  echo 'FAIL: TLS 1.2 downgrade succeeded' >&2; exit 1
fi
printf 'PASS: hybrid negotiated, classical-only and TLS 1.2 rejected; evidence digests:\n'
sha256sum "$workdir/openssl-version.txt" "$workdir/hybrid.log" "$workdir/classical.log" "$workdir/tls12.log"
