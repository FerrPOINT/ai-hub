#!/usr/bin/env python3
"""Export the UI input reference from the authoritative design/API contracts."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def label(schema):
    if "$ref" in schema:
        return schema["$ref"].rsplit("/", 1)[-1]
    if "oneOf" in schema:
        return " / ".join(label(x) for x in schema["oneOf"])
    value = schema.get("type", "object")
    return " / ".join(value) if isinstance(value, list) else value


def build():
    design = json.loads((ROOT / "docs/design/design-contract.json").read_text())
    spec = json.loads((ROOT / "docs/contracts/openapi.v1.json").read_text())
    lines = [
        "# Поля форм и payload mapping",
        "",
        "Canonical schema — draft OpenAPI; этот reference экспортирован из design contract.",
        "Derived fields не редактируются пользователем. Runtime DTO/validation ещё не реализованы.",
        "Native prototype constraints иллюстрируют UX; server authorization/validation обязательны.",
        "",
    ]
    for key, form in design["forms"].items():
        schema = spec["components"]["schemas"][form["schema"]]
        lines.extend(
            [
                f"## {key} — {form['operation']}",
                "",
                f"Body schema: {form['schema']}.",
                "",
                "| Поле | Тип | Required | Constraints / default |",
                "|---|---|---|---|",
            ]
        )
        for name in form["fields"]:
            prop = schema["properties"][name]
            bounds = {
                k: v
                for k, v in prop.items()
                if k
                in (
                    "format",
                    "pattern",
                    "enum",
                    "const",
                    "minimum",
                    "maximum",
                    "minLength",
                    "maxLength",
                    "minItems",
                    "maxItems",
                    "default",
                )
            }
            text = json.dumps(bounds, ensure_ascii=False).replace("|", "\\|")
            lines.append(
                f"| {name} | {label(prop)} | {'да' if name in schema.get('required', []) else 'нет'} | "
                + chr(96)
                + text
                + chr(96)
                + " |"
            )
        lines.extend(
            [
                "",
                "Derived: "
                + ("; ".join(form["derived"]) or "только actor/operation metadata."),
                "Result: " + form["result"] + ".",
                "",
            ]
        )
    lines.extend(
        [
            "## Header и field-error conventions",
            "",
            "Idempotency-Key — UUID на новый user intent, stable при readback; ETag/If-Match",
            "берётся из exact config response. Principal/project provenance не caller свободный ввод.",
            "Decimal amounts strings: financial18 decimals/rates12; no browser float arithmetic.",
            "UTC intervals [from,to), nullable unknown не empty zero. Bounds/capabilities конфликтуют",
            "до external I/O. Invalid fields aria-invalid + associated error; input retained.",
            "One-time key generation — отдельная explicit operation, no hidden issue after save.",
            "",
        ]
    )
    return "\n".join(lines)


if __name__ == "__main__":
    (ROOT / "docs/UI_FIELD_REFERENCE.md").write_text(build(), encoding="utf-8")
    print("UI field reference exported from design/API contracts.")
