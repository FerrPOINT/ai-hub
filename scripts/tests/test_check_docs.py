"""Fault injection tests for the documentation gate, not application tests."""

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from check_docs import validate


class DocumentationGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aihub-doc-gate-")
        self.root = Path(self.temp.name) / "repo"
        shutil.copytree(
            ROOT,
            self.root,
            ignore=shutil.ignore_patterns(".git", ".local", "__pycache__"),
        )

    def tearDown(self):
        self.temp.cleanup()

    def mutate(self, relative, change):
        path = self.root / relative
        data = json.loads(path.read_text(encoding="utf-8-sig"))
        change(data)
        path.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")

    def test_current_baseline_passes(self):
        self.assertEqual(validate(self.root), [])

    def test_missing_behavioral_test_fails(self):
        self.mutate(
            "docs/traceability.json",
            lambda d: d["requirements"][0].update(tests=["TC-999"]),
        )
        self.assertTrue(
            any("missing behavioral test" in e for e in validate(self.root))
        )

    def test_stage_dependency_cycle_fails(self):
        self.mutate(
            "docs/traceability.json",
            lambda d: d["phases"][0]["depends_on"].append("S7"),
        )
        self.assertTrue(any("cycle" in e for e in validate(self.root)))

    def test_broken_openapi_ref_fails(self):
        self.mutate(
            "docs/contracts/openapi.v1.json",
            lambda d: d["components"]["schemas"]["Health"].update(
                {"$ref": "#/components/schemas/Missing"}
            ),
        )
        self.assertTrue(any("Unresolved OpenAPI ref" in e for e in validate(self.root)))

    def test_duplicate_required_field_fails(self):
        self.mutate(
            "docs/contracts/openapi.v1.json",
            lambda d: d["components"]["schemas"]["Statistics"]["required"].append(
                "filter_echo"
            ),
        )
        self.assertTrue(any("Duplicate required" in e for e in validate(self.root)))

    def test_missing_route_operation_fails(self):
        self.mutate(
            "docs/ui-routes.json",
            lambda d: d["routes"][0]["operations"].append("missingOperation"),
        )
        self.assertTrue(
            any("Route operation missing" in e for e in validate(self.root))
        )

    def test_broken_link_and_anchor_fail(self):
        path = self.root / "README.md"
        path.write_text(
            path.read_text()
            + "\n[Broken](docs/no-file.md)\n[Anchor](README.md#no-anchor)\n",
            encoding="utf-8",
        )
        errors = validate(self.root)
        self.assertTrue(any("Broken local link" in e for e in errors))
        self.assertTrue(any("Broken anchor" in e for e in errors))

    def test_unknown_cost_coercion_fails(self):
        self.mutate(
            "docs/examples/accounting-example.json",
            lambda d: d["unknown"].update(amount="0"),
        )
        self.assertTrue(
            any("Unknown charge cannot be zero" in e for e in validate(self.root))
        )

    def test_duplicate_subscription_cost_fails(self):
        self.mutate(
            "docs/examples/accounting-example.json",
            lambda d: d["subscription"].update(expected_cash_total="750"),
        )
        self.assertTrue(
            any("Subscription must not multiply" in e for e in validate(self.root))
        )

    def test_bad_decimal_formula_fails(self):
        self.mutate(
            "docs/examples/accounting-example.json",
            lambda d: d["expected"].update(request_estimated="0.001"),
        )
        self.assertTrue(any("decimal example" in e for e in validate(self.root)))

    def test_fake_completed_implementation_fails(self):
        self.mutate(
            "docs/traceability.json",
            lambda d: d["requirements"][0].update(status="passed"),
        )
        self.assertTrue(any("unsupported readiness" in e for e in validate(self.root)))

    def test_invalid_cached_category_fails(self):
        self.mutate(
            "docs/examples/accounting-example.json",
            lambda d: d["usage"].update(cached_input_tokens=9999),
        )
        self.assertTrue(any("category bounds" in e for e in validate(self.root)))


if __name__ == "__main__":
    unittest.main()
