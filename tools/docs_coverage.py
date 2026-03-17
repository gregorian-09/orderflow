#!/usr/bin/env python3
"""Documentation coverage checks for Rust, Python, Java, and C public APIs."""

from __future__ import annotations

import argparse
import ast
import os
import pathlib
import re
import subprocess
import sys
import tomllib
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from typing import Iterable


ROOT = pathlib.Path(__file__).resolve().parents[1]
PYTHON_BINDING_ROOT = ROOT / "bindings" / "python" / "orderflow"
JAVA_BINDING_ROOT = ROOT / "bindings" / "java" / "src" / "main" / "java" / "com" / "orderflow" / "bindings"
C_HEADER_PATH = ROOT / "crates" / "of_ffi_c" / "include" / "orderflow.h"
PYPROJECT_PATH = ROOT / "bindings" / "python" / "pyproject.toml"
JAVA_POM_PATH = ROOT / "bindings" / "java" / "pom.xml"


@dataclass
class CoverageResult:
    target: str
    documented: int
    total: int
    details: list[str]

    @property
    def pct(self) -> float:
        if self.total == 0:
            return 100.0
        return (self.documented / self.total) * 100.0

    @property
    def ok(self) -> bool:
        return self.documented == self.total


@dataclass
class GateResult:
    target: str
    ok: bool
    message: str


def _has_block_doc(lines: list[str], line_index: int) -> bool:
    idx = line_index - 1
    while idx >= 0:
        stripped = lines[idx].strip()
        if stripped == "":
            idx -= 1
            continue
        if stripped.startswith("@"):
            idx -= 1
            continue
        break

    if idx < 0 or not lines[idx].strip().endswith("*/"):
        return False

    while idx >= 0 and "/*" not in lines[idx]:
        idx -= 1
    if idx < 0:
        return False
    return lines[idx].strip().startswith("/**")


def check_rust_missing_docs() -> GateResult:
    rust_packages = [
        "of_core",
        "of_signals",
        "of_persist",
        "of_adapters",
        "of_runtime",
        "of_ffi_c",
    ]
    cargo_cmd = ["cargo", "check", "--all-features", "--quiet"]
    for pkg in rust_packages:
        cargo_cmd.extend(["-p", pkg])

    env = os.environ.copy()
    existing_rustflags = env.get("RUSTFLAGS", "").strip()
    env["RUSTFLAGS"] = (
        "-Dmissing-docs" if not existing_rustflags else f"{existing_rustflags} -Dmissing-docs"
    )
    proc = subprocess.run(
        cargo_cmd,
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
    )
    if proc.returncode == 0:
        return GateResult(
            target="Rust public API docs",
            ok=True,
            message="100.00% (enforced via rustc missing_docs lint)",
        )

    combined = f"{proc.stdout}\n{proc.stderr}"
    missing_hits = re.findall(r"error: missing documentation for [^\n]+", combined)
    preview = "\n".join(missing_hits[:8])
    return GateResult(
        target="Rust public API docs",
        ok=False,
        message=(
            f"missing_docs lint failed with {len(missing_hits)} missing item(s).\n"
            f"{preview if preview else combined[:600]}"
        ),
    )


def check_python_docs() -> CoverageResult:
    total = 0
    documented = 0
    missing: list[str] = []

    for path in sorted(PYTHON_BINDING_ROOT.glob("*.py")):
        module = ast.parse(path.read_text(encoding="utf-8"))
        total += 1
        if ast.get_docstring(module):
            documented += 1
        else:
            missing.append(f"{path.relative_to(ROOT)}:<module>")

        for node in module.body:
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                if node.name.startswith("_"):
                    continue
                total += 1
                if ast.get_docstring(node):
                    documented += 1
                else:
                    missing.append(f"{path.relative_to(ROOT)}:{node.name}")

            if isinstance(node, ast.ClassDef):
                if node.name.startswith("_"):
                    continue
                total += 1
                if ast.get_docstring(node):
                    documented += 1
                else:
                    missing.append(f"{path.relative_to(ROOT)}:{node.name}")

                for member in node.body:
                    if isinstance(member, (ast.FunctionDef, ast.AsyncFunctionDef)):
                        if member.name.startswith("_"):
                            continue
                        total += 1
                        if ast.get_docstring(member):
                            documented += 1
                        else:
                            missing.append(
                                f"{path.relative_to(ROOT)}:{node.name}.{member.name}"
                            )

    return CoverageResult(
        target="Python binding public API docstrings",
        documented=documented,
        total=total,
        details=missing,
    )


