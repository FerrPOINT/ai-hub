"""Negative readiness checks exercise missing prerequisites and contract drift."""

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from check_readiness import validate


class ReadinessContractsTests(unittest.TestCase):
    def changed(self, mutate_spec=None, mutate_data=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in (
                "docs/contracts/openapi.v1.json",
                "docs/contracts/data-dictionary.v1.json",
            ):
                target = root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / relative, target)
                contents = json.loads(target.read_text(encoding="utf-8"))
                mutate = mutate_spec if "openapi" in relative else mutate_data
                if mutate:
                    mutate(contents)
                target.write_text(json.dumps(contents), encoding="utf-8")
            return validate(root)

    def test_current_readiness_contract(self):
        self.assertEqual(validate(), [])

    def test_price_dependency_cannot_appear_after_source(self):
        self.assertTrue(
            self.changed(
                mutate_data=lambda d: next(
                    t for t in d["tables"] if t["name"] == "price_revisions"
                ).update(phase="S3")
            )
        )

    def test_probe_cannot_precede_budget(self):
        self.assertTrue(
            self.changed(
                mutate_data=lambda d: next(
                    t for t in d["tables"] if t["name"] == "budget_policies"
                ).update(phase="S3")
            )
        )

    def test_filter_query_cannot_drift_from_echo(self):
        self.assertTrue(
            self.changed(
                mutate_spec=lambda s: next(
                    p
                    for p in s["paths"]["/api/v1/statistics/summary"]["get"][
                        "parameters"
                    ]
                    if p["name"] == "actual_model"
                ).pop("x-filter-field", None)
            )
        )

    def test_cancellation_cannot_lose_authorization(self):
        self.assertTrue(
            self.changed(
                mutate_spec=lambda s: s["paths"][
                    "/api/v1/project-tariff-activations/{activation_id}/cancel"
                ]["post"].update(security=[])
            )
        )

    def test_service_protocol_cannot_be_untyped(self):
        self.assertTrue(
            self.changed(
                mutate_spec=lambda s: s["components"]["schemas"][
                    "ServiceInferenceInput"
                ].pop("oneOf", None)
            )
        )

    def test_unknown_source_cannot_require_fake_id(self):
        self.assertTrue(
            self.changed(
                mutate_spec=lambda s: s["components"]["schemas"]["ProjectCharge"][
                    "properties"
                ].update(cost_source_revision_id={"type": "string", "format": "uuid"})
            )
        )


if __name__ == "__main__":
    unittest.main()
