"""Exact conversions and cross-owner identity must survive neighboring values."""

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from check_alignment import microdollars_to_decimal, namespace_identity, validate


class AlignmentTests(unittest.TestCase):
    def test_exact_microdollars(self):
        self.assertEqual(microdollars_to_decimal("1"), "0.000001")
        self.assertEqual(
            microdollars_to_decimal("18446744073709551615"), "18446744073709.551615"
        )
        self.assertEqual(microdollars_to_decimal("0"), "0")
        for value in ("18446744073709551616", "-1", "1.0", 1, "01"):
            with self.assertRaises(ValueError):
                microdollars_to_decimal(value)

    def test_registry_is_part_of_identity(self):
        original = {
            "registry_instance_id": "10000000-0000-4000-8000-000000000001",
            "namespace_id": "20000000-0000-4000-8000-000000000001",
        }
        neighbor = dict(
            original, registry_instance_id="10000000-0000-4000-8000-000000000002"
        )
        self.assertNotEqual(namespace_identity(original), namespace_identity(neighbor))
        self.assertIsNone(namespace_identity(None))
        for value in (
            {"namespace_id": original["namespace_id"]},
            dict(original, label="same name"),
            dict(original, namespace_id="00000000-0000-0000-0000-000000000000"),
        ):
            with self.assertRaises(ValueError):
                namespace_identity(value)

    def test_real_baseline(self):
        self.assertEqual(validate(ROOT), [])

    def test_missing_source_mapping_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp) / "repo"
            shutil.copytree(
                ROOT,
                target,
                ignore=shutil.ignore_patterns(".git", ".local", "__pycache__"),
            )
            path = target / "docs/contracts/admin-extraction-map.json"
            data = json.loads(path.read_text())
            data["records"] = [r for r in data["records"] if r["id"] != "UI-05"]
            path.write_text(json.dumps(data), encoding="utf-8")
            self.assertIn("Incomplete Admin extraction mapping", validate(target))


if __name__ == "__main__":
    unittest.main()
