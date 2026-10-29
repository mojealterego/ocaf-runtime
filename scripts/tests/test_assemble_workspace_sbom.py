import json
import unittest
import test_verify_workspace_sbom as fixture
from scripts.assemble_workspace_sbom import assemble
from scripts.verify_workspace_sbom import SbomError


class Assemble(unittest.TestCase):
    setUp = fixture.WorkspaceSbomTests.setUp
    add_g01 = fixture.WorkspaceSbomTests.add_g01
    def test_standalone_inventory_keeps_graph_separate(self):
        member=self.add_g01()
        root_path=self.root/'bom.json'
        root=json.loads(root_path.read_text())
        root['components']=root['components'][:-1]
        root['dependencies']=[{'ref':'root','dependsOn':['ledger']}]
        root_path.write_text(json.dumps(root))
        path=member/'bom.json'
        bom=json.loads(path.read_text())
        bom['metadata']['component']['bom-ref']='g01'
        bom['components'][0]['bom-ref']='ledger'
        bom['components'][0]['licenses']=[{'license':{'id':'MIT'}}]
        bom['dependencies']=[{'ref':'g01','dependsOn':['ledger']}]
        path.write_text(json.dumps(bom))
        self.assertEqual(assemble(self.root)['status'],'PASS')
        result=json.loads(root_path.read_text())
        self.assertEqual(result['dependencies'][0],root['dependencies'][0])
        self.assertEqual(result['dependencies'][1]['dependsOn'],['workspace:swarms/g01:ledger'])
        self.assertEqual(result['components'][-1]['components'][0]['licenses'],bom['components'][0]['licenses'])
        with self.assertRaises(SbomError): assemble(self.root)
