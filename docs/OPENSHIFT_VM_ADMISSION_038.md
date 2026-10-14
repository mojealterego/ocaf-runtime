# OpenShift VM admission preflight, without provisioning

The workflow `.github/workflows/ocaf-stage038-redhat-vm-admission.yml` checks the **server-side admission** of a bounded, stopped VM spec on the existing OpenShift cluster:
- Project: `mojealterego-dev`; name: `ocaf038-preflight`;
- Fedora DataSource provided by Red Hat (`openshift-virtualization-os-images/fedora`);
- CPU request 1, limit 2; RAM request 2Gi, limit 3Gi; disk 30Gi;
- `runStrategy: Halted`, root login disabled, no SSH password, no external route;
- API: `kubectl create --dry-run=server`. **No object is persisted.**

This step is not a VM deployment or proof of systemd. The user requested full automation, but an admission test is an explicit safety stage before making a 60-minute VM resource request in the free Sandbox. It fails closed and never emits Kubernetes stdout/stderr that might contain tenant data.

If server dry run passes, a separate deployment workflow with a strict automatic cleanup, one VM at a time, will be required. Stage 0.38 is still NO-GO.
