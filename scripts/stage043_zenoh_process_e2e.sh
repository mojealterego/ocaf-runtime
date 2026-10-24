#!/usr/bin/env bash
set -euo pipefail
umask 077
root="$(mktemp -d)"
server_pid=""
cleanup() { if [[ -n "$server_pid" ]]; then kill "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true; fi; rm -rf "$root"; }
trap cleanup EXIT
cd "$root"
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.pem -days 1 -subj /CN=OCAF-Test-CA >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -keyout server.key -out server.csr -subj /CN=localhost >/dev/null 2>&1
printf 'subjectAltName=DNS:localhost,IP:127.0.0.1\nextendedKeyUsage=serverAuth\n' >server.ext
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca.key -CAcreateserial -out server.pem -days 1 -extfile server.ext >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -keyout client.key -out client.csr -subj /CN=omas-test >/dev/null 2>&1
printf 'extendedKeyUsage=clientAuth\n' >client.ext
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca.key -CAcreateserial -out client.pem -days 1 -extfile client.ext >/dev/null 2>&1
cat >server.json5 <<EOF
{mode:"peer",scouting:{multicast:{enabled:false},gossip:{enabled:false}},listen:{endpoints:["tls/127.0.0.1:17443"]},transport:{link:{protocols:["tls"],tls:{enable_mtls:true,verify_name_on_connect:true,close_link_on_expiration:true,root_ca_certificate:"$root/ca.pem",listen_private_key:"$root/server.key",listen_certificate:"$root/server.pem"}}}}
EOF
cat >client.json5 <<EOF
{mode:"client",scouting:{multicast:{enabled:false},gossip:{enabled:false}},connect:{endpoints:["tls/localhost:17443"]},transport:{link:{protocols:["tls"],tls:{enable_mtls:true,verify_name_on_connect:true,close_link_on_expiration:true,root_ca_certificate:"$root/ca.pem",connect_private_key:"$root/client.key",connect_certificate:"$root/client.pem"}}}}
EOF
export OCAF_OMAT_PEER=omas_a OCAF_OMAT_TENANT=tenant_a OCAF_OMAT_SHARED_KEY_B64
OCAF_OMAT_SHARED_KEY_B64="$(python3 -c 'import base64; print(base64.b64encode(bytes([7])*32).decode())')"
export OCAF_OMAT_REPLAY_DB="$root/replay.sqlite" OCAF_OMAT_REQUEST_ID=req_1 OCAF_OMAT_NONCE=0123456789abcdef
server_bin="$GITHUB_WORKSPACE/target/debug/ocaf-omat-zenoh"
client_bin="$GITHUB_WORKSPACE/target/debug/ocaf-omas-zenoh-client"
start_server() { OCAF_ZENOH_CONFIG="$root/server.json5" "$server_bin" >"$root/server.log" 2>&1 & server_pid=$!; sleep 3; kill -0 "$server_pid"; }
probe_rejected() { local output; if output="$(OCAF_ZENOH_CONFIG="$root/client.json5" timeout 15 "$client_bin" 2>&1)"; then echo "unexpected tool execution accepted"; exit 1; fi; grep -q 'OMAt rejected request' <<<"$output" || { echo "$output"; exit 1; }; }
start_server
probe_rejected
python3 - <<'PY'
import os,sqlite3
p=os.environ['OCAF_OMAT_REPLAY_DB']
assert sqlite3.connect(p).execute('select count(*) from omat_replay').fetchone()[0]==1
PY
kill "$server_pid"; wait "$server_pid" || true; server_pid=""
start_server
probe_rejected
python3 - <<'PY'
import os,sqlite3
p=os.environ['OCAF_OMAT_REPLAY_DB']
assert sqlite3.connect(p).execute('select count(*) from omat_replay').fetchone()[0]==1
PY
kill -9 "$server_pid"; wait "$server_pid" 2>/dev/null || true; server_pid=""
if OCAF_ZENOH_CONFIG="$root/client.json5" timeout 15 "$client_bin" >"$root/disconnect.log" 2>&1; then echo 'unexpected success after server crash'; exit 1; fi
printf 'PASS: mTLS two-process signed denial, durable replay after restart, server crash fail-closed\n'
