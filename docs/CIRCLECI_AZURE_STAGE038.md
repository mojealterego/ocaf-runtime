# OCAF 0.38 — CI Linux VM and Azure account integration

## CircleCI — user account exists, project/permissions unverified

`.circleci/config.yml` uses a **real x86_64 Ubuntu 24.04 Linux virtual machine**, not a Docker container.
Its `machine` executor is `medium` (2 vCPU / 7.5 GB), within CircleCI's Free-plan selectable classes *subject to the actual organization's plan*.
The plan offers credits for open-source Linux builds, but this repo's real entitlement is not verified.

Both jobs remain **OFF BY DEFAULT** via `run_stage038_linux: false`; connecting/importing the GitHub repository alone never triggers compute. When ready, run a pipeline with `run_stage038_linux: true`. The first job records actual PID 1/systemd and cgroup2 Linux host evidence; if it succeeds, the second runs five cycles of official signed-hash-verified Wasmtime integration.
No VM is persistent beyond the pipeline; passing these jobs is **not** stage 0.38 operator acceptance.

CircleCI login and GitHub connection must be completed on CircleCI by the account owner. An accessible CircleCI homepage URL does *not* grant this chat account privileges to operate its projects.

## Azure — user account exists, cloud subscription/OIDC entitlement unverified

`.github/workflows/stage038-azure-readonly.yml` is dispatch-only; it does not run automatically, create resources or authorize a billing change. It uses Azure/GitHub federated OIDC. The user must grant a scoped Azure read-only identity the official GitHub issuer
`https://token.actions.githubusercontent.com` with an appropriate audience `api://AzureADTokenExchange`, and subject `repo:mojealterego/ocaf-runtime:ref:refs/heads/main`. Provide the minimal Reader role scoped only to the intended resource group or subscription as appropriate.

Store `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID` as GitHub Actions secrets. Never place Azure passwords, bearer tokens, full IDs or credentials in chat, repository or CI logs.

The job validates the authenticated subscription context and counts existing VMs without listing VM names/IDs, making any deployments or changing Azure spending. Azure Free quotas/credits must be independently verified before requesting a deployment.

**All findings remain NO-GO** pending persistent systemd/Wasmtime, actual tenant/mTLS acceptance, failure injection and operator security signoff. Only `main` is used.
