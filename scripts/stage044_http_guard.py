"""HTTP client guard for stage 0.44 acceptance probes carrying bearer secrets.

urllib's default redirect handler can resend Authorization or X-Vault-Token to
the Location target. Acceptance probes must observe redirects as failures.
"""
import urllib.request


class RejectRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, newurl):
        return None


def guarded_opener(tls_context):
    return urllib.request.build_opener(
        urllib.request.HTTPSHandler(context=tls_context), RejectRedirect()
    )


def require(condition, observation):
    """Enforce acceptance checks even when Python runs with `-O`."""
    if not condition:
        raise RuntimeError("stage 0.44 acceptance failed: " + observation)