JAVA_PUBLIC_DECL_RE = re.compile(
    r"^\s*public\s+"
    r"(?:static\s+|final\s+|abstract\s+|default\s+|synchronized\s+|native\s+|strictfp\s+)*"
    r"("
    r"(?:class|interface|enum|record)\s+\w+"
    r"|"
    r"(?:[\w<>\[\], ?]+)\s+\w+\s*(?:\(|;)"
    r")"
)


def _is_java_api_line(line: str) -> bool:
    stripped = line.strip()
    if stripped.startswith("package ") or stripped.startswith("import "):
        return False
    if stripped.startswith("public static void main("):
        return False
    return bool(JAVA_PUBLIC_DECL_RE.match(line))


def check_java_docs() -> CoverageResult:
    total = 0
    documented = 0
    missing: list[str] = []

    for path in sorted(JAVA_BINDING_ROOT.rglob("*.java")):
        lines = path.read_text(encoding="utf-8").splitlines()
        for idx, line in enumerate(lines):
            if not _is_java_api_line(line):
                continue

            total += 1
            if _has_block_doc(lines, idx):
                documented += 1
            else:
                missing.append(f"{path.relative_to(ROOT)}:{idx + 1}:{line.strip()}")

    return CoverageResult(
        target="Java binding public API JavaDoc",
        documented=documented,
        total=total,
        details=missing,
    )


def _c_api_decl(line: str) -> bool:
    stripped = line.strip()
    if stripped.startswith("typedef struct of_engine") or stripped.startswith(
        "typedef struct of_subscription"
    ):
        return True
    if re.match(r"^typedef\s+(struct|enum)\b", stripped):
        return True
    if re.match(r"^(uint32_t|const char\*|int32_t|void)\s+of_[a-z0-9_]+\s*\(", stripped):
        return True
    return False


def check_c_docs() -> CoverageResult:
    lines = C_HEADER_PATH.read_text(encoding="utf-8").splitlines()
    total = 0
    documented = 0
    missing: list[str] = []

    for idx, line in enumerate(lines):
        if not _c_api_decl(line):
            continue
        total += 1
        if _has_block_doc(lines, idx):
            documented += 1
        else:
            missing.append(f"{C_HEADER_PATH.relative_to(ROOT)}:{idx + 1}:{line.strip()}")

    return CoverageResult(
        target="C binding public API Doxygen docs",
        documented=documented,
        total=total,
        details=missing,
    )


def check_pypi_metadata() -> GateResult:
    with PYPROJECT_PATH.open("rb") as fp:
        data = tomllib.load(fp)

    project = data.get("project", {})
    urls = project.get("urls", {})

    required_fields: list[tuple[str, bool]] = [
        ("project.readme", bool(project.get("readme"))),
        ("project.description", bool(project.get("description"))),
        ("project.urls.Documentation", bool(urls.get("Documentation"))),
        ("project.urls.Repository", bool(urls.get("Repository"))),
        ("project.urls.Changelog", bool(urls.get("Changelog"))),
    ]

    failed = [name for name, ok in required_fields if not ok]
    if failed:
        return GateResult(
            target="PyPI metadata/docs fields",
            ok=False,
            message=f"Missing required metadata fields: {', '.join(failed)}",
        )
    return GateResult(
        target="PyPI metadata/docs fields",
        ok=True,
        message="All required metadata fields are present",
    )


