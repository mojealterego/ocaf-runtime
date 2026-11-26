"""Real filesystem and CLI negative checks for fail-closed 111 fleet validation."""
from __future__ import annotations

import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from scripts.validate_agent_fleet_111 import REQUIRED, FleetValidationError, validate

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'scripts/validate_agent_fleet_111.py'


class FleetGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix='ocaf-fleet-gate-')
        cls.root = Path(cls.temp.name)
        cls.registry_path = cls.root / 'docs/agents/agent_registry_111.json'
        cls.registry_path.parent.mkdir(parents=True)
        cls.agents = []
        for i in range(1, 112):
            agent_id = f'OCAF-AGENT-{i:03d}'
            name = f'Agent{i}'
            subdir = f'{agent_id}-{name.lower()}'
            package_name = f'ocaf-agent-{i:03d}'
            cls.agents.append(dict(agent_id=agent_id, display_name=name,
                group_id=f'G{min(i, 33):02d}',package_name=package_name,
                package_path='AGENT-FLEET-111/'+subdir,lifecycle='DRAFT',
                deployment_status='NOT_DEPLOYED',artifact_status='NOT_BUILT',
                model_backend='UNASSIGNED',source_access='disabled',
                active_admission='DENY',ostc_enabled=False))
            parent = cls.root / 'OCAF Agents MOJEALTEREGO/AGENT-FLEET-111' / subdir
            for filename in REQUIRED:
                target = parent / filename
                target.parent.mkdir(parents=True,exist_ok=True)
                target.write_text('expected source\n')
            (parent/'config/agent.toml').write_text(
                f'agent_id = "{agent_id}"\ndisplay_name = "{name}"\n'
                'ostc_enabled = false\nsource_access = "disabled"\nactive_admission = "DENY"\n')
            (parent/'Cargo.toml').write_text(f'name = "{package_name}"\n')
            (parent/'src/lib.rs').write_text(f'pub const DISPLAY_NAME: &str = "{name}";\n')
        cls.registry = dict(schema_version=1,fleet_id='OCAF-FLEET-111',
            declared_agent_count=111,operational_swarm_count=33,
            active_runtime_agent_count=0,naming_contract={
                'display_name_field':'display_name','technical_id_field':'agent_id',
                'package_name_field':'package_name','names_unique':True},agents=cls.agents)
        cls.restore_registry()

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    @classmethod
    def restore_registry(cls):
        cls.registry_path.write_text(json.dumps(cls.registry),encoding='utf-8')

    def tearDown(self):
        self.restore_registry()

    def test_real_111_package_fixture_is_structurally_valid_only(self):
        result=validate(self.root)
        self.assertEqual(result['registered_agents'],111)
        self.assertFalse(result['implementation_verified'])
        self.assertFalse(result['production_approved'])
        self.assertEqual(result['active_runtime_agents'],0)

    def test_python_optimized_mode_must_reject_forged_active_runtime(self):
        forged=copy.deepcopy(self.registry)
        forged['active_runtime_agent_count']=1
        self.registry_path.write_text(json.dumps(forged),encoding='utf-8')
        for opt in ([],['-O']):
            with self.subTest(opt=opt):
                p=subprocess.run([sys.executable,*opt,str(SCRIPT),'--root',str(self.root)],
                    capture_output=True,text=True,timeout=15)
                self.assertNotEqual(p.returncode,0,p.stdout)
                self.assertIn('active runtime',p.stderr)

    def test_reject_forged_agent_states_without_assertions(self):
        for key,value in (('active_admission','ALLOW'),('ostc_enabled',True),
                ('deployment_status','DEPLOYED'),('artifact_status','BUILT'),
                ('model_backend','fake'),('source_access','enabled')):
            with self.subTest(key=key):
                forged=copy.deepcopy(self.registry)
                forged['agents'][0][key]=value
                self.registry_path.write_text(json.dumps(forged),encoding='utf-8')
                with self.assertRaisesRegex(FleetValidationError,'admission'):
                    validate(self.root)
        self.restore_registry()

    def test_reject_duplicate_and_out_of_order_agent(self):
        forged=copy.deepcopy(self.registry)
        forged['agents'][1]['agent_id']=forged['agents'][0]['agent_id']
        self.registry_path.write_text(json.dumps(forged),encoding='utf-8')
        with self.assertRaisesRegex(FleetValidationError,'ID or order'):
            validate(self.root)

    def test_reject_symlinked_required_source(self):
        file=self.root/'OCAF Agents MOJEALTEREGO/AGENT-FLEET-111/OCAF-AGENT-001-agent1/src/lib.rs'
        contents=file.read_text();file.unlink()
        outside=self.root/'outside.rs';outside.write_text(contents)
        file.symlink_to(outside)
        try:
            with self.assertRaisesRegex(FleetValidationError,'symlink'):
                validate(self.root)
        finally:
            file.unlink();file.write_text(contents);outside.unlink()

    def test_reject_missing_required_component(self):
        file=self.root/'OCAF Agents MOJEALTEREGO/AGENT-FLEET-111/OCAF-AGENT-001-agent1/tests/security.rs'
        contents=file.read_text();file.unlink()
        try:
            with self.assertRaises((FleetValidationError,FileNotFoundError)):
                validate(self.root)
        finally:
            file.write_text(contents)

    def test_reject_path_traversal_even_when_dir_exists(self):
        forged=copy.deepcopy(self.registry)
        forged['agents'][0]['package_path']='AGENT-FLEET-111/../../outside'
        self.registry_path.write_text(json.dumps(forged),encoding='utf-8')
        with self.assertRaisesRegex(FleetValidationError,'unsafe package path'):
            validate(self.root)

    def test_reject_duplicate_json_keys(self):
        self.registry_path.write_text('{"schema_version":1,"schema_version":1}',encoding='utf-8')
        with self.assertRaisesRegex(FleetValidationError,'duplicate JSON'):
            validate(self.root)

    def test_success_cli_normal_and_optimized(self):
        for opt in ([],['-O']):
            with self.subTest(opt=opt):
                p=subprocess.run([sys.executable,*opt,str(SCRIPT),'--root',str(self.root)],
                    capture_output=True,text=True,timeout=15)
                self.assertEqual(p.returncode,0,p.stdout+p.stderr)
                self.assertIn('PASS_STRUCTURE_ONLY',p.stdout)


if __name__=='__main__':
    unittest.main()
