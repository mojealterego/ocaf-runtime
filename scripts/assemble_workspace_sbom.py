#!/usr/bin/env python3
"""Include standalone workspace applications without inventing root dependency edges."""
import argparse
import copy
import json
from pathlib import Path
from .verify_workspace_sbom import (_read_bom, _component_identity, SbomError,
                                    STANDALONE_BOM_DEPENDENCIES, verify)
from .supply_chain import REVIEWED_WORKSPACE_MEMBERS


def assemble(repo):
    root = _read_bom(repo / 'bom.json')
    original = (repo / 'bom.json').read_bytes()
    components = root.get('components')
    if not isinstance(components, list):
        raise SbomError('missing root components')
    for path in STANDALONE_BOM_DEPENDENCIES:
        expected = REVIEWED_WORKSPACE_MEMBERS[path]
        member = _read_bom(repo / path / 'bom.json')
        if _component_identity(member) != expected:
            raise SbomError('standalone identity mismatch')
        if any(c.get('name') == expected[0] for c in components):
            raise SbomError('standalone application already present')
        prefix = 'workspace:' + path + ':'
        def remap(value):
            if isinstance(value, list):
                return [remap(v) for v in value]
            if not isinstance(value, dict):
                return value
            result = {}
            for key, item in value.items():
                if key in ('bom-ref', 'ref') and isinstance(item, str):
                    result[key] = prefix + item
                elif key in ('dependsOn', 'provides') and isinstance(item, list):
                    result[key] = [prefix + v for v in item]
                else:
                    result[key] = remap(item)
            return result
        application = remap(copy.deepcopy(member['metadata']['component']))
        application['components'] = remap(member.get('components', []))
        components.append(application)
        # Keep G01's dependency graph separate: no root -> G01 edge.
        root.setdefault('dependencies', []).extend(remap(member.get('dependencies', [])))
    try:
        (repo / 'bom.json').write_text(json.dumps(root, indent=2)+'\n')
        return verify(repo)
    except Exception:
        (repo / 'bom.json').write_bytes(original)
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(assemble(args.repo)))
