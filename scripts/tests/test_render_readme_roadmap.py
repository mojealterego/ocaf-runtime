"""README first 269 numbered positions must match exact source inventory/Builder status."""
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import render_readme_roadmap as renderer

ROOT=Path(__file__).resolve().parents[2]

class FullReadmeRoadmapTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inv=json.loads((ROOT/"docs/roadmap/canonical_v11_inventory.json").read_text())
        cls.builders=json.loads((ROOT/"docs/roadmap/agents_builder_products_v11.json").read_text())
        cls.readme=(ROOT/"README.md").read_text()

    def test_very_first_visible_element_is_a_clickable_markdown_status_box(self):
        # README is what GitHub actually renders on the repository front page.
        # The clickable roadmap and five original Code/NoCode product pairs
        # must be in the first visible bordered table, NOT hidden below text.
        first = self.readme.replace(renderer.BEGIN, "", 1).lstrip()
        self.assertTrue(
            first.startswith("| 📌 **ROADMAPA OCAF"),
            "top of GitHub README must render the roadmap as a Markdown table",
        )
        before_main_heading=first.split("# OCAF — PEŁNA ROADMAPA",1)[0]
        self.assertIn("[OTWÓRZ PEŁNĄ ROADMAPĘ](ROADMAP.md)",before_main_heading)
        self.assertIn("195 etapów",before_main_heading)
        self.assertIn("64 podetapy",before_main_heading)
        self.assertIn("269",before_main_heading)
        for platform in ("Chmurowy","Desktopowy Windows","Desktopowy macOS",
                         "Android","iOS"):
            row=next((x for x in before_main_heading.splitlines()
                      if f"Agents Builder {platform}" in x),None)
            self.assertIsNotNone(row,f"missing original Agents Builder {platform}")
            self.assertIn("Code",row)
            self.assertIn("NoCode",row)
            self.assertIn("DO ZROBIENIA",row)
        self.assertIn("WASI",before_main_heading)
        self.assertIn("PostgreSQL",before_main_heading)

    def test_forged_pinned_panel_or_missing_roadmap_link_fails_validator(self):
        for bad in (
            self.readme.replace("[OTWÓRZ PEŁNĄ ROADMAPĘ](ROADMAP.md)",
                                "[NIE MA](somewhere.md)",1),
            self.readme.replace("Agents Builder Desktopowy macOS","Agent macOS",1),
        ):
            with self.assertRaises(renderer.ReadmeRoadmapError):
                renderer.validate_readme(bad,self.inv,self.builders)

    def test_every_number_1_to_269_at_top_with_executable_gate_and_status(self):
        result=renderer.validate_readme(self.readme,self.inv,self.builders)
        self.assertEqual(result["numbered_items"],269)
        self.assertEqual(result["main_stage_items"],195)
        self.assertEqual(result["technical_substage_items"],64)
        self.assertEqual(result["original_agents_builder_variants"],10)
        self.assertFalse(result["production_approved"])

    def test_original_agents_builder_android_ios_cloud_windows_macos_code_nocode_preserved(self):
        head=self.readme.split(renderer.END,1)[0]
        for item in self.builders["products"]:
            self.assertEqual(head.count(f"**{item['id']} —"),1)

    def test_unsupported_status_forgery_or_missing_number_denied(self):
        prefix=renderer.render_section(self.inv,self.builders)
        for bad in (
            self.readme.replace("1. **[DO ZROBIENIA]**","1. **[ZROBIONE]**",1),
            self.readme.replace("269. **[DO ZROBIENIA]**","269. **[ZROBIONE]**",1),
            self.readme.replace("195. **[DO ZROBIENIA]**","",1),
            self.readme.replace("261. **[DO ZROBIENIA]**","262. **[DO ZROBIENIA]**",1),
            self.readme.replace(renderer.BEGIN,"",1),
            self.readme.replace(renderer.END,"",1),
        ):
            with self.subTest(sample=bad[:100]),self.assertRaises(renderer.ReadmeRoadmapError):
                renderer.validate_readme(bad,self.inv,self.builders)

    def test_manifest_status_change_must_rewrite_readme_after_each_stage(self):
        import copy
        new=copy.deepcopy(self.inv)
        new["main"][55]["status"]="PARTIALLY_VERIFIED"
        with self.assertRaises(renderer.ReadmeRoadmapError):
            renderer.validate_readme(self.readme,new,self.builders)

if __name__=="__main__":
    unittest.main()
