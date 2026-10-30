#!/usr/bin/env python3
"""0.56: static Python AST and Rust CST security review of candidate source.

This identifies selected dangerous call targets and language features; it is
NOT a proof of safety or complete interprocedural/taint analysis.
"""
from __future__ import annotations
import ast
from omnidev_tree_sitter import parse_rust, _parser, _walk

PYTHON_IMPORT_DENY = {"socket", "ctypes", "cffi"}
PYTHON_CALL_DENY = {
    "eval", "exec", "__import__", "getattr", "setattr",
    "os.system", "os.popen", "os.execv", "os.execve",
    "subprocess.run", "subprocess.Popen", "subprocess.call",
    "subprocess.check_output", "subprocess.check_call",
    "importlib.import_module", "pickle.loads", "marshal.loads",
}


def python_security_review(source: str) -> list[dict]:
    tree = ast.parse(source)
    aliases: dict[str, str] = {}
    findings: list[dict] = []

    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                aliases[alias.asname or alias.name] = alias.name
                if alias.name.split(".")[0] in PYTHON_IMPORT_DENY:
                    findings.append({"line": node.lineno, "rule": "UNTRUSTED_IMPORT"})
        if isinstance(node, ast.ImportFrom):
            module = node.module or ""
            for alias in node.names:
                aliases[alias.asname or alias.name] = module + "." + alias.name
            if module.split(".")[0] in PYTHON_IMPORT_DENY:
                findings.append({"line": node.lineno, "rule": "UNTRUSTED_IMPORT"})

    def qualified(node: ast.expr) -> str:
        if isinstance(node, ast.Name):
            return aliases.get(node.id, node.id)
        if isinstance(node, ast.Attribute):
            return qualified(node.value) + "." + node.attr
        return "<dynamic>"

    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            target = qualified(node.func)
            if target == "<dynamic>" or target in PYTHON_CALL_DENY:
                findings.append({"line": node.lineno, "rule": "DANGEROUS_CALL",
                                 "target": target})
        if isinstance(node, ast.ImportFrom) and any(a.name == "*" for a in node.names):
            findings.append({"line": node.lineno, "rule": "WILDCARD_IMPORT"})
    return sorted(findings, key=lambda x: (x["line"], x["rule"]))


def rust_security_review(source: bytes) -> list[dict]:
    parse_rust(source)  # strict syntax gate, including ERROR nodes
    tree = _parser().parse(source)
    findings: list[dict] = []
    for node in _walk(tree.root_node):
        if node.type in {"unsafe_block", "unsafe_function", "unsafe_trait",
                         "extern_block", "foreign_mod_item", "asm_macro"}:
            findings.append({"line": node.start_point.row + 1,
                             "rule": "UNSAFE_OR_FFI_RUST", "target": node.type})
        if node.type == "macro_invocation":
            target = source[node.start_byte:node.end_byte].decode("utf-8")
            if target.lstrip().startswith(("include!", "asm!", "global_asm!")):
                findings.append({"line": node.start_point.row + 1,
                                 "rule": "DYNAMIC_OR_NATIVE_CODE_RUST"})
        if node.type == "call_expression":
            callee = node.child_by_field_name("function")
            if callee is not None:
                name = source[callee.start_byte:callee.end_byte].decode("utf-8")
                if (name.endswith(("Command::new", "Command::arg",
                                   "std::fs::write", "std::fs::remove_file"))
                        or name in {"libc::system", "std::process::exit"}):
                    findings.append({"line": node.start_point.row + 1,
                                     "rule": "SENSITIVE_RUST_EFFECT", "target": name})
    return sorted(findings, key=lambda x: (x["line"], x["rule"]))


def review_source(path: str, data: bytes) -> dict:
    if path.endswith(".py"):
        findings = python_security_review(data.decode("utf-8"))
    elif path.endswith(".rs"):
        findings = rust_security_review(data)
    else:
        raise ValueError("unrecognized language")
    return {"status": "DENIED_FOR_AUTOMATION" if findings else "OPERATOR_REVIEW_REQUIRED",
            "scope": "STATIC_PYTHON_AST_RUST_CST_SELECTED_EFFECTS",
            "findings": findings}
