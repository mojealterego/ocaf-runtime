#!/usr/bin/env python3
"""OCAF stage 0.40.1: independent runner-host PostgreSQL crash proof comparator.

Requires three distinct Linux kernel boot IDs (on three independently
scheduled GitHub Actions Ubuntu jobs), each running a REAL PostgreSQL 16
three-postmaster streaming failover lab. A three-way comparison validates
all real local primary SIGKILL/IMMEDIATE and replication receipts.

This is NOT inter-host PostgreSQL streaming, cross-host leader election,
host power-off, split brain or production multi-host HA.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re

import stage040_postgres_ha as pg


class MultiPgHostError(ValueError):
    pass


BOOT_ID=re.compile(r"[a-f0-9]{8}-[a-f0-9]{4}-[1-8][a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}\Z")
ROLES=frozenset(("a","b","c"))


def compare_three_hosts(reports:dict)->dict:
    if not isinstance(reports,dict) or set(reports)!=ROLES:
        raise MultiPgHostError("three distinct actual runner reports required")
    boots=[]
    for role in ("a","b","c"):
        report=reports[role]
        try:
            pg.validate_report(report)
        except (ValueError,TypeError) as exc:
            raise MultiPgHostError("false PostgreSQL failover status on runner "+role) from exc
        boot=report.get("kernel_boot_id")
        if (not isinstance(boot,str) or BOOT_ID.fullmatch(boot) is None
            or report.get("standby_c_wal_receiver_status")!="streaming"
            or report.get("three_physically_independent_hosts") is not False
            or report.get("host_power_loss_tested") is not False
            or report.get("interhost_postgres_streaming_verified") is True
            or report.get("stage_040_complete") is not False
            or report.get("production_approved") is not False):
            raise MultiPgHostError("kernel boot ID or failover proof scope is invalid on "+role)
        boots.append(boot)
    if len(set(boots))!=3:
        raise MultiPgHostError("three real distinct Linux kernel boot IDs required")
    return {
        "schema_version":1,
        "status":"THREE_REAL_DISTINCT_LINUX_KERNELS_LOCAL_POSTGRES_FAILOVERS_VERIFIED",
        "distinct_kernels_observed":3,
        "boot_ids":{role:boot for role,boot in zip(("a","b","c"),boots)},
        "postgresql_postmasters_executed_total":9,
        "three_real_pg_failovers_on_three_distinct_kernels":True,
        "real_local_wal_primary_crash_and_reparent_each_kernel":True,
        "interhost_postgres_streaming_verified":False,
        "host_power_loss_tested":False,
        "split_brain_fencing_tested":False,
        "production_approved":False,
        "stage_040_complete":False,
    }


def main()->int:
    parser=argparse.ArgumentParser(description=__doc__)
    for role in ("a","b","c"):
        parser.add_argument("--host-"+role,required=True,type=Path)
    parser.add_argument("--report",required=True,type=Path)
    opts=parser.parse_args()
    try:
        result=compare_three_hosts({
            role:json.loads(getattr(opts,"host_"+role).read_text())
            for role in ("a","b","c")})
        if opts.report.exists() or not opts.report.parent.is_dir():
            raise MultiPgHostError("existing or missing evidence output directory")
        with opts.report.open("x") as fp:
            json.dump(result,fp,sort_keys=True,indent=2)
            fp.write("\n")
    except (OSError,ValueError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status":result["status"],
          "interhost_postgres_streaming_verified":False,
          "production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
