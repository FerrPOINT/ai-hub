#!/usr/bin/env python3
"""Offline structural gate for the AI Hub pre-development baseline."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from decimal import Decimal
from pathlib import Path
from urllib.parse import unquote

REQUIRED = [
    "AGENTS.md",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "THIRD_PARTY_NOTICES.md",
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".base-revision",
    "README.md",
    "docs/TZ.md",
    "docs/DOMAIN_MODEL.md",
    "docs/ARCHITECTURE.md",
    "docs/ROUTING.md",
    "docs/ACCOUNTING.md",
    "docs/ANALYTICS.md",
    "docs/PROVIDERS.md",
    "docs/EVALUATIONS.md",
    "docs/SECURITY.md",
    "docs/THREAT_MODEL.md",
    "docs/PRODUCT_REQUIREMENTS.md",
    "docs/DATA_MODEL.md",
    "docs/BASE_INTEGRATION.md",
    "docs/FRONTEND_ARCHITECTURE.md",
    "docs/UI_UX.md",
    "docs/ENV.md",
    "docs/LOCAL_SETUP.md",
    "docs/MIGRATIONS.md",
    "docs/DEPLOYMENT.md",
    "docs/BACKUP_RESTORE.md",
    "docs/OPERATIONS.md",
    "docs/MONITORING.md",
    "docs/STANDARDS.md",
    "docs/API.md",
    "docs/contracts/INFERENCE_V1.md",
    "docs/contracts/ADMIN_HANDOFF_V1.md",
    "docs/IMPLEMENTATION_PLAN.md",
    "docs/ROADMAP.md",
    "docs/TESTING.md",
    "docs/QUALITY_GATE.md",
    "docs/CI_CD.md",
    "docs/RISK_REGISTER.md",
    "docs/TRACEABILITY.md",
    "docs/CURRENT_STATE.md",
    "docs/PRE_DEVELOPMENT_GATE.md",
    "docs/contracts/openapi.v1.json",
    "docs/ui-routes.json",
    "docs/traceability.json",
    "docs/ADR_INDEX.md",
    "docs/SOURCE_AUDIT.md",
    "docs/source-audit.json",
    "docs/RELEASE.md",
    "docs/examples/accounting-example.json",
    "docs/adr/0001-product-ownership.md",
    "docs/adr/0002-stack-and-base.md",
    "docs/adr/0003-profile-routing.md",
    "docs/adr/0004-financial-ledger.md",
    "docs/adr/0005-access-and-retention.md",
    "docs/adr/0006-adapter-qualification.md",
    "LICENSE",
    "NOTICE",
    "scripts/check_docs.py",
    "scripts/validate_contract.py",
    "scripts/tests/test_check_docs.py",
    ".github/PULL_REQUEST_TEMPLATE.md",
]
LINK = re.compile(r"(?<!!)\[[^\]]+\]\(([^)\s]+)\)")
FENCE = re.compile(r"^\x60{3}.*?^\x60{3}[ \t]*$", re.MULTILINE | re.DOTALL)
HEADING = re.compile(r"^#{1,6}\s+(.+)$", re.MULTILINE)
METHODS = {"get", "post", "put", "patch", "delete"}


def anchors(text):
    result = set(re.findall(r'<a\s+name="([^"]+)"', text))
    for heading in HEADING.findall(text):
        heading = re.sub(r"[^\w\- ]", "", heading.lower()).replace(" ", "-")
        result.add(heading)
    return result


def validate(root: Path):
    root = root.resolve()
    errors = []
    for name in REQUIRED:
        if not (root / name).is_file():
            errors.append(f"Missing required artifact: {name}")
    for path in list(root.glob("*.md")) + list((root / "docs").rglob("*.md")):
        try:
            text = path.read_text(encoding="utf-8-sig")
        except UnicodeError:
            errors.append(f"Not UTF-8: {path.relative_to(root)}")
            continue
        if not text.strip() or not text.startswith("#"):
            errors.append(f"Empty/unheaded document: {path.relative_to(root)}")
        if len(re.findall(r"^\x60{3}", text, re.MULTILINE)) % 2:
            errors.append(f"Unclosed code fence: {path.relative_to(root)}")
        if re.search(r"\b(?:TODO|TBD|FIXME)\b|\{\{[^}]+\}\}", text):
            errors.append(f"Unresolved placeholder: {path.relative_to(root)}")
        for target in LINK.findall(FENCE.sub("", text)):
            if target.startswith(("https://", "http://", "mailto:")):
                continue
            file, _, fragment = unquote(target).partition("#")
            dest = (path.parent / file).resolve() if file else path
            if not dest.is_relative_to(root):
                errors.append(
                    f"Nonportable local link: {path.relative_to(root)} -> {target}"
                )
            elif not dest.exists():
                errors.append(
                    f"Broken local link: {path.relative_to(root)} -> {target}"
                )
            elif (
                fragment
                and dest.suffix == ".md"
                and fragment not in anchors(dest.read_text(encoding="utf-8-sig"))
            ):
                errors.append(f"Broken anchor: {path.relative_to(root)} -> {target}")

    documents = {}
    for path in (root / "docs").rglob("*.json"):
        try:
            documents[str(path.relative_to(root)).replace("\\", "/")] = json.loads(
                path.read_text(encoding="utf-8-sig")
            )
        except (UnicodeError, ValueError) as exc:
            errors.append(f"Invalid JSON {path.relative_to(root)}: {exc}")
    trace = documents.get("docs/traceability.json")
    spec = documents.get("docs/contracts/openapi.v1.json")
    route_doc = documents.get("docs/ui-routes.json")
    if not all([trace, spec, route_doc]):
        return errors + ["Traceability, OpenAPI and UI route manifests are required"]

    requirements = trace["requirements"]
    tests = trace["tests"]
    phases = trace["phases"]

    def ids(rows, field, label):
        values = [row[field] for row in rows]
        if len(values) != len(set(values)):
            errors.append(f"Duplicate {label} IDs")
        return set(values)

    req_ids = ids(requirements, "id", "requirement")
    test_ids = ids(tests, "id", "test")
    phase_ids = ids(phases, "id", "phase")
    if not any(x.startswith("FR-") for x in req_ids) or not any(
        x.startswith("NFR-") for x in req_ids
    ):
        errors.append("Both FR and NFR requirements are required")
    covered = set()
    for item in requirements:
        if not item["tests"] or not set(item["tests"]) <= test_ids:
            errors.append(f"{item['id']}: missing behavioral test")
        if item["phase"] not in phase_ids or not (root / item["document"]).is_file():
            errors.append(f"{item['id']}: missing phase/specification")
        if item.get("status") != "planned" or not item.get("acceptance"):
            errors.append(f"{item['id']}: unsupported readiness or empty criterion")
    for test in tests:
        if not test["requirements"] or not set(test["requirements"]) <= req_ids:
            errors.append(f"{test['id']}: unknown requirement")
        if test.get("status") != "not_run" or not test.get("scenario"):
            errors.append(f"{test['id']}: unsupported test evidence")
        covered.update(test["requirements"])
        for req in test["requirements"]:
            matching = [x for x in requirements if x["id"] == req]
            if matching and test["id"] not in matching[0]["tests"]:
                errors.append(f"{test['id']}: asymmetric test mapping")
    if covered != req_ids:
        errors.append("Uncovered requirement")

    pending = {p["id"]: set(p["depends_on"]) for p in phases}
    if any(not deps <= phase_ids for deps in pending.values()):
        errors.append("Unknown phase dependency")
    done = set()
    while pending:
        ready = {key for key, deps in pending.items() if deps <= done}
        if not ready:
            errors.append("Phase dependency cycle")
            break
        done |= ready
        pending = {key: deps for key, deps in pending.items() if key not in ready}
    for name, expected, pattern in [
        ("docs/PRODUCT_REQUIREMENTS.md", req_ids, r"^\|\s*((?:FR|NFR)-\d{3})\s*\|"),
        ("docs/TRACEABILITY.md", req_ids, r"^\|\s*((?:FR|NFR)-\d{3})\s*\|"),
        ("docs/TESTING.md", test_ids, r"^\|\s*(TC-\d{3})\s*\|"),
    ]:
        file_path = root / name
        actual = (
            re.findall(pattern, file_path.read_text(encoding="utf-8-sig"), re.MULTILINE)
            if file_path.exists()
            else []
        )
        if set(actual) != expected or len(actual) != len(expected):
            errors.append(f"Text/manifest coverage mismatch: {name}")

    if spec.get("openapi") != "3.1.0" or spec.get("x-readiness") != "design-only":
        errors.append("OpenAPI must be explicitly design-only 3.1")

    def walk(value):
        if isinstance(value, dict):
            if "$ref" in value:
                target = value["$ref"]
                if not target.startswith("#/"):
                    errors.append("Non-local OpenAPI $ref")
                else:
                    try:
                        resolved = spec
                        for part in target[2:].split("/"):
                            resolved = resolved[
                                part.replace("~1", "/").replace("~0", "~")
                            ]
                    except (KeyError, TypeError):
                        errors.append(f"Unresolved OpenAPI ref: {target}")
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)

    walk(spec)
    op_ids = set()
    for path, operations in spec["paths"].items():
        for method, op in operations.items():
            if method not in METHODS:
                continue
            key = op["operationId"]
            if key in op_ids:
                errors.append(f"Duplicate operation ID: {key}")
            op_ids.add(key)
            placeholders = set(re.findall(r"\{([^}]+)\}", path))
            path_params = {
                p["name"]
                for p in op.get("parameters", [])
                if p["in"] == "path" and p.get("required")
            }
            if placeholders != path_params:
                errors.append(f"OpenAPI path parameters differ: {key}")
            if path.startswith(("/api/", "/v1/")) and not op.get("security"):
                errors.append(f"Unprotected operation: {key}")
            if not set(op.get("x-requirements", [])) <= req_ids:
                errors.append(f"Unknown operation requirement: {key}")
            if not any(code.startswith("2") for code in op["responses"]):
                errors.append(f"Missing operation success schema: {key}")
            if (
                path in ("/v1/chat/completions", "/v1/responses")
                and "200" not in op["responses"]
            ):
                errors.append("Compatible inference must return HTTP 200")

    routes = route_doc["routes"]
    ids(routes, "path", "route")
    for route in routes:
        if not route["operations"] or not set(route["operations"]) <= op_ids:
            errors.append(f"Route operation missing: {route['path']}")
        if not set(route["tests"]) <= test_ids or route.get("qa") != "not_run":
            errors.append(f"Route evidence invalid: {route['path']}")
        if route["layout"] not in {"wide", "reading", "detail-with-aside"}:
            errors.append(f"Unknown layout: {route['path']}")
        if not {
            "loading",
            "empty",
            "error",
            "permission_denied",
            "partial",
            "stale",
        } <= set(route["states"]):
            errors.append(f"Missing route states: {route['path']}")
    api = (root / "docs/API.md").read_text(encoding="utf-8-sig")
    if not all(re.search(r"\|\s*" + re.escape(key) + r"\s*\|", api) for key in op_ids):
        errors.append("API text misses an operation")
    ui = (root / "docs/UI_UX.md").read_text(encoding="utf-8-sig")
    if not all(
        re.search(r"\|\s*" + re.escape(route["path"]) + r"\s*\|", ui)
        for route in routes
    ):
        errors.append("UI text misses a route")

    example = documents.get("docs/examples/accounting-example.json")
    if example:
        rates, usage, expected = (
            example["rates_per_million"],
            example["usage"],
            example["expected"],
        )
        if not (
            0 <= usage["cached_input_tokens"] <= usage["input_tokens"]
            and 0 <= usage["reasoning_tokens"] <= usage["output_tokens"]
            and all(Decimal(rate) >= 0 for rate in rates.values())
        ):
            errors.append("Accounting category bounds are invalid")
        main = (
            Decimal(usage["input_tokens"] - usage["cached_input_tokens"])
            * Decimal(rates["input_uncached"])
            + Decimal(usage["cached_input_tokens"]) * Decimal(rates["input_cached"])
            + Decimal(usage["output_tokens"]) * Decimal(rates["output_billable"])
        ) / Decimal(1000000)
        total = main + Decimal(example["failed_attempt_uncached_tokens"]) * Decimal(
            rates["input_uncached"]
        ) / Decimal(1000000)
        if main != Decimal(expected["main_attempt_estimated"]) or total != Decimal(
            expected["request_estimated"]
        ):
            errors.append("Accounting decimal example is inconsistent")
        if (
            example["unknown"]["amount"] is not None
            or example["unknown"]["confidence"] != "unknown"
        ):
            errors.append("Unknown charge cannot be zero")
        if Decimal(example["subscription"]["expected_cash_total"]) != Decimal(
            example["subscription"]["period_charge"]
        ):
            errors.append("Subscription must not multiply by calls")
        if Decimal(example["late_receipt"]["expected_effective_charge"]) != Decimal(
            example["late_receipt"]["confirmed_charge"]
        ):
            errors.append("Confirmed settlement must replace estimate")
        precision = example["precision_case"]
        exact = (
            Decimal(precision["tokens"])
            * Decimal(precision["rate_per_million"])
            / Decimal(1000000)
        )
        if exact != Decimal(precision["expected_amount"]) or exact == 0:
            errors.append("Per-request sub-micro cost must remain exact")
    audit = documents.get("docs/source-audit.json")
    if audit:
        for filename in ("LICENSE", "NOTICE"):
            source = next(x for x in audit["sources"] if x["path"] == filename)
            digest = hashlib.sha256((root / filename).read_bytes()).hexdigest()
            if digest != source.get("canonical_sha256", source["sha256"]):
                errors.append(f"Canonical {filename} differs")
    else:
        errors.append("Source audit missing")
    pin = (root / ".base-revision").read_text().strip()
    if not re.fullmatch(r"[0-9a-f]{40}", pin):
        errors.append("Base revision must be full SHA")
    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[1]
    )
    args = parser.parse_args()
    findings = validate(args.root)
    if findings:
        print("\n".join(findings))
        return 1
    print("AI Hub documentation structural gate: PASS")
    print("Application/DB/provider/runtime/browser acceptance: NOT RUN")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
