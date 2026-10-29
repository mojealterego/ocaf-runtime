import sys
import unittest
import json
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from import_sasos_tool_catalog import parse_catalog


def sample(number: int) -> str:
    suffix = f"{number:03d}"
    return f'''pub struct SasosTool{suffix};
impl OmaToolContract for SasosTool{suffix} {{
 fn tool_id(&self) -> &'static str {{ "SASOS_TOOL_{suffix}_SAMPLE" }}
 fn name(&self) -> &'static str {{ "Example" }}
 fn description(&self) -> &'static str {{ "inert metadata" }}
 fn input_schema(&self) -> ToolInputSchema {{ ToolInputSchema {{ required_fields: vec!["prompt"] }} }}
 fn connected_modules(&self) -> &'static [&'static str] {{ &["OMAT"] }}
 fn security_clearance(&self) -> SecurityClearance {{ SecurityClearance::Ring2Worker }}
 fn execution_runtime(&self) -> &'static str {{ "Cloud_API" }}
}}
'''


def source() -> str:
    return sample(1) + sample(2) + '''
tools.insert("SASOS_TOOL_001_SAMPLE", Box::new(SasosTool001));
tools.insert("SASOS_TOOL_002_SAMPLE", Box::new(SasosTool002));
'''


class CatalogImportTests(unittest.TestCase):
    def test_committed_catalog_cannot_claim_execution(self):
        catalog_path = Path(__file__).resolve().parents[1] / "omat/catalog/sasos_tools_001_555.json"
        entries = json.loads(catalog_path.read_text(encoding="utf-8"))
        self.assertEqual([entry["number"] for entry in entries], list(range(1, 556)))
        self.assertEqual(len({entry["tool_id"] for entry in entries}), 555)
        for entry in entries:
            self.assertEqual(entry["status"], "CATALOG_ONLY")
            self.assertIs(entry["executable"], False)
            self.assertIs(entry["imandra_verified"], False)
            self.assertIs(entry["zero_copy_verified"], False)
            self.assertIsNone(entry["measured_latency_microseconds"])

    def test_catalog_is_inert(self):
        entries = parse_catalog(source(), expected_count=2)
        self.assertEqual([entry["number"] for entry in entries], [1, 2])
        self.assertFalse(entries[0]["executable"])
        self.assertFalse(entries[0]["imandra_verified"])
        self.assertIsNone(entries[0]["measured_latency_microseconds"])

    def test_rejects_missing_definition(self):
        with self.assertRaisesRegex(ValueError, "every number"):
            parse_catalog(source().replace(sample(2), ""), expected_count=2)

    def test_rejects_registry_mismatch(self):
        with self.assertRaisesRegex(ValueError, "insertions differ"):
            parse_catalog(source().replace('tools.insert("SASOS_TOOL_002_SAMPLE", Box::new(SasosTool002));', ""), expected_count=2)


if __name__ == "__main__":
    unittest.main()