def _pom_text(root: ET.Element, tag: str) -> str | None:
    ns = {"m": "http://maven.apache.org/POM/4.0.0"}
    node = root.find(f"m:{tag}", ns)
    if node is None or node.text is None:
        return None
    return node.text.strip()


def check_maven_metadata() -> GateResult:
    tree = ET.parse(JAVA_POM_PATH)
    root = tree.getroot()
    ns = {"m": "http://maven.apache.org/POM/4.0.0"}

    missing: list[str] = []
    for required_tag in ("name", "description", "url"):
        if not _pom_text(root, required_tag):
            missing.append(required_tag)

    if root.find("m:licenses/m:license", ns) is None:
        missing.append("licenses/license")
    if root.find("m:developers/m:developer", ns) is None:
        missing.append("developers/developer")
    if root.find("m:scm", ns) is None:
        missing.append("scm")

    plugin_artifacts = {
        node.text.strip()
        for node in root.findall(".//m:build/m:plugins/m:plugin/m:artifactId", ns)
        if node.text
    }
    for required_plugin in ("maven-source-plugin", "maven-javadoc-plugin"):
        if required_plugin not in plugin_artifacts:
            missing.append(f"build/plugins/{required_plugin}")

    if missing:
        return GateResult(
            target="Maven Central metadata/docs requirements",
            ok=False,
            message=f"Missing required POM metadata: {', '.join(missing)}",
        )
    return GateResult(
        target="Maven Central metadata/docs requirements",
        ok=True,
        message="Required metadata + source/javadoc plugins are present",
    )


def format_coverage_table(results: Iterable[CoverageResult]) -> str:
    rows = [("Target", "Documented", "Total", "Coverage")]
    for res in results:
        rows.append((res.target, str(res.documented), str(res.total), f"{res.pct:.2f}%"))

    widths = [max(len(row[i]) for row in rows) for i in range(4)]
    out: list[str] = []
    out.append(
        " | ".join(value.ljust(widths[idx]) for idx, value in enumerate(rows[0]))
    )
    out.append("-|-".join("-" * w for w in widths))
    for row in rows[1:]:
        out.append(" | ".join(value.ljust(widths[idx]) for idx, value in enumerate(row)))
    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--enforce",
        action="store_true",
        help="Return non-zero if any target is below 100%% or any gate fails.",
    )
    args = parser.parse_args()

    rust_gate = check_rust_missing_docs()
    python_cov = check_python_docs()
    java_cov = check_java_docs()
    c_cov = check_c_docs()
    pypi_gate = check_pypi_metadata()
    maven_gate = check_maven_metadata()

    print("Documentation Coverage Report\n")
    print(format_coverage_table([python_cov, java_cov, c_cov]))
    print()
    print(
        f"Rust public API docs: {'PASS' if rust_gate.ok else 'FAIL'} - {rust_gate.message}"
    )
    print(f"PyPI docs metadata: {'PASS' if pypi_gate.ok else 'FAIL'} - {pypi_gate.message}")
    print(
        f"Maven docs metadata: {'PASS' if maven_gate.ok else 'FAIL'} - {maven_gate.message}"
    )

    failed = []
    if not rust_gate.ok:
        failed.append(rust_gate.target)
    for cov in (python_cov, java_cov, c_cov):
        if not cov.ok:
            failed.append(cov.target)
            print()
            print(f"Missing docs in {cov.target}:")
            for item in cov.details[:50]:
                print(f"  - {item}")
            if len(cov.details) > 50:
                print(f"  ... and {len(cov.details) - 50} more")
    for gate in (pypi_gate, maven_gate):
        if not gate.ok:
            failed.append(gate.target)

    if args.enforce and failed:
        print()
        print("FAILED targets:")
        for target in failed:
            print(f"- {target}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
