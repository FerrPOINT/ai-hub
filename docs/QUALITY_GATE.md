# Гейты качества

## Действующий documentation gate

```shell
python scripts/check_docs.py
python scripts/check_design.py
python -m unittest discover -s scripts/tests -v
uv run --no-project --with openapi-spec-validator==0.7.2 python scripts/validate_contract.py
git diff --check
```

Проверка файлов/UTF8, ссылок/anchors, coverage requirements/tests/phases/routes,
OpenAPI refs/operation mapping и synthetic decimal example.
Structural PASS требует ручного semantic review owner boundaries, cost confidence,
streaming/retry/cancel, access и planned-vs-verified claims.
Проверка OpenAPI meta-schema отдельным installed validator дополняет structural
gate, если он доступен; semantic acceptance остаётся TC-007/033.

## После actual code

| Gate         | Scope и доказательство                                                                             |
| ------------ | -------------------------------------------------------------------------------------------------- |
| Rust         | fmt, clippy -D warnings, --locked tests/build; isolated domain deps                                |
| PostgreSQL   | Real own schema fresh/upgrade/locks/CAS/ledger/restore                                             |
| Frontend     | Frozen dependencies, types/unit/lint, generated client drift, packed Base consumer, computed theme |
| API          | Rust-generated OpenAPI + old-client compatibility/SSE/tool/authorization cases                     |
| Images       | Exact source/Base/digest, non-root and own state/config                                            |
| Live         | Actual target version, SSO/logout/PAT/service grants, provider and consumer                        |
| UI           | All operational routes/states/viewport/themes and fresh screenshots                                |
| Financial    | Every request/attempt, knownness, charge/estimate/unknown/reserve consistency                      |
| Restore/load | Canonical receipts with real nonempty own state and bounded measurement                            |

Запланированные commands формируются вместе с manifests, затем фиксируются здесь.
Не писать «cargo test passed» без исполнимого проекта/результата.

## Performance targets

На isolated qualified host с hardware receipt: 20 concurrent controlled requests,
own admission overhead p95<=50ms без provider latency; 100000 representative rows,
90-day range summaries p95<=2s; response/worker buffers bounded.
Методика отделяет warm/cache/DB/network, записывает sample/window/content bounds.
Это цели NFR-003; tuning не повод менять acceptance criteria на измеренный провал.

## Evidence и ready

Documentation prepared, source implemented, local gate, hosted checks, image runtime
и user/provider acceptance — разные статусы. Нельзя выбрать самый зелёный.
Before release: all requirements tested, blockers resolved/explicit accepted,
served source identity and config verified, backup/rollback/load checks attached.
Hosted CI optional mirror; локальные обязательные gates не заменяются его отсутствием.

## Актуализация 2026-10-09

Дополнительный действующий gate: `python scripts/check_alignment.py`. Он проверяет source mapping, namespaces, filters и exact legacy decimal conversion. IAB regression runner `scripts/design_flows.mjs` принимает существующий IAB tab; отдельный браузер не запускает.

Additional semantic gates: `python scripts/check_alignment.py`, `python scripts/check_semantics.py`, `node scripts/check_service_contract.mjs`. Service vectors и Statistics export проверяются independent schema validator; actual transport not_run.

Implementation-readiness prerequisite gate: `python scripts/check_readiness.py`. Он сравнивает stage FK/probe prerequisites, canonical query/echo/export refs, cancellation scope/CAS/durability, unknown source guard и protocol union. Независимые schema/negative tests и rendered IAB дополняют этот static check.
