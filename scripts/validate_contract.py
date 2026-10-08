#!/usr/bin/env python3
"""Independent validation of the design OpenAPI and contract examples."""

import argparse
import json
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
    registry = Registry().with_resource(
        uri, Resource.from_contents(spec, default_specification=DRAFT202012)
    )

    def accepts(name, value):
        checker = Draft202012Validator(
            {"$ref": f"{uri}#/components/schemas/{name}"},
            registry=registry,
            format_checker=FormatChecker(),
        )
        return checker.is_valid(value)

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
    print("Independent OpenAPI 3.1 and schema validation: PASS")
    print("Money/draft/tools/stateless/pinned-mode examples: PASS")
    print("Application behavior and live provider access: NOT RUN")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
