#!/usr/bin/env python3
"""Collect GitHub CI evidence for one commit; never authorize production."""
import argparse
from datetime import datetime, timezone
import json
import re
import subprocess
from pathlib import Path

WORKFLOWS = {
    'workspace': '.github/workflows/stage042-workspace-compat.yml',
    'supply_chain': '.github/workflows/supply-chain.yml',
    'rollback': '.github/workflows/stage042-rollback-drill.yml',
    'stateful_rollback': '.github/workflows/stage042-stateful-migration-rollback.yml',
}


def evaluate(sha, repository, runs):
    if not re.fullmatch('[0-9a-f]{40}', sha):
        raise ValueError('full source commit required')
    if not re.fullmatch('[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repository):
        raise ValueError('invalid repository')
    if set(runs) != set(WORKFLOWS):
        raise ValueError('all required workflow slots must be explicit')
    blockers, evidence, seen = [], {}, set()
    for label, path in WORKFLOWS.items():
        run = runs[label]
        if run is None:
            blockers.append(label + ':missing_run')
            continue
        rid=run.get('id')
        if type(rid) is not int or rid <= 0 or rid in seen:
            raise ValueError('invalid or repeated run ID')
        seen.add(rid)
        if (run.get('head_sha') != sha or run.get('path') != path
                or run.get('head_repository', {}).get('full_name') != repository
                or run.get('repository', {}).get('full_name') != repository
                or run.get('head_branch') != 'main'
                or run.get('event') not in ('push', 'workflow_dispatch')):
            raise ValueError('workflow provenance does not match candidate')
        evidence[label] = {key: run.get(key) for key in (
            'id','run_attempt','head_sha','path','status','conclusion','html_url')}
        if run.get('status') != 'completed' or run.get('conclusion') != 'success':
            blockers.append(label + ':not_successful')
    # CI validates its documented lab scope. A passing job cannot establish
    # acceptance of a durable deployment or comprehensive secret restoration.
    blockers.extend(['full_runtime_state_rollback_acceptance_missing',
                     'all_secret_providers_rollback_acceptance_missing',
                     'production_acceptance_evidence_missing'])
    return dict(schema_version=1, source_commit=sha, repository=repository,
                observed_at_utc=datetime.now(timezone.utc).isoformat(),
                status='NO_GO', production_approved=False,
                stage_042_complete=False, blockers=blockers, ci=evidence,
                scope='GitHub API run results; workflow-specific scopes apply',
                independent_audit_waived_by_owner=True,
                operator_signature_created=False)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--sha',required=True)
    p.add_argument('--repository',default='mojealterego/ocaf-runtime')
    for label in WORKFLOWS:
        p.add_argument('--'+label.replace('_','-')+'-run',type=int)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    # Validate identifiers before constructing API paths.
    evaluate(a.sha,a.repository,{label:None for label in WORKFLOWS})
    runs={}
    for label in WORKFLOWS:
        rid=getattr(a,label+'_run')
        if rid is None:
            runs[label]=None
            continue
        if rid <= 0: p.error('positive run ID required')
        raw=subprocess.run(['gh','api',f'repos/{a.repository}/actions/runs/{rid}'],
                           capture_output=True,check=True,timeout=30)
        runs[label]=json.loads(raw.stdout)
    result=evaluate(a.sha,a.repository,runs)
    with a.output.open('x') as out:
        json.dump(result,out,indent=2)
        out.write('\n')
    print(json.dumps({'status':result['status'],'source_commit':a.sha}))


if __name__=='__main__': main()
