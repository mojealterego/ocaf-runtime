"""OCAF v11 gate: restore original Code/NoCode Agents Builder products and pin full Roadmap."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import validate_roadmap_v11 as guard

ROOT=Path(__file__).resolve().parents[2]

class RestoreOriginalBuilderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        read=lambda p:(ROOT/p).read_text(encoding="utf-8")
        cls.inv=json.loads(read("docs/roadmap/canonical_v11_inventory.json"))
        cls.old=json.loads(read("docs/roadmap/canonical_v10_inventory.json"))
        cls.root=read("ROADMAP.md")
        cls.mirror=read("docs/ROADMAP_FULL_ECOSYSTEM_2026.md")
        cls.readme=read("README.md")
        cls.builders=json.loads(read("docs/roadmap/agents_builder_products_v11.json"))
        cls.losses=json.loads(read("docs/roadmap/missing_capabilities_v11.json"))
        cls.ledger=read("docs/roadmap/STAGE_CHANGELOG.md")
        cls.archive=read("docs/archive/ROADMAP_FULL_ECOSYSTEM_v9_2026-10-15.md")

    def verify(self,**updates):
        args=dict(v11=copy.deepcopy(self.inv),v10_data=copy.deepcopy(self.old),
                  root=self.root,mirror=self.mirror,readme=self.readme,
                  builders=copy.deepcopy(self.builders),
                  losses=copy.deepcopy(self.losses),ledger=self.ledger,
                  archive=self.archive)
        args.update(updates)
        return guard.validate(**args)

    def test_exact_333_64_33_no_lost_stages_and_all_original_products(self):
        result=self.verify()
        self.assertEqual(result["main_stages"],334)
        self.assertEqual(result["technical_substages"],64)
        self.assertEqual(result["distinct_agents_builder_products"],10)
        self.assertEqual(result["scope_gaps_audited"],24)
        self.assertEqual(result["next_new_main_stage"],"335")
        self.assertTrue(result["canonical_root_roadmap_pinned"])
        self.assertFalse(result["production_approved"])

    def test_missing_windows_or_macos_or_mobile_variant_is_fatal(self):
        for platform,mode in (("Cloud","Code"),("Windows","NoCode"),
                              ("macOS","Code"),("Android","NoCode"),("iOS","Code")):
            b=copy.deepcopy(self.builders)
            b["products"]=[p for p in b["products"]
                           if not (p["platform"]==platform and p["mode"]==mode)]
            with self.subTest(platform=platform,mode=mode),self.assertRaises(guard.CanonicalV11Error):
                self.verify(builders=b)

    def test_collapsing_desktop_or_mobile_under_one_builder_is_rejected(self):
        b=copy.deepcopy(self.builders)
        b["products"][4]["platform"]="Windows"
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(builders=b)
        b=copy.deepcopy(self.builders)
        b["products"][7]["mode"]="Code"
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(builders=b)

    def test_preexisting_builder_milestone_ids_must_not_be_reallocated(self):
        inv=copy.deepcopy(self.inv)
        inv["main"][52]["id"]="1.95"
        with self.assertRaises((guard.CanonicalV11Error,ValueError)):
            self.verify(v11=inv)
        inv=copy.deepcopy(self.inv)
        inv["main"][53]["previous_scope"]="invented history"
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(v11=inv)

    def test_no_fake_builder_prod_claim_or_non_builder_history_edit(self):
        b=copy.deepcopy(self.builders)
        b["products"][0]["status"]="IMPLEMENTED_VERIFIED"
        b["products"][0]["implemented_verified"]=True
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(builders=b)
        inv=copy.deepcopy(self.inv)
        inv["main"][40]["status"]="PRODUCTION_COMPLETE"
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(v11=inv)

    def test_technical_stage_identity_and_unrevised_scope_are_protected(self):
        inv=copy.deepcopy(self.inv)
        inv["technical"][0]["id"]="0.38.deleted"
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(v11=inv)
        inv=copy.deepcopy(self.inv)
        stage=next(row for row in inv["technical"] if row["id"]=="0.39.1")
        stage["scope"]="silently replaced scope"
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(v11=inv)

    def test_root_pin_mirror_and_scope_audit_are_mandatory(self):
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(root=self.root.replace("## Najważniejsze: pierwotny AGENTS BUILDER","## Deleted"))
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(readme=self.readme.replace("| **329** |", "| **MISSING** |", 1))
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(mirror=self.mirror.replace("| **0.39.10** |", "| **MISSING** |", 1))
        loss=copy.deepcopy(self.losses);loss["items"].pop()
        with self.assertRaises(guard.CanonicalV11Error):
            self.verify(losses=loss)

    def test_restored_products_are_tools_to_build_agents_not_agents(self):
        self.assertEqual(self.builders["category"],"AGENTS_BUILDER_PRODUCTS_NOT_AGENTS")
        self.assertEqual(set((x["platform"],x["mode"]) for x in self.builders["products"]),
            {(p,m) for p in ("Cloud","Windows","macOS","Android","iOS")
                    for m in ("Code","NoCode")})
        self.assertTrue(all(x["status"]=="NOT_IMPLEMENTED" for x in self.builders["products"]))

if __name__=="__main__":
    unittest.main()
