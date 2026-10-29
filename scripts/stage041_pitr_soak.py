#!/usr/bin/env python3
"""Repeated real local PG crash/PITR; no offsite or integrated-runtime claim."""
import argparse
import json
import math
from pathlib import Path
import time
from stage041_pitr import execute, validate_pitr


def summarize(cycles, elapsed, requested):
    if type(requested) is not int or requested not in (30, 1800, 3600, 7200):
        raise ValueError('expected 30, 60 or 120 minutes')
    if not isinstance(elapsed, (int, float)) or isinstance(elapsed, bool) or not math.isfinite(elapsed) or elapsed < requested:
        raise ValueError('requested wall-clock duration not reached')
    if not cycles:
        raise ValueError('no completed real recovery cycles')
    timings = []
    for cycle in cycles:
        validate_pitr(cycle)
        value = cycle.get('crash_to_verified_restore_ms')
        if type(value) is not int or value < 0:
            raise ValueError('missing measured recovery duration')
        if (cycle.get('missing_target_rows') != 0
                or cycle.get('deliberately_excluded_post_target_rows') != 5
                or cycle.get('rpo_scope') != 'exact_selected_lsn_not_latest_pre_crash_commit'
                or 'rpo_seconds' not in cycle or cycle['rpo_seconds'] is not None):
            raise ValueError('invalid target-loss or time-based RPO claim')
        timings.append(value)
    timings.sort()
    return dict(schema_version=1, status=('LOCAL_PITR_SMOKE_PASS' if requested==30
                                         else 'LOCAL_REPEATED_PITR_SOAK_PASS'),
                requested_seconds=requested, measured_elapsed_seconds=elapsed,
                completed_cycles=len(cycles),
                rto_p50_ms=timings[math.ceil(len(timings)*.50)-1],
                rto_p95_ms=timings[math.ceil(len(timings)*.95)-1],
                rto_p99_ms=timings[math.ceil(len(timings)*.99)-1],
                rto_max_ms=max(timings), missing_target_rows=0,
                deliberately_excluded_rows_per_cycle=5, rpo_seconds=None,
                scope='repeated_local_database_crash_and_selected_lsn_restore',
                continuous_client_load_tested=False, offsite_tested=False,
                telemetry_backpressure_tested=False, guest_quarantine_tested=False,
                integrated_broker_vm_database_fault_tested=False,
                stage_041_complete=False, production_approved=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pg-bin-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--seconds', type=int, choices=(30,1800,3600,7200), required=True)
    parser.add_argument('--ack-ephemeral-postgres-pitr-crash-lab', action='store_true')
    args = parser.parse_args()
    if not args.ack_ephemeral_postgres_pitr_crash_lab:
        parser.error('explicit disposable local PostgreSQL crash lab required')
    if not args.output.is_absolute() or args.output.is_symlink():
        parser.error('absolute new private output directory required')
    args.output.mkdir(mode=0o700, exist_ok=False)
    start = time.monotonic()
    cycles = []
    try:
        while time.monotonic()-start < args.seconds:
            result = execute(args.pg_bin_dir, args.output/f'cycle-{len(cycles):04d}.json')
            cycles.append(result)
            print(json.dumps({'completed_cycles':len(cycles),
                              'elapsed_seconds':int(time.monotonic()-start)}), flush=True)
        report = summarize(cycles, time.monotonic()-start, args.seconds)
    except Exception as error:
        (args.output/'failure.json').write_text(json.dumps(dict(
            error_type=type(error).__name__, error_message=str(error)[:300],
            status='FAILED', completed_cycles=len(cycles),
            measured_elapsed_seconds=time.monotonic()-start,
            stage_041_complete=False, production_approved=False)))
        print(f'PITR SOAK FAILED: {type(error).__name__}: {str(error)[:300]}', flush=True)
        raise
    with (args.output/'summary.json').open('x') as out:
        json.dump(report, out, indent=2)
        out.write('\n')


if __name__ == '__main__':
    main()
