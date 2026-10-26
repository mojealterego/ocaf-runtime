#!/usr/bin/env python3
"""Live OAuth network acceptance: fail closed without a real issuer and token."""
import concurrent.futures
import json
import os
from pathlib import Path
import ssl
import stat
import urllib.error
import urllib.request
from stage044_http_guard import guarded_opener, require

def required(name):
    value = os.environ.get(name, "")
    if not value or "\n" in value or "\r" in value:
        raise RuntimeError("missing or invalid configuration: " + name)
    return value.rstrip("/")

def fetch(url, method="GET", token=None):
    ctx = ssl.create_default_context()
    ctx.minimum_version = ssl.TLSVersion.TLSv1_3
    headers = {"Accept": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + token
    req = urllib.request.Request(url, method=method, headers=headers)
    opener = guarded_opener(ctx)
    try:
        with opener.open(req, timeout=8) as resp:
            body = resp.read(65537)
            if len(body) > 65536:
                raise RuntimeError("oversized response")
            return resp.status, body
    except urllib.error.HTTPError as err:
        body = err.read(65537)
        if len(body) > 65536:
            raise RuntimeError("oversized error response")
        return err.code, body

def main():
    issuer = required("OCAF_E2E_ISSUER")
    gateway = required("OCAF_E2E_GATEWAY")
    tenant = required("OCAF_E2E_TENANT")
    user = required("OCAF_E2E_USER")
    resource = required("OCAF_E2E_RESOURCE")
    path = Path(required("OCAF_E2E_TOKEN_FILE"))
    if not all(v.startswith("https://") for v in (issuer, gateway, resource)):
        raise RuntimeError("HTTPS is mandatory")
    if not path.is_file() or path.is_symlink() or stat.S_IMODE(path.stat().st_mode) & 0o077:
        raise RuntimeError("token file must be owner-only and not a symlink")
    token = path.read_text(encoding="utf-8").strip()
    if not token or len(token) > 16384 or any(ch.isspace() for ch in token):
        raise RuntimeError("invalid test token")
    status, data = fetch(issuer + "/.well-known/openid-configuration")
    require(status == 200, "issuer discovery status")
    meta = json.loads(data)
    require(meta["issuer"] == issuer, "issuer identity")
    require(meta["authorization_endpoint"].startswith(issuer + "/"), "authorization endpoint")
    require(meta["token_endpoint"].startswith(issuer + "/"), "token endpoint")
    require("code" in meta["response_types_supported"], "authorization code support")
    require("authorization_code" in meta["grant_types_supported"], "grant type support")
    require("S256" in meta.get("code_challenge_methods_supported", []), "PKCE S256 support")
    status, data = fetch(gateway + "/.well-known/oauth-protected-resource")
    require(status == 200, "protected resource metadata status")
    protected = json.loads(data)
    require(protected["resource"] == resource, "protected resource audience")
    require(issuer in protected["authorization_servers"], "protected resource issuer")
    endpoint = gateway + "/omat/" + tenant + "/users/" + user + "/authorize"
    require(fetch(endpoint, "POST")[0] == 401, "missing bearer denied")
    require(fetch(endpoint, "POST", "invalid.jwt.token")[0] in (401,403), "malformed bearer denied")
    require(fetch(endpoint, "POST", token)[0] == 204, "authorized bearer accepted")
    require(fetch(gateway + "/omat/wrong-tenant/users/" + user + "/authorize", "POST", token)[0] == 403, "wrong tenant denied")
    require(fetch(gateway + "/omat/" + tenant + "/users/wrong-user/authorize", "POST", token)[0] == 403, "wrong user denied")
    def negative(index):
        status, _ = fetch(endpoint, "POST", "invalid.jwt.token" + str(index))
        return status in (401,403)
    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool:
        require(all(pool.map(negative, range(48))), "negative-load bypass")
    print(json.dumps({"stage":"0.44","network":"PASS","negative_requests":48,
        "tls":"1.3","consent_mfa":"NOT_TESTED","kms_hsm":"NOT_TESTED",
        "external_refresh":"NOT_TESTED"}))

if __name__ == "__main__":
    main()
