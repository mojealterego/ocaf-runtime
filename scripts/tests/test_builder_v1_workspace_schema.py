"""Contract-only tests for INV-011 / REQ-020..025.

The JSON Schema checks structure and cardinality. It is NOT a DAG validator,
policy evaluator, sandbox implementation or compiled Rust evidence.
"""
from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import unittest

try:
    from jsonschema import Draft202012Validator
except ImportError:
    Draft202012Validator = None


SCHEMA = Path(__file__).resolve().parents[2] / 'schemas/agents_builder_v1_workspace.schema.json'


def sample():
    budget = {'max_steps': 50, 'max_duration_ms': 30_000, 'max_tokens': 16384,
              'max_cost_micros': 500000, 'max_cpu_ms': 5000,
              'max_ram_bytes': 128 * 1024 * 1024,
              'max_concurrency': 4, 'max_delegation_depth': 3}
    err = {'max_retries': 3, 'base_backoff_ms': 200, 'max_backoff_ms': 5000,
           'jitter_percent': 20, 'retryable_classes': ['transient_network'],
           'checkpoint_before_execution': True, 'idempotency_key_template': None}
    return {
        'workspace_id': 'test_ws', 'tenant_id': 'tenant_one', 'owner_id': 'operator',
        'schema_version': '1.0.0', 'revision': 1,
        'agents': {'agent_1': {'agent_id':'agent_1', 'name':'Test Agent',
                              'entry_graph_id':'workflow', 'global_budget':budget}},
        'graphs': {'workflow': {
            'graph_id':'workflow', 'profile':'dag_workflow',
            'nodes': {
              'start': {'node_id':'start', 'label':'Begin', 'node_type':'trigger',
                        'contract_version':'1.0', 'inputs':[],
                        'outputs':[{'port_name':'text', 'data_type':'string',
                                    'cardinality':'required-single'}],
                        'config':{},'capabilities':[], 'resource_budget':budget,
                        'error_policy':err,'ui_position':[80,160]},
              'end': {'node_id':'end','label':'Finish','node_type':'output',
                      'contract_version':'1.0',
                      'inputs':[{'port_name':'input','data_type':'string',
                                 'cardinality':'required-single'}],
                      'outputs':[],'config':{},'capabilities':[],
                      'resource_budget':budget,'error_policy':err,'ui_position':None},
            },
            'edges':[{'edge_id':'edge1','source_node':'start','source_port':'text',
                     'target_node':'end','target_port':'input','data_type':'string',
                     'cardinality':'required-single','is_bounded_backedge':False}]
        }},
        'tools':{},'policy_refs':[], 'secret_refs':{}, 'artifacts':[],
    }


@unittest.skipUnless(Draft202012Validator is not None, "jsonschema==4.26.0 required for schema-contract regressions")
class SchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads(SCHEMA.read_text(encoding='utf-8'))
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)

    def check(self, workspace):
        return list(self.validator.iter_errors(workspace))

    def test_round_trip_reference_fixture_conforms(self):
        ws=sample()
        self.assertEqual(self.check(ws), [])
        self.assertEqual(self.check(json.loads(json.dumps(ws,sort_keys=True))), [])

    def test_unknown_field_rejected(self):
        ws=sample();ws['graphs']['workflow']['nodes']['start']['elevate_admin']=True
        self.assertTrue(self.check(ws))

    def test_unknown_capability_rejected(self):
        ws=sample();ws['graphs']['workflow']['nodes']['start']['capabilities']=['os.root']
        self.assertTrue(self.check(ws))

    def test_secret_reference_is_typed_and_no_plaintext_secret_field(self):
        ws=sample();ws['secret_refs']['secret_1']={'secret_id':'secret_1','provider':'Keychain',
                                                  'key_path':'opaque/key'}
        self.assertEqual(self.check(ws),[])
        ws['secret_refs']['secret_1']['plaintext']='not-allowed'
        self.assertTrue(self.check(ws))

    def test_strict_schema_version(self):
        ws=sample();ws['schema_version']='1.1.0'
        self.assertTrue(self.check(ws))

    def test_reject_unbounded_loop(self):
        ws=sample();ws['graphs']['workflow']['nodes']['start']['node_type']={'loop':{'max_iterations':0}}
        self.assertTrue(self.check(ws))
        ws['graphs']['workflow']['nodes']['start']['node_type']={'loop':{'max_iterations':100}}
        self.assertEqual(self.check(ws),[])

    def test_reject_budget_overflow_or_zero(self):
        ws=sample();ws['graphs']['workflow']['nodes']['start']['resource_budget']['max_steps']=0
        self.assertTrue(self.check(ws))

    def test_reject_untyped_ports(self):
        ws=sample();ws['graphs']['workflow']['edges'][0]['data_type']='system-command'
        self.assertTrue(self.check(ws))

    def test_structured_schema_must_be_versioned(self):
        ws=sample();port=ws['graphs']['workflow']['nodes']['start']['outputs'][0]
        port['data_type']={'structured-schema':{'schema_id':'audit','schema_version':'1.0.0'}}
        self.assertEqual(self.check(ws),[])
        port['data_type']={'structured-schema':{'schema_id':'audit'}}
        self.assertTrue(self.check(ws))

    def test_no_hidden_permissions_on_tool_record(self):
        ws=sample();ws['tools']['external']={'tool_id':'external','contract_version':'1.0',
                                            'argv_schema':['string'], 'required_capability':'tool.http.egress',
                                            'secret_value':'abc'}
        self.assertTrue(self.check(ws))

    def test_unrecognized_node_kind_rejected(self):
        ws=sample();ws['graphs']['workflow']['nodes']['start']['node_type']='execute_as_root'
        self.assertTrue(self.check(ws))

    def test_cannot_add_unknown_root_fields(self):
        ws=sample();ws['admin']=True
        self.assertTrue(self.check(ws))


if __name__=='__main__':unittest.main()
