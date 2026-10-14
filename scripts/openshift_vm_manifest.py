#!/usr/bin/env python3
"""Generate a bounded single-VM OpenShift Virtualization admission candidate."""
import argparse
import json
import re
from pathlib import Path

NAMESPACE = "mojealterego-dev"
NAME = "ocaf038-preflight"
SOURCE_NAMESPACE = "openshift-virtualization-os-images"

def build_manifest() -> dict:
    cloud_init = "#cloud-config\nssh_pwauth: false\ndisable_root: true\n"
    return {
        "apiVersion": "kubevirt.io/v1",
        "kind": "VirtualMachine",
        "metadata": {
            "name": NAME,
            "namespace": NAMESPACE,
            "labels": {"app.kubernetes.io/name": "ocaf-stage038", "ocaf.mojalterego/phase": "admission-only"},
        },
        "spec": {
            "runStrategy": "Halted",
            "dataVolumeTemplates": [{
                "metadata": {"name": NAME + "-disk"},
                "spec": {
                    "sourceRef": {"kind": "DataSource", "name": "fedora", "namespace": SOURCE_NAMESPACE},
                    "storage": {"resources": {"requests": {"storage": "30Gi"}}},
                },
            }],
            "template": {
                "metadata": {"labels": {"kubevirt.io/domain": NAME}},
                "spec": {
                    "domain": {
                        "cpu": {"cores": 1},
                        "resources": {"requests": {"cpu": "1", "memory": "2Gi"},
                                      "limits": {"cpu": "2", "memory": "3Gi"}},
                        "devices": {
                            "disks": [{"name": "rootdisk", "disk": {"bus": "virtio"}},
                                      {"name": "cloudinitdisk", "disk": {"bus": "virtio"}}],
                            "interfaces": [{"name": "default", "masquerade": {}}],
                        },
                    },
                    "networks": [{"name": "default", "pod": {}}],
                    "volumes": [{"name": "rootdisk", "dataVolume": {"name": NAME + "-disk"}},
                                {"name": "cloudinitdisk", "cloudInitNoCloud": {"userData": cloud_init}}],
                },
            },
        },
    }

def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--out", type=Path, required=True)
    args = p.parse_args()
    data = build_manifest()
    args.out.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print("Generated fixed-name, stopped VM admission candidate (not applied)")

if __name__ == "__main__":
    main()
