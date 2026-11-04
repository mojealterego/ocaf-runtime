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
    evidence = ROOT.parents[1] / "dist" / "kaios-chromium-e2e.png"
    evidence.parent.mkdir(parents=True, exist_ok=True)
    page.screenshot(path=str(evidence), full_page=True)
    assert evidence.stat().st_size > 1024
    print("PASS KaiOS Chromium DOM E2E: NoCode, Code, delete, persistence shim; screenshot:", evidence)
    browser.close()
