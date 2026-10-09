#!/usr/bin/env python3
"""Independent validation of the design OpenAPI and contract examples."""

import argparse
import json
from decimal import Decimal
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker
from openapi_spec_validator import validate
from referencing import Registry, Resource
from referencing.jsonschema import DRAFT202012


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[1]
    )
    root = parser.parse_args().root.resolve()
    spec = json.loads((root / "docs/contracts/openapi.v1.json").read_text())
    validate(spec)
    for schema in spec["components"]["schemas"].values():
        Draft202012Validator.check_schema(schema)
    uri = "urn:aihub:openapi"
    # Validate JSON decimal numbers as their wire lexemes, not binary float approximations.
    validation_spec = json.loads(json.dumps(spec), parse_float=Decimal)
    registry = Registry().with_resource(
        uri, Resource.from_contents(validation_spec, default_specification=DRAFT202012)
    )

    def accepts(name, value):
        checker = Draft202012Validator(
            {"$ref": f"{uri}#/components/schemas/{name}"},
            registry=registry,
            format_checker=FormatChecker(),
        )
        return checker.is_valid(json.loads(json.dumps(value), parse_float=Decimal))

    money = {
        "amount": None,
        "currency": "USD",
        "confidence": "unknown",
        "source": "pending",
    }
    assert accepts("Money", money)
    assert not accepts("Money", {**money, "amount": "0"})
    assert accepts("Money", {**money, "amount": "0.011", "confidence": "estimated"})
    assert accepts(
        "Money", {**money, "amount": "0.000000000000000001", "confidence": "estimated"}
    )
    chat = {"model": "main-dev", "messages": [{"role": "user", "content": "synthetic"}]}
    assert accepts("ChatInput", chat)
    assert not accepts("ChatInput", {**chat, "api_base": "https://invalid.example"})
    assert accepts(
        "ChatInput",
        {
            **chat,
            "messages": [
                {"role": "user", "content": "synthetic"},
                {
                    "role": "assistant",
                    "content": None,
                    "tool_calls": [
                        {
                            "id": "call-1",
                            "type": "function",
                            "function": {"name": "check", "arguments": "{}"},
                        }
                    ],
                },
                {
                    "role": "tool",
                    "content": "synthetic-result",
                    "tool_call_id": "call-1",
                },
            ],
        },
    )
    response = {"model": "app-test", "input": "synthetic", "store": False}
    assert accepts("ResponsesInput", response)
    assert not accepts("ResponsesInput", {**response, "previous_response_id": "other"})
    assert not accepts("ResponsesInput", {**response, "store": True})
    assert accepts(
        "ResponsesInput",
        {
            **response,
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "input_text", "text": "synthetic"}],
                },
                {
                    "type": "function_call",
                    "call_id": "call-1",
                    "name": "check",
                    "arguments": "{}",
                },
                {
                    "type": "function_call_output",
                    "call_id": "call-1",
                    "output": "synthetic-result",
                },
            ],
        },
    )
    profile = {
        "slug": "app-test",
        "display_name": "Test",
        "mode": "pinned_test",
        "deployments": [
            {
                "connection_id": "11111111-1111-4111-8111-111111111111",
                "generation": 1,
                "model_id": "model-a",
            }
        ],
        "parameters": {},
        "input_limit": 100,
        "output_limit": 20,
        "context_limit": 120,
        "required_capabilities": ["text"],
        "timeout_seconds": 10,
        "max_attempts": 1,
    }
    assert accepts("ProfileInput", profile)
    assert not accepts("ProfileInput", {**profile, "max_attempts": 2})
    assert not accepts(
        "ProfileInput", {**profile, "deployments": profile["deployments"] * 2}
    )
    assert not accepts(
        "BudgetInput",
        {
            "scope_type": "client",
            "scope_id": "a",
            "currency": "USD",
            "period": "utc_day",
            "hard_limit": "-1",
            "warning_thresholds": [80],
        },
    )
    assert accepts(
        "Rate",
        {"numerator": 0, "denominator": 0, "value": None, "basis": "explicit_probes"},
    )
    assert not accepts(
        "Rate",
        {"numerator": 0, "denominator": 0, "value": 1, "basis": "explicit_probes"},
    )
    source_price = {
        "connection_id": "10000000-0000-4000-8000-000000000001",
        "model_id": "model-a",
        "mode": "provider_auto",
        "manual_price_revision_id": None,
        "expected_version": 0,
        "currency": "USD",
        "effective_from": "2026-10-09T13:30:00Z",
        "effective_to": None,
    }
    assert accepts("PricingSourceInput", source_price)
    assert not accepts("PricingSourceInput", {**source_price, "mode": "manual"})
    assert not accepts(
        "PricingSourceInput", {k: v for k, v in source_price.items() if k != "currency"}
    )
    assert accepts("PricingSourceInput", {**source_price, "currency": "EUR"})
    assert accepts(
        "PricingSourceInput",
        {
            **source_price,
            "mode": "manual",
            "manual_price_revision_id": "20000000-0000-4000-8000-000000000001",
        },
    )
    tariff = {
        "namespace": {
            "registry_instance_id": source_price["connection_id"],
            "namespace_id": "20000000-0000-4000-8000-000000000001",
        },
        "virtual_model_id": None,
        "currency": "USD",
        "mode": "default_markup",
        "markup_bps": 2000,
        "input_uncached": None,
        "input_cached": None,
        "output_billable": None,
        "unit": "per_million_tokens",
        "effective_from": "2026-10-09T00:00:00Z",
        "effective_to": None,
        "expected_version": 0,
    }
    assert accepts("ProjectTariffInput", tariff)
    assert not accepts("ProjectTariffInput", {**tariff, "namespace": None})
    assert not accepts("ProjectTariffInput", {**tariff, "mode": "custom_rates"})
    custom = {
        **tariff,
        "mode": "custom_rates",
        "markup_bps": None,
        "input_uncached": "3",
        "output_billable": "10",
    }
    assert accepts("ProjectTariffInput", custom)
    assert not accepts(
        "ProjectTariffInput", {**custom, "input_uncached": "0.0000000000001"}
    )
    assert not accepts("ProjectTariffInput", {**custom, "input_uncached": 3})
    assert not accepts("ProjectTariffInput", {**tariff, "unit": "per_token"})
    assert not accepts("ProjectTariffInput", {**tariff, "markup_bps": 2000.5})
    assert not accepts(
        "ProjectTariffInput",
        {
            **tariff,
            "namespace": {
                **tariff["namespace"],
                "namespace_id": "00000000-0000-0000-0000-000000000000",
            },
        },
    )
    vector = json.loads(
        (root / "docs/examples/service-adapter-vector.json").read_text(encoding="utf-8")
    )
    body = json.loads(vector["raw_body_utf8"])
    assert accepts("ServiceInferenceInput", body)
    response_body = {
        **body,
        "protocol": "responses",
        "invocation": {"model": "main-dev", "input": "synthetic", "store": False},
    }
    assert accepts("ServiceInferenceInput", response_body)
    assert not accepts("ServiceInferenceInput", {**response_body, "protocol": "chat"})
    assert not accepts("ServiceInferenceInput", {**body, "protocol": "responses"})
    assert accepts("ServiceInferenceClaims", vector["claims"])
    assert accepts("ExecutionContextV2", body["execution_context"])
    assert not accepts("ServiceInferenceInput", {**body, "task_id": "legacy"})
    assert not accepts(
        "ServiceInferenceClaims", {**vector["claims"], "fencing_token": 1}
    )
    assert not accepts(
        "ServiceInferenceClaims", {**vector["claims"], "profile_revision_id": "12"}
    )
    budget = {
        "scope_type": "project",
        "scope_id": tariff["namespace"]["namespace_id"],
        "namespace": tariff["namespace"],
        "currency": "USD",
        "period": "utc_day",
        "hard_limit": "0.000001",
        "warning_thresholds": [80],
    }
    assert accepts("BudgetInput", budget)
    assert not accepts("BudgetInput", {**budget, "namespace": None})
    assert not accepts("BudgetInput", {**budget, "scope_id": "Платформа"})
    score = {
        "case_id": "a1",
        "profile_revision_id": vector["claims"]["profile_revision_id"],
        "repetition": 1,
        "score": 0.125,
        "reason": "Synthetic",
        "expected_version": 0,
    }
    assert accepts("ManualScoreInput", score)
    assert accepts("ManualScoreInput", {**score, "score": 0.29})
    assert not accepts("ManualScoreInput", {**score, "score": 0.1255})
    assert not accepts("ManualScoreInput", {**score, "repetition": 0})
    exported = json.loads(
        (root / "docs/examples/statistics-export.json").read_text(encoding="utf-8")
    )
    assert accepts("Statistics", exported["payload"])
    full_filters = {
        **exported["payload"]["filter_echo"],
        "provider_id": vector["claims"]["client_id"],
        "actual_model": "model-a",
        "evaluation_run_id": vector["claims"]["request_id"],
        "timezone": "UTC",
    }
    assert accepts("StatisticsFilters", full_filters)
    assert not accepts("StatisticsFilters", {**full_filters, "group_by": "model"})
    assert not accepts("StatisticsFilters", {**full_filters, "actual_model": 12})
    assert not accepts("StatisticsFilters", {**full_filters, "provider_id": "name"})
    cancellation = {
        "expected_policy_version": 1,
        "reason": "Synthetic cancelled schedule",
    }
    assert accepts("TariffCancellationInput", cancellation)
    assert not accepts("TariffCancellationInput", {**cancellation, "reason": ""})
    assert not accepts(
        "TariffCancellationInput", {**cancellation, "expected_policy_version": 0}
    )
    unknown_charge = {
        "id": vector["claims"]["request_id"],
        "request_id": vector["claims"]["request_id"],
        "attempt_id": vector["claims"]["request_id"],
        "namespace": None,
        "tariff_revision_id": None,
        "cost_source_revision_id": None,
        "cost_source_state": "unconfigured",
        "basis_source": "unavailable",
        "currency": "USD",
        "cost_basis": None,
        "project_amount": None,
        "margin_amount": None,
        "basis_confidence": "unknown",
        "status": "pending",
        "occurred_at": "2026-10-09T13:30:00Z",
        "source_event_id": "synthetic-unpriced",
        "supersedes_id": None,
    }
    assert accepts("ProjectCharge", unknown_charge)
    late_receipt = {
        **unknown_charge,
        "basis_source": "provider_receipt",
        "basis_confidence": "confirmed",
        "cost_basis": "0.05",
        "project_amount": "0.06",
        "margin_amount": "0.01",
        "status": "final",
        "source_event_id": "synthetic-trusted-late-receipt",
        "supersedes_id": vector["claims"]["client_id"],
    }
    assert accepts("ProjectCharge", late_receipt)
    assert not accepts(
        "ProjectCharge", {**late_receipt, "basis_source": "price_estimate"}
    )
    assert not accepts("ProjectCharge", {**unknown_charge, "cost_basis": "0"})
    assert not accepts(
        "ProjectCharge",
        {**unknown_charge, "cost_source_revision_id": vector["claims"]["client_id"]},
    )
    assert not accepts(
        "ProjectCharge", {**unknown_charge, "cost_source_state": "configured"}
    )
    assert accepts(
        "ProjectCharge",
        {
            **unknown_charge,
            "cost_source_state": "configured",
            "cost_source_revision_id": vector["claims"]["client_id"],
        },
    )
    assert not accepts(
        "Statistics", {**exported["payload"], "filter_echo": {"binding": "all"}}
    )
    print("Service vector / Namespace budget / manual score schema examples: PASS")
    print("Project tariff/source positive and negative schema examples: PASS")
    print("Independent OpenAPI 3.1 and schema validation: PASS")
    print("Money/draft/tools/stateless/pinned-mode examples: PASS")
    print("Application behavior and live provider access: NOT RUN")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
