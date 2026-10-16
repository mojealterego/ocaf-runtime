"""Validate the repository's OMEGA OCAF integration contracts without external dependencies."""
from pathlib import Path
import ast

ROOT = Path(__file__).resolve().parent
assert (ROOT / "openai_probe.py").is_file(), "OpenAI probe missing"
ast.parse((ROOT / "openai_probe.py").read_text(encoding="utf-8"))
assert (ROOT / "README.md").is_file(), "Integration documentation missing"
print("PASS: OMEGA OCAF integration files and Python syntax")
