#!/usr/bin/env python3
"""Validate the development/design handoff against its real artifacts."""

import hashlib
import json
import re
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = [
    "docs/DESIGN_SYSTEM.md",
    "docs/UI_SCREEN_SPEC.md",
    "docs/UI_FIELD_REFERENCE.md",
    "docs/USE_CASES.md",
    "docs/ACCESS_MATRIX.md",
    "docs/ERROR_CATALOG.md",
    "docs/STATE_MACHINES.md",
    "docs/RECONCILIATION.md",
    "docs/DATA_DICTIONARY.md",
    "docs/MIGRATION_PLAN.md",
    "docs/IMPLEMENTATION_PACKETS.md",
    "docs/DESIGN_ACCEPTANCE.md",
    "docs/DEVELOP_READY.md",
    "docs/design/README.md",
    "docs/design/prototype.html",
    "docs/design/QA.md",
    "docs/design/gallery.html",
    "docs/design/design-contract.json",
    "docs/design/evidence.json",
    "docs/contracts/data-dictionary.v1.json",
]


class Document(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = []
        self.external = []

    def handle_starttag(self, tag, attrs):
        values = dict(attrs)
        if "id" in values:
            self.ids.append(values["id"])
        if tag in {"script", "link", "img", "iframe"}:
            for key in ("src", "href"):
                value = values.get(key, "")
                if value.startswith(("http://", "https://", "//")):
                    self.external.append(value)


def validate(root=ROOT, require_evidence=True):
    errors = []
    for name in REQUIRED:
        if name.endswith("evidence.json") and not require_evidence:
            continue
        if not (root / name).is_file():
            errors.append("Missing design handoff: " + name)
    if errors:
        return errors
    spec = json.loads((root / "docs/contracts/openapi.v1.json").read_text())
    design = json.loads((root / "docs/design/design-contract.json").read_text())
    routes = json.loads((root / "docs/ui-routes.json").read_text())["routes"]
    operations = {
        op["operationId"]
        for methods in spec["paths"].values()
        for op in methods.values()
    }
    if {p["route"] for p in design["pages"]} != {r["path"] for r in routes}:
        errors.append("Design does not cover every operational route")
    for page in design["pages"]:
        if not set(page["operations"]) <= operations:
            errors.append("Unknown design operation: " + page["route"])
        if not page["sections"] or not page["purpose"]:
            errors.append("Design page has no content hierarchy: " + page["route"])
    for name, form in design["forms"].items():
        schema = spec["components"]["schemas"][form["schema"]]
        if form["operation"] not in operations or not set(form["fields"]) <= set(
            schema["properties"]
        ):
            errors.append("Form/API field mismatch: " + name)
    data = json.loads((root / "docs/contracts/data-dictionary.v1.json").read_text())
    tables = {t["name"]: {c["name"] for c in t["columns"]} for t in data["tables"]}
    if len(tables) != len(data["tables"]):
        errors.append("Duplicate data table")
    for relation in data["relationships"]:
        if relation["column"] not in tables.get(relation["table"], set()) or relation[
            "target_column"
        ] not in tables.get(relation["target_table"], set()):
            errors.append("Invalid data relationship: " + repr(relation))
    for table in data["tables"]:
        if len({c["name"] for c in table["columns"]}) != len(table["columns"]):
            errors.append("Duplicate column: " + table["name"])
        if not all(c["type"] and c["rule"] for c in table["columns"]):
            errors.append("Untyped field: " + table["name"])
    prototype = root / "docs/design/prototype.html"
    source = prototype.read_text()
    parser = Document()
    parser.feed(source)
    if len(parser.ids) != len(set(parser.ids)):
        errors.append("Duplicate prototype static IDs")
    if parser.external:
        errors.append("Prototype has external dependency: " + repr(parser.external))
    if re.search(r"\bfetch\s*\(|XMLHttpRequest|__BASE_CSS__|[§¤]", source):
        errors.append("Prototype invokes external transport or contains a placeholder")
    scripts = re.findall(r"<script>(.*?)</script>", source, re.DOTALL)
    if len(scripts) != 1:
        errors.append("Prototype script is not self-contained")
    if not require_evidence:
        return errors
    evidence = json.loads((root / "docs/design/evidence.json").read_text())
    html_sha = hashlib.sha256(prototype.read_bytes()).hexdigest()
    script_sha = hashlib.sha256(scripts[0].encode()).hexdigest()
    if evidence["html_sha256"] != html_sha or evidence["script_sha256"] != script_sha:
        errors.append("Design evidence refers to stale source")
    if (
        evidence.get("surface") != "Codex in-app browser"
        or evidence.get("kind") != "prototype"
    ):
        errors.append("Invalid design evidence surface/kind")
    checks = evidence["geometry"]
    for route in [r["path"] for r in routes]:
        for width in (375, 1440, 2560):
            if not any(
                x["route"] == route and x["width"] == width and x["theme"] == "dark"
                for x in checks
            ):
                errors.append(f"Missing route/width evidence: {route}/{width}")
        for theme in ("gray", "light"):
            if not any(x["route"] == route and x["theme"] == theme for x in checks):
                errors.append(f"Missing theme evidence: {route}/{theme}")
    if any(x["scrollWidth"] > x["clientWidth"] or x["bad"] for x in checks):
        errors.append("Prototype geometry failure")
    for shot in evidence["screenshots"]:
        path = root / "docs/design/screenshots" / shot["file"]
        if (
            not path.is_file()
            or hashlib.sha256(path.read_bytes()).hexdigest() != shot["sha256"]
        ):
            errors.append("Missing/changed screenshot: " + shot["file"])
    if not evidence["flows"] or any(x["result"] != "PASS" for x in evidence["flows"]):
        errors.append("Unverified design flows")
    if evidence.get("console_errors"):
        errors.append("Prototype console errors")
    if any(x["ratio"] < 4.5 for x in evidence.get("contrast", [])):
        errors.append("Insufficient normal text token contrast")
    if any(
        x["state"] != x["expectedState"]
        or x["scrollWidth"] > x["clientWidth"]
        or x["bad"]
        for x in evidence.get("states", [])
    ):
        errors.append("State render/geometry mismatch")
    if any(
        x["pageActions"]
        for x in evidence.get("states", [])
        if x["state"] in {"forbidden", "not_found", "loading"}
    ):
        errors.append("Forbidden/missing context shows page CRUD")
    if any(
        x["pendingEditable"]
        for x in evidence.get("states", [])
        if x["state"] == "pending"
    ):
        errors.append("Pending mutation allows duplicate editing")
    return errors


if __name__ == "__main__":
    errors = validate()
    if errors:
        print("\n".join(errors))
        raise SystemExit(1)
    print("AI Hub development/design handoff gate: PASS")
    print("Real application/provider/runtime acceptance: NOT RUN")
