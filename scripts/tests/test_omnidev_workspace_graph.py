"""0.56 — Cargo workspace graph: positive/negative dependency contract tests."""
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from omnidev_workspace_graph import GraphError, construct_graph


def meta(deps=None, member_names=("root", "engine")):
    deps = deps if deps is not None else {"root": ["engine"], "engine": []}
    packages = []
    for n in member_names:
        packages.append({
            "id": f"path+file:///repo/{n}#0.1.0",
            "name": n,
            "version": "0.1.0",
            "manifest_path": f"/repo/{n}/Cargo.toml",
            "dependencies": [
                {"name": d, "path": f"/repo/{d}", "source": None}
                for d in deps[n]
            ],
        })
    return {"workspace_root": "/repo", "workspace_members": [p["id"] for p in packages], "packages": packages}


class WorkspaceGraphTests(unittest.TestCase):
    def test_internal_dependency_graph_and_order(self):
        out = construct_graph(meta(), Path("/repo"))
        self.assertEqual(out["nodes"], ["engine", "root"])
        self.assertEqual(out["edges"], [{"from": "root", "to": "engine"}])
        self.assertEqual(out["topological_order"], ["engine", "root"])
        self.assertEqual(out["stage"], "0.56")
        self.assertEqual(len(out["graph_sha256"]), 64)

    def test_reordering_packages_cannot_change_fingerprint(self):
        first = meta()
        second = json.loads(json.dumps(first))
        second["packages"].reverse()
        self.assertEqual(construct_graph(first, Path("/repo")), construct_graph(second, Path("/repo")))

    def test_detects_cycles(self):
        with self.assertRaisesRegex(GraphError, "cycle"):
            construct_graph(meta({"root": ["engine"], "engine": ["root"]}), Path("/repo"))

    def test_rejects_missing_workspace_members(self):
        m = meta()
        m["workspace_members"].append("path+file:///repo/rogue#0.1.0")
        with self.assertRaises(GraphError):
            construct_graph(m, Path("/repo"))

    def test_rejects_path_escape(self):
        m = meta()
        m["packages"][1]["manifest_path"] = "/tmp/engine/Cargo.toml"
        with self.assertRaisesRegex(GraphError, "outside"):
            construct_graph(m, Path("/repo"))

    def test_rejects_duplicate_names_that_break_node_identity(self):
        m = meta()
        m["packages"][1]["name"] = "root"
        with self.assertRaises(GraphError):
            construct_graph(m, Path("/repo"))

    def test_rejects_unresolved_internal_dependency(self):
        m = meta({"root": ["missing"], "engine": []})
        with self.assertRaisesRegex(GraphError, "unresolved"):
            construct_graph(m, Path("/repo"))

    def test_rejects_relative_path_alias_without_matching_member(self):
        m = meta()
        m["packages"][0]["dependencies"][0]["path"] = "/repo/other"
        with self.assertRaisesRegex(GraphError, "unresolved"):
            construct_graph(m, Path("/repo"))


if __name__ == "__main__":
    unittest.main()
