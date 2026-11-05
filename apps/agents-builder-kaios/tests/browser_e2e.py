#!/usr/bin/env python3
"""Chromium DOM E2E for KaiOS UI. Requires playwright and Chromium.
Uses an in-memory localStorage shim because sandboxed about:blank denies Storage.
Not a KaiOS device installation test.
"""
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
html = (ROOT / "index.html").read_text(encoding="utf-8")
html = html.replace('<link rel="stylesheet" href="style.css">', "<style>" + (ROOT / "style.css").read_text(encoding="utf-8") + "</style>")
html = html.replace('<script src="canonical-graph.js"></script>', "<script>" + (ROOT / "canonical-graph.js").read_text(encoding="utf-8") + "</script>")
html = html.replace('<script src="app.js"></script>', "<script>" + (ROOT / "app.js").read_text(encoding="utf-8") + "</script>")
html = html.replace('<script src="canonical-ui.js"></script>', "<script>" + (ROOT / "canonical-ui.js").read_text(encoding="utf-8") + "</script>")
assert "<script>" in html and "<style>" in html

with sync_playwright() as playwright:
    browser = playwright.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 240, "height": 320})
    page.evaluate("""() => {
        const store = {};
        Object.defineProperty(window, 'localStorage', { configurable: true, value: {
            getItem: key => Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null,
            setItem: (key, value) => { store[key] = String(value); }
        }});
    }""")
    page.set_content(html)
    page.locator("#node-name").fill("first")
    page.locator("#add").click()
    assert page.locator("#nodes li").count() == 1
    page.locator("#nodes li button").click()
    assert page.locator("#nodes li").count() == 0
    page.locator("#node-name").fill("persisted")
    page.locator("#add").click()
    page.locator("#save").click()
    page.set_content(html)
    page.locator("#load").click()
    assert page.locator("#nodes li").count() == 1
    assert "persisted" in page.locator("#nodes li").inner_text()
    page.locator("#code").click()
    page.locator("#source").fill('{"version":1,"project_id":"agent","nodes":[{"id":"second"}]}')
    page.locator("#save").click()
    page.set_content(html)
    page.locator("#load").click()
    assert "second" in page.locator("#nodes li").inner_text()
    canonical = {
        "schema_version": 1, "tenant_id": "local", "project_id": "agent", "revision": 1,
        "max_cost_minor": 0, "grants": [],
        "nodes": [
            {"id": "start", "kind": "input", "inputs": [], "outputs": [{"name": "out", "kind": "text"}], "tool_name": None, "max_cost_minor": 0},
            {"id": "finish", "kind": "output", "inputs": [{"name": "in", "kind": "text"}], "outputs": [], "tool_name": None, "max_cost_minor": 0}
        ],
        "edges": [{"from_node": "start", "from_port": "out", "to_node": "finish", "to_port": "in"}]
    }
    import json
    page.locator("#canonical-source").fill(json.dumps(canonical))
    page.locator("#canonical-check").click()
    assert json.loads(page.locator("#canonical-result").inner_text())["ordered_nodes"] == ["start", "finish"]
    page.locator("#canonical-tenant").fill("unauthorized")
    page.locator("#canonical-check").click()
    assert "Tenant denied" in page.locator("#canonical-result").inner_text()
    page.locator("#canonical-tenant").fill("local")
    canonical["edges"][0]["to_port"] = "unknown"
    page.locator("#canonical-source").fill(json.dumps(canonical))
    page.locator("#canonical-check").click()
    assert "Invalid edge port" in page.locator("#canonical-result").inner_text()
    page.locator("#canonical-new").click()
    assert json.loads(page.locator("#canonical-source").input_value())["edges"][0]["to_node"] == "output"
    page.locator("#canonical-node-id").fill("step")
    page.locator("#canonical-transform").click()
    assert json.loads(page.locator("#canonical-result").inner_text())["ordered_nodes"] == ["input", "step", "output"]
    page.locator("#canonical-remove").click()
    assert json.loads(page.locator("#canonical-result").inner_text())["ordered_nodes"] == ["input", "output"]
    page.locator("#canonical-transform").click()
    assert json.loads(page.locator("#canonical-result").inner_text())["ordered_nodes"] == ["input", "step", "output"]
    page.locator("#canonical-save").click()
    page.locator("#canonical-source").fill("")
    page.locator("#canonical-load").click()
    assert len(json.loads(page.locator("#canonical-source").input_value())["edges"]) == 2
    page.locator("#canonical-port-kind").select_option("json")
    page.locator("#canonical-new").click()
    assert json.loads(page.locator("#canonical-source").input_value())["nodes"][0]["outputs"][0]["kind"] == "json"
    page.locator("#canonical-transform").click()
    assert json.loads(page.locator("#canonical-source").input_value())["nodes"][-1]["inputs"][0]["kind"] == "json"
    page.locator("#canonical-remove").click()
    assert json.loads(page.locator("#canonical-result").inner_text())["ordered_nodes"] == ["input", "output"]
    page.locator("#canonical-export").click()
    exported = page.locator("#canonical-import").input_value()
    assert json.loads(exported)["project_id"] == "agent"
    page.locator("#canonical-source").fill("")
    page.locator("#canonical-import-button").click()
    assert json.loads(page.locator("#canonical-source").input_value())["project_id"] == "agent"
    page.locator("#canonical-import").fill(exported.replace('"tenant_id": "local"', '"tenant_id": "other"'))
    page.locator("#canonical-import-button").click()
    assert "Tenant denied" in page.locator("#canonical-result").inner_text()
    page.locator("#canonical-port-kind").select_option("text")
    page.locator("#canonical-new").click()
    page.locator("#canonical-node-id").fill("tool1")
    page.locator("#canonical-tool-name").fill("search.tool")
    page.locator("#canonical-tool-cost").fill("7")
    page.locator("#canonical-tool").click()
    tool_graph = json.loads(page.locator("#canonical-source").input_value())
    assert tool_graph["grants"] == ["search.tool"]
    assert tool_graph["max_cost_minor"] == 7
    assert json.loads(page.locator("#canonical-result").inner_text())["requested_tools"] == ["search.tool"]
    page.locator("#canonical-remove-tool").click()
    removed_tool_graph = json.loads(page.locator("#canonical-source").input_value())
    assert removed_tool_graph["grants"] == []
    assert removed_tool_graph["max_cost_minor"] == 0
    assert json.loads(page.locator("#canonical-result").inner_text())["ordered_nodes"] == ["input", "output"]
    page.locator("#canonical-new").click()
    page.locator("#canonical-node-id").fill("input")
    page.locator("#canonical-new-node-id").fill("source")
    page.locator("#canonical-rename").click()
    renamed_graph = json.loads(page.locator("#canonical-source").input_value())
    assert renamed_graph["edges"][0]["from_node"] == "source"
    assert renamed_graph["revision"] == 2
    evidence = ROOT.parents[1] / "dist" / "kaios-chromium-e2e.png"
    evidence.parent.mkdir(parents=True, exist_ok=True)
    page.screenshot(path=str(evidence), full_page=True)
    assert evidence.stat().st_size > 1024
    print("PASS KaiOS Chromium DOM E2E: NoCode, Code, delete, persistence shim; screenshot:", evidence)
    browser.close()
