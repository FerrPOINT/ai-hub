"""Regression tests for reviewed contract boundaries, including neighboring identities."""

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from check_semantics import validate


class ReviewSemanticsTests(unittest.TestCase):
    def mutate(self, change):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in [
                "docs/contracts/openapi.v1.json",
                "docs/contracts/data-dictionary.v1.json",
                "docs/examples/consumer-flow.json",
            ]:
                target = root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / relative, target)
            path = root / "docs/contracts/openapi.v1.json"
            spec = json.loads(path.read_text(encoding="utf-8"))
            change(spec)
            path.write_text(json.dumps(spec), encoding="utf-8")
            return validate(root)

    def test_current_contract(self):
        self.assertEqual(validate(), [])

    def test_budget_scope_cannot_drop_identity(self):
        self.assertTrue(
            self.mutate(
                lambda s: s["components"]["schemas"]["BudgetInput"]["required"].remove(
                    "namespace"
                )
            )
        )

    def test_paginated_namespace_cursor_is_usable(self):
        self.assertTrue(
            self.mutate(
                lambda s: s["paths"]["/api/v1/namespaces"]["get"].update(parameters=[])
            )
        )

    def test_service_needs_key_and_signature(self):
        self.assertTrue(
            self.mutate(
                lambda s: s["paths"]["/internal/v1/service-inference"]["post"].update(
                    security=[{"HubClientKey": []}]
                )
            )
        )

    def test_base_context_stays_separate(self):
        self.assertTrue(
            self.mutate(
                lambda s: s["components"]["schemas"]["ExecutionContextV2"][
                    "properties"
                ].update(workspace={"type": "string"})
            )
        )

    def test_filter_echo_distinguishes_unbound(self):
        self.assertTrue(
            self.mutate(
                lambda s: s["components"]["schemas"]["StatisticsFilters"][
                    "required"
                ].remove("binding")
            )
        )

    def test_manual_scoring_has_a_write(self):
        self.assertTrue(
            self.mutate(
                lambda s: s["paths"].pop("/api/v1/evaluations/{evaluation_id}/scores")
            )
        )

    def test_source_policy_cannot_lose_currency(self):
        self.assertTrue(
            self.mutate(
                lambda s: s["components"]["schemas"]["PricingSourceInput"][
                    "required"
                ].remove("currency")
            )
        )


if __name__ == "__main__":
    unittest.main()
