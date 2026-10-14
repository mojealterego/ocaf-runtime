# Red Hat Developer Sandbox / OpenShift Virtualization — OCAF 0.38

## Target and authorization
Browser console: `https://console-openshift-console.apps.rm3.7wse.p1.openshiftapps.com/k8s/ns/mojealterego-dev/virtualization-overview`.

Only the namespace `mojealterego-dev` is in scope. The console URL is **not** an authenticated Kubernetes API endpoint, kubeconfig, access token or grant of RBAC permissions. Do not infer the API server URL from the console hostname. OpenShift permissions must be checked against a session authorised by the operator.

The private GitHub Actions workflow `.github/workflows/ocaf-stage038-redhat-sandbox.yml` supports a **read-only diagnostic** after the user separately authorizes a connection:
- `OCAF_SANDBOX_API_URL`: exact URL from authenticated `oc whoami --show-server` (HTTPS, port 6443, `*.openshiftapps.com`);
- `OCAF_SANDBOX_TOKEN`: short-lived, least-privilege token stored only as a GitHub Actions secret, not in issues, chat, commits or artifacts.

The workflow must run on `main` with `workflow_dispatch`; it does not create VMs, pods, service accounts, roles, PVs, projects, network routes, or paid resources. Its JSON artifact is sanitized and contains only counts, RBAC answers and an explicit NO-GO. No installation or cluster mutation is authorised by this workflow.

## Full OCAF acceptance needs a different execution path

1. With authenticated browser or authorised Kubernetes client, identify available pre-configured Red Hat VM templates and permitted CPU/RAM/disk quotas. Prefer a Red Hat-preconfigured Linux VM with systemd PID 1. Never guess the image tag.
2. Confirm create permission for `virtualmachines.kubevirt.io`, quota and no conflict with existing resources.
3. Provision **only** the approved single test VM and run time-bounded acceptance. Do not interrupt other workloads.
4. Within Red Hat's 60-minute continuous VM limit, measure PID 1, cgroup v2 controllers, signed WASIp1 / Wasmtime, tenant isolation, RPC/mTLS, restart and failure behavior; export redacted, hashable evidence.
5. Stop test resources safely. Record observed constraints and `NO-GO` until independent operational signoff.

Red Hat Developer Sandbox automatically shuts down Linux VMs after one hour of uninterrupted operation. It is not permanent HA hosting. Automatic deletion of OpenShift pods can occur after 12 hours. This lab can provide targeted guest OS security evidence, not full production approval.

## UI orchestration
ChatGPT's browser automation requires a separately signed-in browser profile for the Red Hat site; the user's mobile browser session or the existence of this public console URL does not authenticate external automation. Do not paste OpenShift tokens in chat. If browser profile is connected, inspect RBAC and available VM templates before attempting any provisioning.

Stage 0.38 is still **PARTIAL / NO-GO**. Stage 0.39 remains blocked.
