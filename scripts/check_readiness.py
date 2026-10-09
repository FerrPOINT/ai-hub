"""Check implementation prerequisites that structural and rendered gates cannot prove."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def validate(root=ROOT):
    spec = json.loads(
        (root / "docs/contracts/openapi.v1.json").read_text(encoding="utf-8")
    )
    data = json.loads(
        (root / "docs/contracts/data-dictionary.v1.json").read_text(encoding="utf-8")
    )
    tables = {t["name"]: t for t in data["tables"]}
    schemas = spec["components"]["schemas"]
    errors = []

    def require(condition, message):
        if not condition:
            errors.append(message)

    for relation in data["relationships"]:
        owner, target = tables[relation["table"]], tables[relation["target_table"]]
        require(
            owner["phase"] >= target["phase"],
            "Future-stage FK: " + owner["name"] + " -> " + target["name"],
        )
    for name in (
        "requests",
        "attempts",
        "usage_facts",
        "price_revisions",
        "ledger_entries",
        "budget_policies",
        "budget_periods",
        "reservations",
    ):
        require(
            tables[name]["phase"] <= tables["probe_snapshots"]["phase"],
            "Probe prerequisite appears too late: " + name,
        )

    filters = schemas.get("StatisticsFilters", {})
    require(
        schemas["Statistics"]["properties"]["filter_echo"].get("$ref")
        == "#/components/schemas/StatisticsFilters",
        "Statistics echo is not canonical",
    )
    for path in (
        "/api/v1/statistics/summary",
        "/api/v1/statistics/breakdown",
        "/api/v1/exports",
    ):
        for parameter in spec["paths"][path]["get"].get("parameters", []):
            if parameter["name"] in {"format", "limit", "cursor"}:
                continue
            field = parameter.get("x-filter-field")
            if field and field.startswith("namespace."):
                expected = (
                    "#/components/schemas/NamespaceRef/properties/"
                    + field.split(".")[1]
                )
            else:
                expected = "#/components/schemas/StatisticsFilters/properties/" + str(
                    field
                )
            require(
                field is not None and parameter["schema"].get("$ref") == expected,
                "Unmapped query filter: " + path + "/" + parameter["name"],
            )
    require(
        {
            "actual_model",
            "evaluation_run_id",
            "provider_id",
            "project_binding",
            "timezone",
        }
        <= set(filters.get("required", [])),
        "Canonical filters omit a supported dimension",
    )

    charge = schemas["ProjectCharge"]
    require(
        "cost_source_state" in charge["required"] and bool(charge.get("allOf")),
        "Unknown cost source has no guarded representation",
    )
    require(
        any(
            x.get("type") == "null"
            for x in charge["properties"]["cost_source_revision_id"].get("oneOf", [])
        ),
        "Unconfigured source requires a fabricated ID",
    )

    cancel = (
        spec["paths"]
        .get("/api/v1/project-tariff-activations/{activation_id}/cancel", {})
        .get("post", {})
    )
    require(
        cancel.get("security") == [{"CentralAuth": []}],
        "Tariff cancellation lacks control authorization",
    )
    require(
        {"activation_id", "Idempotency-Key"}
        <= {x["name"] for x in cancel.get("parameters", [])},
        "Tariff cancellation lacks identity/replay boundary",
    )
    require(
        "project_tariff_cancellations" in tables, "Tariff cancellation is not durable"
    )
    require(
        {"expected_policy_version", "reason"}
        <= set(schemas.get("TariffCancellationInput", {}).get("required", [])),
        "Tariff cancellation lacks CAS/audit reason",
    )

    branches = schemas["ServiceInferenceInput"].get("oneOf", [])
    require(
        {
            (
                b.get("properties", {}).get("protocol", {}).get("const"),
                b.get("properties", {}).get("invocation", {}).get("$ref"),
            )
            for b in branches
        }
        == {
            ("chat", "#/components/schemas/ChatInput"),
            ("responses", "#/components/schemas/ResponsesInput"),
        },
        "Service protocol does not discriminate invocation",
    )
    return errors


if __name__ == "__main__":
    findings = validate()
    if findings:
        raise SystemExit("\n".join(findings))
    print("AI Hub implementation-readiness contract gate: PASS; application NOT RUN")
