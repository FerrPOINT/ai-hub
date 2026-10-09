"""Validate documentation-level owner mapping and exact legacy conversions."""

import json
import re
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def microdollars_to_decimal(value):
    if not isinstance(value, str) or not re.fullmatch(r"0|[1-9][0-9]{0,19}", value):
        raise ValueError("uint64 decimal string required")
    number = int(value)
    if number > 2**64 - 1:
        raise ValueError("uint64 overflow")
    return format(Decimal(number) / Decimal(1000000), "f")


def namespace_identity(value):
    if value is None:
        return None
    from uuid import UUID

    if not isinstance(value, dict) or set(value) != {
        "registry_instance_id",
        "namespace_id",
    }:
        raise ValueError("exact NamespaceRef required")
    pair = tuple(UUID(value[key]) for key in ("registry_instance_id", "namespace_id"))
    if any(item.int == 0 for item in pair):
        raise ValueError("nonnil UUID required")
    return pair


def validate(root=ROOT):
    errors = []
    spec = json.loads((root / "docs/contracts/openapi.v1.json").read_text())
    mapping = json.loads(
        (root / "docs/contracts/admin-extraction-map.json").read_text()
    )
    rows = mapping["records"]
    expected = (
        {f"UI-{i:02}" for i in range(1, 9)}
        | {f"DB-{i:02}" for i in range(1, 5)}
        | {f"RT-{i:02}" for i in range(1, 5)}
        | {f"KEEP-{i:02}" for i in range(1, 5)}
    )
    if {row["id"] for row in rows} != expected or len(rows) != len(expected):
        errors.append("Incomplete Admin extraction mapping")
    operations = {
        op["operationId"]
        for methods in spec["paths"].values()
        for op in methods.values()
    }
    for row in rows:
        if not all(
            row.get(key)
            for key in (
                "source_fields",
                "target_surface",
                "target_store",
                "preservation",
            )
        ):
            errors.append("Empty extraction row: " + row["id"])
        if (
            row["target_operations"] != "retained"
            and not set(row["target_operations"].split("/")) <= operations
        ):
            errors.append("Unknown extraction target operation: " + row["id"])
    rules = mapping["rules"]
    if (
        rules["workspace_is_namespace"]
        or rules["old_proof_active"]
        or rules["aggregate_creates_receipts"]
        or rules["credential_default"] != "reauthorize"
    ):
        errors.append("Unsafe extraction policy")
    fields = spec["components"]["schemas"]["NamespaceRef"]["properties"]
    if set(fields) != {"registry_instance_id", "namespace_id"} or any(
        v.get("format") != "uuid" for v in fields.values()
    ):
        errors.append("NamespaceRef differs from Base identity")
    for path, parameters in {
        "/api/v1/providers": {"q", "status"},
        "/api/v1/virtual-models": {"q", "mode", "status"},
        "/api/v1/audit": {"q", "action", "registry_instance_id", "namespace_id"},
    }.items():
        if not parameters <= {
            p["name"] for p in spec["paths"][path]["get"]["parameters"]
        }:
            errors.append("Missing declared UI filters: " + path)
    if (
        not {
            "listNamespaceBindings",
            "listNotifications",
            "ackNotification",
            "listModelContextPreferences",
            "saveModelContextPreference",
        }
        <= operations
    ):
        errors.append("Missing alignment operation")
    example = json.loads((root / "docs/examples/consumer-flow.json").read_text())
    for case in example["migration_money_cases"]:
        if Decimal(microdollars_to_decimal(case["microdollars"])) != Decimal(
            case["decimal"]
        ):
            errors.append("Rounded legacy money")
    namespace_identity(example["namespace"])
    routes = json.loads((root / "docs/ui-routes.json").read_text())["routes"]
    for route in routes:
        if (
            route.get("route_class") not in {"operational", "auth"}
            or not route.get("roles")
            or not route.get("fixture")
        ):
            errors.append("Missing route metadata: " + route["path"])
    if not any(r["path"] == "/login" and r["route_class"] == "auth" for r in routes):
        errors.append("Missing auth route")
    return errors


if __name__ == "__main__":
    errors = validate()
    if errors:
        print("\n".join(errors))
        raise SystemExit(1)
    print("AI Hub alignment contract: PASS; actual migration/runtime NOT RUN")
