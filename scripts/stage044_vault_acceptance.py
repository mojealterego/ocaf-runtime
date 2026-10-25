#!/usr/bin/env python3
"""Live Vault Transit acceptance. Requires a real configured HTTPS Vault, never dev mode."""
import base64
import json
import os
from pathlib import Path
import secrets
import ssl
import stat
import urllib.error
import urllib.request

def env(name):
    value = os.environ.get(name, "")
    if not value or "\r" in value or "\n" in value:
        raise RuntimeError("missing operator setting: " + name)
    return value.rstrip("/")

def main():
    base = env("OCAF_VAULT_ADDR")
    key = env("OCAF_VAULT_KEY")
    tenant = env("OCAF_E2E_TENANT")
    user = env("OCAF_E2E_USER")
    provider = env("OCAF_E2E_PROVIDER")
    token_path = Path(env("OCAF_VAULT_TOKEN_FILE"))
    if not base.startswith("https://") or "/" in key or not token_path.is_file() or token_path.is_symlink():
        raise RuntimeError("Vault must use HTTPS and a private token file")
    if stat.S_IMODE(token_path.stat().st_mode) & 0o077:
        raise RuntimeError("Vault token file permissions are not private")
    token = token_path.read_text(encoding="utf-8").strip()
    if not token or len(token) > 4096 or any(ord(ch) < 33 or ord(ch) > 126 for ch in token):
        raise RuntimeError("invalid Vault operator token")
    ctx = base64.b64encode(("ocaf:044:vault:v1\0"+tenant+"\0"+user+"\0"+provider).encode()).decode()
    plaintext = secrets.token_bytes(32)
    ctx_bad = base64.b64encode(("ocaf:044:vault:v1\0wrong-tenant\0"+user+"\0"+provider).encode()).decode()
    ssl_ctx = ssl.create_default_context()
    ssl_ctx.minimum_version = ssl.TLSVersion.TLSv1_3
    def call(operation, payload, expected=200):
        url = base + "/v1/transit/" + operation + "/" + key
        body = json.dumps(payload, separators=(",", ":")).encode()
        req = urllib.request.Request(url, data=body, method="POST",
            headers={"X-Vault-Token":token,"Content-Type":"application/json","Accept":"application/json"})
        opener = urllib.request.build_opener(urllib.request.HTTPSHandler(context=ssl_ctx))
        try:
            with opener.open(req, timeout=8) as response:
                status, raw = response.status, response.read(65537)
        except urllib.error.HTTPError as error:
            status, raw = error.code, error.read(65537)
        if len(raw) > 65536 or status != expected:
            raise RuntimeError("Vault operation or response rejected: " + operation)
        return json.loads(raw)
    enc = call("encrypt", {"plaintext":base64.b64encode(plaintext).decode(),"context":ctx})
    cipher = enc["data"]["ciphertext"]
    assert cipher.startswith("vault:v")
    dec = call("decrypt", {"ciphertext":cipher,"context":ctx})
    assert base64.b64decode(dec["data"]["plaintext"],validate=True) == plaintext
    rewrapped = call("rewrap", {"ciphertext":cipher,"context":ctx})["data"]["ciphertext"]
    dec2 = call("decrypt", {"ciphertext":rewrapped,"context":ctx})
    assert base64.b64decode(dec2["data"]["plaintext"],validate=True) == plaintext
    call("decrypt", {"ciphertext":cipher,"context":ctx_bad}, expected=400)
    print(json.dumps({"stage":"0.44","vault_transit":"PASS","context_isolation":"PASS",
        "rewrap":"PASS","hsm_hardware_attestation":"NOT_TESTED",
        "service_broker_integration":"NOT_TESTED"}))

if __name__ == "__main__":
    main()
