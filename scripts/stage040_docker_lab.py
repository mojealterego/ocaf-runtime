#!/usr/bin/env python3
"""Collect a REAL three-process PostgreSQL lab report from disposable Docker.

All PostgreSQL servers share one host kernel. This is neither independent-host
HA nor a production quorum/fencing certificate. Requires Docker, not host
PostgreSQL. The complete JSON is collected before the container tmpfs vanishes.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import uuid

import stage040_postgres_ha as ha


class DockerLabError(ValueError):
    pass


SOURCES=("scripts/stage040_postgres_ha.py", "scripts/stage040_quorum.py",
         "ops/stage040/Dockerfile")
IMAGE_ID=re.compile(r"sha256:[0-9a-f]{64}\Z")
RUNTIME_FLAGS=("--rm", "--network=none", "--cap-drop=ALL",
               "--security-opt=no-new-privileges", "--memory=2g",
               "--pids-limit=256", "--read-only", "--user=postgres",
               "--tmpfs=/tmp:rw,nosuid,nodev,size=1g,mode=1777")
COLLECT_COMMAND=('umask 077; python3 /work/scripts/stage040_postgres_ha.py '
                 '"$@" >/tmp/status.json && cat /tmp/result.json')


def source_hashes(root:Path)->dict:
    digests={}
    for relative in SOURCES:
        path=root/relative
        if path.is_symlink() or not path.is_file():
            raise DockerLabError("regular lab source missing: "+relative)
        digests[relative]=hashlib.sha256(path.read_bytes()).hexdigest()
    return digests


def _command(argv:list[str],*,timeout:int)->subprocess.CompletedProcess:
    try:
        result=subprocess.run(argv,check=False,text=True,capture_output=True,
                              timeout=timeout)
    except (OSError,subprocess.TimeoutExpired) as exc:
        raise DockerLabError("Docker lab command unavailable or timed out") from exc
    if result.returncode or len(result.stdout)>2*1024*1024:
        # Do not emit raw Docker/harness output, which is not a public log API.
        raise DockerLabError("Docker lab command failed or output exceeded bound")
    return result


def _validate_requested_work(report:dict,seconds:int,witness:bool)->None:
    ha.validate_report(report)
    soak=report.get("pgbench_wal_soak")
    if (not isinstance(soak,dict)
        or soak.get("actual_pgbench_completed") is not True
        or type(soak.get("duration_seconds")) is not int
        or soak["duration_seconds"]!=seconds
        or soak.get("both_streaming_replicas_replayed_pgbench_dataset") is not True
        or soak.get("load_during_replication") is not True):
        raise DockerLabError("complete requested real pgbench/WAL observation missing")
    if witness:
        observed=report.get("witness_lab")
        if (not isinstance(observed,dict)
            or observed.get("authority")!="single_local_sqlite_witness"
            or observed.get("majority_consensus_tested") is not False
            or observed.get("harness_write_guard_only") is not True
            or observed.get("database_server_side_fencing_tested") is not False
            or observed.get("physical_host_fault_tested") is not False
            or observed.get("former_a_rejoined_by_pg_rewind") is not True
            or observed.get("physical_replication_slot_active_on_c") is not True
            or observed.get("former_a_wal_receiver_status")!="streaming"
            or observed.get("final_committed_event_ids")!="1,2,3,4,5"):
            raise DockerLabError("requested single-host witness observation missing")
    elif report.get("witness_lab") is not None:
        raise DockerLabError("unexpected witness exercise in container report")


def run_lab(*,report:Path,soak_seconds:int=30,exercise_witness:bool=True,
            repo_root:Path|None=None)->dict:
    if type(soak_seconds) is not int or not 15<=soak_seconds<=120:
        raise DockerLabError("soak must be an integer between 15 and 120 seconds")
    if type(exercise_witness) is not bool:
        raise DockerLabError("explicit boolean witness selection required")
    if (not isinstance(report,Path) or not report.is_absolute()
        or ".." in report.parts or report.exists() or report.is_symlink()
        or not report.parent.is_dir()):
        raise DockerLabError("new absolute report in an existing host directory required")
    root=Path(__file__).resolve().parents[1] if repo_root is None else repo_root
    if not isinstance(root,Path) or not root.is_absolute() or not root.is_dir():
        raise DockerLabError("absolute existing repository root required")
    sources=source_hashes(root)
    container_name="ocaf040-lab-"+uuid.uuid4().hex
    cleanup_ok=False
    try:
        built=_command(["docker","build","--quiet","--file",
                        str(root/"ops/stage040/Dockerfile"),str(root)],timeout=600)
        image_id=built.stdout.strip()
        if IMAGE_ID.fullmatch(image_id) is None:
            raise DockerLabError("Docker build did not return an immutable image ID")
        args=["--pg-bin-dir","/usr/lib/postgresql/16/bin",
              "--report","/tmp/result.json","--soak-seconds",str(soak_seconds),
              "--ack-local-ephemeral-postgres-primary-kill"]
        if exercise_witness:
            args.append("--exercise-witness")
        command=["docker","run","--name",container_name,*RUNTIME_FLAGS,
                 "--entrypoint","sh",image_id,"-c",COLLECT_COMMAND,
                 "ocaf040-collector",*args]
        observed=_command(command,timeout=soak_seconds+480)
        try:
            result=json.loads(observed.stdout)
        except (TypeError,ValueError) as exc:
            raise DockerLabError("container did not return its complete JSON report") from exc
        _validate_requested_work(result,soak_seconds,exercise_witness)
        if source_hashes(root)!=sources:
            raise DockerLabError("lab source changed while image/run evidence was collected")
        result["docker_collector"]={
            "schema_version":1,
            "collected_at_utc":datetime.now(timezone.utc).isoformat(),
            "scope":"single_host_kernel_three_postgresql_processes",
            "source_sha256":sources,"image_id":image_id,
            "runtime_flags":list(RUNTIME_FLAGS),
            "container_name":container_name,"host_uid":os.geteuid(),
            "full_report_collected_before_tmpfs_removal":True,
            "requested_soak_seconds":soak_seconds,
            "requested_witness_exercise":exercise_witness,
            "independent_hosts_verified":False,
            "production_approved":False,"stage_040_complete":False,
        }
    finally:
        # Docker may create a container before its client times out or errors.
        # Always target only our unpredictable dedicated name, including on
        # build/launch failure. --rm may already have removed it successfully.
        try:
            removed=subprocess.run(["docker","rm","--force",container_name],
                                   check=False,text=True,capture_output=True,timeout=30)
            if removed.returncode==0:
                cleanup_ok=True
            else:
                # A normally completed --rm container no longer exists. Verify
                # absence without hiding daemon/access errors as cleanup success.
                absent=subprocess.run(["docker","container","ls","--all",
                    "--filter","name=^/"+container_name+"$","--format","{{.Names}}"],
                    check=False,text=True,capture_output=True,timeout=15)
                cleanup_ok=absent.returncode==0 and not absent.stdout.strip()
        except (OSError,subprocess.TimeoutExpired):
            cleanup_ok=False
    if not cleanup_ok:
        raise DockerLabError("dedicated Docker container cleanup could not be verified")
    result["docker_collector"]["container_cleanup_verified"]=True
    fd=os.open(report,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
    with os.fdopen(fd,"w",encoding="utf-8") as sink:
        json.dump(result,sink,sort_keys=True,indent=2)
        sink.write("\n")
    return result


def main()->int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report",type=Path,required=True)
    parser.add_argument("--soak-seconds",type=int,default=30)
    parser.add_argument("--exercise-witness",action=argparse.BooleanOptionalAction,default=True)
    args=parser.parse_args()
    try:
        result=run_lab(report=args.report,soak_seconds=args.soak_seconds,
                       exercise_witness=args.exercise_witness)
    except (OSError,ValueError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status":result["status"],"report":str(args.report),
                      "production_approved":False,"stage_040_complete":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
