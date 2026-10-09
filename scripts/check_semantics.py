"""Semantic contract obligations; rendered behavior is checked separately through IAB."""

import json
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def validate(root=ROOT):
    errors = []
    spec = json.loads(
        (root / "docs/contracts/openapi.v1.json").read_text(encoding="utf-8")
    )
    schemas = spec["components"]["schemas"]

    def need(ok, message):
        if not ok:
            errors.append(message)

    budget = schemas["BudgetInput"]
    need(
        {"currency", "effective_from", "effective_to"}
        <= set(schemas["PricingSourceInput"]["required"]),
        "Pricing source must retain currency and effective interval",
    )
    need(
        "namespace" in budget["required"]
        and budget["properties"]["scope_id"].get("format") == "uuid",
        "Budget must use typed Namespace and UUID target",
    )
    for path in [
        "/api/v1/namespaces",
        "/api/v1/connections/{connection_id}/model-contexts",
    ]:
        names = {x["name"] for x in spec["paths"][path]["get"]["parameters"]}
        need(
            {"limit", "cursor"} <= names,
            "Bounded list lacks usable pagination: " + path,
        )
    need(
        "label" in schemas["NamespaceBinding"]["required"],
        "Namespace picker lacks display metadata",
    )
    context = spec["paths"]["/api/v1/connections/{connection_id}/model-contexts"]["put"]
    need(
        {"If-Match", "Idempotency-Key"} <= {x["name"] for x in context["parameters"]},
        "Context save lacks CAS/idempotency",
    )
    need('"0"' in context["description"], "Context first-create precondition undefined")
    required = {"schema_version", "operation_id", "namespace", "task", "repositories"}
    need(
        set(schemas["ExecutionContextV2"]["properties"]) == required,
        "Base V2 shape drift",
    )
    service = spec["paths"]["/internal/v1/service-inference"]["post"]
    need(
        service["security"] == [{"HubClientKey": [], "FleetDelegation": []}],
        "Service transport must require both authentication boundaries",
    )
    need(
        "/internal/v1/delegations/revoke" in spec["paths"],
        "Revocation transport missing",
    )
    need(
        "/api/v1/evaluations/{evaluation_id}/scores" in spec["paths"],
        "Manual score cannot be submitted",
    )
    need(
        "/api/v1/project-tariff-activations" in spec["paths"],
        "Tariff save is not explicit activation",
    )
    echo = schemas["Statistics"]["properties"]["filter_echo"]
    need(
        {
            "binding",
            "namespace",
            "from",
            "to",
            "currency",
            "group_by",
            "client_id",
            "connection_id",
            "virtual_model_id",
            "profile_revision_id",
            "user_subject",
            "status",
        }
        <= set(echo["required"]),
        "Incomplete canonical filter echo",
    )
    example = json.loads(
        (root / "docs/examples/consumer-flow.json").read_text(encoding="utf-8")
    )
    try:
        uuid.UUID(example["response_headers"]["X-AIHub-Profile-Revision"])
    except ValueError:
        errors.append("Consumer revision header is not UUID")
    dd = json.loads(
        (root / "docs/contracts/data-dictionary.v1.json").read_text(encoding="utf-8")
    )
    tables = {t["name"] for t in dd["tables"]}
    need(
        {
            "project_tariff_policies",
            "project_tariff_activations",
            "evaluation_manual_scores",
            "service_delegations",
        }
        <= tables,
        "Missing timeline/score/delegation persistence",
    )
    return errors


if __name__ == "__main__":
    findings = validate()
    if findings:
        raise SystemExit("\n".join(findings))
    print("Review semantic contract obligations: PASS; actual runtime NOT RUN")
