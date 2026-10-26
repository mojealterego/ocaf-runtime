#!/usr/bin/env bash
# Independent OMAt listeners, independent PKIs, overlap trust and old identity retirement.
# This tests authenticated ingress continuity, NOT execution of an OSTc tool.
set -Eeuo pipefail
umask 077
root="$(mktemp -d)"
declare -a server_pids=()
cleanup() {
  local status=$? log
  if (( status != 0 )); then
    for log in "$root"/*.log; do
      [[ -f "$log" ]] || continue
      printf -- '--- diagnostic: %s ---\\n' "$(basename "$log")"
      tail -n 30 "$log" || true
    done
  fi
  for pid in "${server_pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  for pid in "${server_pids[@]}"; do wait "$pid" 2>/dev/null || true; done
  rm -rf "$root"
}
trap cleanup EXIT

make_ca() {
  local label="$1"
  openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
    -keyout "$root/$label-ca.key" -out "$root/$label-ca.pem" \
    -subj "/CN=OCAF-$label-Rotation-Test-CA" \
    -addext 'basicConstraints=critical,CA:TRUE' \
    -addext 'keyUsage=critical,keyCertSign,cRLSign' >/dev/null 2>&1
}
make_identity() {
  local label="$1" role="$2" ca="$3" subject extension
  if [[ "$role" == server ]]; then
    subject='/CN=localhost'
    extension='subjectAltName=IP:127.0.0.1,DNS:localhost
extendedKeyUsage=serverAuth
keyUsage=digitalSignature,keyEncipherment'
  else
    subject="/CN=omas-$label"
    extension='extendedKeyUsage=clientAuth
keyUsage=digitalSignature,keyEncipherment'
  fi
  openssl req -newkey rsa:2048 -nodes \
    -keyout "$root/$label-$role.key" -out "$root/$label-$role.csr" \
    -subj "$subject" >/dev/null 2>&1
  printf '%s\n' "$extension" >"$root/$label-$role.ext"
  openssl x509 -req -in "$root/$label-$role.csr" \
    -CA "$root/$ca-ca.pem" -CAkey "$root/$ca-ca.key" -CAcreateserial \
    -out "$root/$label-$role.pem" -days 1 \
    -extfile "$root/$label-$role.ext" >/dev/null 2>&1
}
make_ca old
make_ca new
make_identity old server old
make_identity old client old
make_identity new server new
make_identity new client new
cat "$root/old-ca.pem" "$root/new-ca.pem" >"$root/overlap-ca.pem"

openssl verify -CAfile "$root/old-ca.pem" -purpose sslserver "$root/old-server.pem" >/dev/null
openssl verify -CAfile "$root/old-ca.pem" -purpose sslclient "$root/old-client.pem" >/dev/null
openssl verify -CAfile "$root/new-ca.pem" -purpose sslserver "$root/new-server.pem" >/dev/null
openssl verify -CAfile "$root/new-ca.pem" -purpose sslclient "$root/new-client.pem" >/dev/null
openssl verify -CAfile "$root/overlap-ca.pem" -purpose sslclient "$root/old-client.pem" >/dev/null
openssl verify -CAfile "$root/overlap-ca.pem" -purpose sslclient "$root/new-client.pem" >/dev/null
if openssl verify -CAfile "$root/new-ca.pem" "$root/old-client.pem" >/dev/null 2>&1; then
  echo 'FAIL: retired client identity trusted by new-only CA'; exit 1
fi
if openssl verify -CAfile "$root/new-ca.pem" "$root/old-server.pem" >/dev/null 2>&1; then
  echo 'FAIL: retired server identity trusted by new-only CA'; exit 1
fi
if [[ "${OCAF_ROTATION_CERTS_ONLY:-0}" == 1 ]]; then
  echo 'PASS: independent certificate chains, overlap roots and retired identity rejection'
  exit 0
fi

: "${GITHUB_WORKSPACE:?GITHUB_WORKSPACE must point to repository root}"
server_bin="$GITHUB_WORKSPACE/target/debug/ocaf-omat-zenoh"
client_bin="$GITHUB_WORKSPACE/target/debug/ocaf-omas-zenoh-client"
[[ -x "$server_bin" && -x "$client_bin" ]] || { echo 'FAIL: missing compiled binaries'; exit 1; }

server_config() {
  local name="$1" port="$2" identity="$3" trust="$4"
  cat >"$root/$name.json5" <<EOF
{mode:"peer",scouting:{multicast:{enabled:false},gossip:{enabled:false}},listen:{endpoints:["tls/127.0.0.1:$port"]},transport:{link:{protocols:["tls"],tls:{enable_mtls:true,verify_name_on_connect:true,close_link_on_expiration:true,root_ca_certificate:"$root/$trust-ca.pem",listen_private_key:"$root/$identity-server.key",listen_certificate:"$root/$identity-server.pem"}}}}
EOF
}
client_config() {
  local name="$1" port="$2" identity="$3" trust="$4"
  cat >"$root/$name.json5" <<EOF
{mode:"client",scouting:{multicast:{enabled:false},gossip:{enabled:false}},connect:{endpoints:["tls/127.0.0.1:$port"]},transport:{link:{protocols:["tls"],tls:{enable_mtls:true,verify_name_on_connect:true,close_link_on_expiration:true,root_ca_certificate:"$root/$trust-ca.pem",connect_private_key:"$root/$identity-client.key",connect_certificate:"$root/$identity-client.pem"}}}}
EOF
}
server_config old-listener 17443 old old
server_config overlap-listener 17444 new overlap
server_config final-listener 17445 new new
client_config old-to-old 17443 old overlap
client_config old-to-overlap 17444 old overlap
client_config new-to-overlap 17444 new overlap
client_config new-to-final 17445 new new
client_config old-to-final 17445 old overlap
client_config old-server-new-trust 17443 old new

export OCAF_OMAT_PEER=omas_a OCAF_OMAT_TENANT=tenant_a
export OCAF_OMAT_SHARED_KEY_B64
OCAF_OMAT_SHARED_KEY_B64="$(python3 -c 'import base64; print(base64.b64encode(bytes([7])*32).decode())')"
export OCAF_OMAT_REPLAY_DB="$root/replay.sqlite"
start_server() {
  local name="$1" pid
  OCAF_ZENOH_CONFIG="$root/$name.json5" "$server_bin" >"$root/$name.log" 2>&1 &
  pid=$!
  server_pids+=("$pid")
  sleep 2
  if ! kill -0 "$pid" 2>/dev/null; then
    echo "FAIL: $name exited during startup"; cat "$root/$name.log"; exit 1
  fi
  last_server_pid="$pid"
}
# Explicit policy rejection proves the signed request reached OMAt, rather
# than merely failing with a timeout. No external tool execution is enabled.
probe_handled() {
  local config="$1" seq="$2" output
  if output="$(OCAF_ZENOH_CONFIG="$root/$config.json5" \
    OCAF_OMAT_REQUEST_ID="req_$seq" OCAF_OMAT_NONCE="rotation$(printf '%024d' "$seq")" \
    timeout 20 "$client_bin" 2>&1)"; then
    echo "FAIL: unexpected tool execution accepted ($config/$seq)"; return 1
  fi
  if [[ "$output" != *'OMAt rejected request'* ]]; then
    echo "FAIL: request not handled by authenticated policy ($config/$seq): $output"; return 1
  fi
}
# A TLS rejection is valid only when the same listener accepts a trusted client.
probe_tls_rejected() {
  local config="$1" seq="$2" output
  if output="$(OCAF_ZENOH_CONFIG="$root/$config.json5" \
    OCAF_OMAT_REQUEST_ID="req_$seq" OCAF_OMAT_NONCE="rotation$(printf '%024d' "$seq")" \
    timeout 15 "$client_bin" 2>&1)"; then
    echo "FAIL: retired TLS identity unexpectedly accepted ($config)"; return 1
  fi
  if [[ "$output" != *'Zenoh open:'* ]]; then
    echo "FAIL: expected TLS connection rejection, got: $output"; return 1
  fi
}

start_server old-listener
old_pid="$last_server_pid"
probe_handled old-to-old 1
start_server overlap-listener
overlap_pid="$last_server_pid"
probe_handled old-to-overlap 2
probe_handled new-to-overlap 3
probe_handled old-to-old 4

# All in-flight authenticated requests must receive an application response
# before draining the old listener; count their durable nonce records.
declare -a inflight=()
for seq in $(seq 10 21); do
  if ((seq % 2 == 0)); then config=old-to-old; else config=new-to-overlap; fi
  probe_handled "$config" "$seq" >"$root/probe-$seq.log" 2>&1 &
  inflight+=("$!")
done
for i in "${!inflight[@]}"; do
  if ! wait "${inflight[$i]}"; then
    echo 'FAIL: in-flight request lost during overlap'; cat "$root"/probe-*.log; exit 1
  fi
done

start_server final-listener
final_pid="$last_server_pid"
probe_handled new-to-final 30
probe_tls_rejected old-to-final 31
probe_tls_rejected old-server-new-trust 32
probe_handled old-to-old 35
kill "$old_pid"; wait "$old_pid" 2>/dev/null || true
kill "$overlap_pid"; wait "$overlap_pid" 2>/dev/null || true
probe_handled new-to-final 33
probe_handled new-to-final 34

python3 - <<'PY'
import os, sqlite3
path = os.environ['OCAF_OMAT_REPLAY_DB']
expected = {1, 2, 3, 4, *range(10, 22), 30, 33, 34, 35}
rows = sqlite3.connect(path).execute('SELECT principal, nonce FROM omat_replay').fetchall()
assert len(rows) == len(expected), (len(rows), len(expected))
assert all(principal == 'omas_a' for principal, _ in rows)
actual = {int(nonce.removeprefix('rotation')) for _, nonce in rows}
assert actual == expected, (actual, expected)
assert 31 not in actual and 32 not in actual
PY
printf 'PASS: independent mTLS roots; old/new trust overlap; old identities rejected after withdrawal; all 20 authenticated ingress requests accounted for\n'
