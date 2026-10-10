"""Negative handoff tests reject false development-ready evidence."""

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from check_design import validate


class DesignHandoffTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aihub-design-gate-")
        self.root = Path(self.temp.name) / "repo"
        shutil.copytree(
            ROOT,
            self.root,
            ignore=shutil.ignore_patterns(".git", ".local", "__pycache__", "node_modules", "target", "dist", "coverage"),
        )

    def tearDown(self):
        self.temp.cleanup()

    def change(self, relative, edit):
        path = self.root / relative
        data = json.loads(path.read_text())
        edit(data)
        path.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")

    def test_real_baseline(self):
        self.assertEqual(validate(self.root), [])

    def test_stale_source_evidence(self):
        path = self.root / "docs/design/prototype.html"
        path.write_text(path.read_text() + "\n<!-- new source -->\n", encoding="utf-8")
        self.assertTrue(any("stale source" in e for e in validate(self.root)))

    def test_missing_width_proof(self):
        self.change(
            "docs/design/evidence.json",
            lambda d: d.update(
                geometry=[x for x in d["geometry"] if x["width"] != 375]
            ),
        )
        self.assertTrue(any("width evidence" in e for e in validate(self.root)))

    def test_invalid_field_mapping(self):
        self.change(
            "docs/design/design-contract.json",
            lambda d: d["forms"]["price"]["fields"].append("invented_field"),
        )
        self.assertTrue(
            any("Form/API field mismatch" in e for e in validate(self.root))
        )

    def test_invalid_fk_target(self):
        self.change(
            "docs/contracts/data-dictionary.v1.json",
            lambda d: d["relationships"][0].update(target_column="absent"),
        )
        self.assertTrue(
            any("Invalid data relationship" in e for e in validate(self.root))
        )

    def test_wrong_browser_surface(self):
        self.change(
            "docs/design/evidence.json", lambda d: d.update(surface="external browser")
        )
        self.assertTrue(any("surface/kind" in e for e in validate(self.root)))

    def test_stale_individual_flow(self):
        self.change(
            "docs/design/evidence.json",
            lambda d: d["flows"][0].update(sourceScriptHash="old-script"),
        )
        self.assertTrue(any("Stale individual" in e for e in validate(self.root)))

    def test_missing_behavioral_regression(self):
        self.change(
            "docs/design/evidence.json",
            lambda d: d.update(
                flows=[x for x in d["flows"] if x["name"] != "connection-edit-identity"]
            ),
        )
        self.assertTrue(
            any("behavioral UX regression" in e for e in validate(self.root))
        )


if __name__ == "__main__":
    unittest.main()
