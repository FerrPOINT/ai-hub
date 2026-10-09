# Готовность к разработке

> Текущий статус: DEVELOP_READY для документов и дизайна. Actual application/runtime acceptance not_run; main/Namespace integration остаётся отдельной вехой. См. [CURRENT_STATE](CURRENT_STATE.md).

Этот gate проверяет достаточность документационного baseline для старта S1.
Работающий UI/API, provider access и runtime release здесь не требуются и не
подменяются фиктивной реализацией. Product acceptance после S7 — отдельный gate.

## Проверяемые входы

| Область                       | Authoritative evidence                                                   | Проверка                                                            |
| ----------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------- |
| Прямой пользовательский scope | TZ, PRODUCT_REQUIREMENTS                                                 | Gateway, custom profiles/dev-test, статистика и расходы покрыты     |
| Стандарты репозитория         | README/AGENTS/CHANGELOG/CONTRIBUTING/SECURITY/LICENSE/NOTICE/THIRD_PARTY | Нет заглушек и ложных runtime claims; license/notice exact Base     |
| Owners/альтернативы           | ARCHITECTURE, ADR, ADMIN_HANDOFF                                         | Нет второго владельца данных/config, no shared DB/credentials       |
| Domain/data                   | DOMAIN_MODEL/DATA_MODEL/MIGRATIONS                                       | Request/attempt/revision/facts/ledger и lifecycle согласованы       |
| API                           | API, INFERENCE_V1, draft OpenAPI                                         | Typed operations/wire/errors, unsupported semantics explicit        |
| Финансы                       | ACCOUNTING, ANALYTICS, decimal fixture                                   | Не смешаны estimate/charge/subscription/unknown/currencies          |
| UI                            | UI_UX, route manifest/frontend docs                                      | Все действия имеют API, states и будущую UI/viewport проверку       |
| Security/ops                  | SECURITY/THREAT_MODEL/ENV/backup/deploy                                  | Grants/SSRF/vault/recovery и protected resources определены         |
| Реализация                    | IMPLEMENTATION_PLAN/ROADMAP/RISK_REGISTER                                | Ready S1, dependencies/components/oracles и future gates            |
| Проверки                      | TESTING/QUALITY_GATE/TRACEABILITY                                        | Каждый FR/NFR связан с planned behavioral tests                     |
| Источники                     | SOURCE_AUDIT                                                             | Точные местные snapshots, external official references, limitations |
| Publication                   | Clean Git + exact remote head readback                                   | Docs committed/pushed, не compare URL вместо delivery               |

## Статус

DEVELOP_READY для начала S1. Offline, semantic и rendered design gates завершены.
Проверены root matrix, owner boundaries/transition, 38 requirements и их 39
planned behavioral tests, stage DAG, 16 operational + 1 auth UI routes и соответствующие API actions.
Согласованы draft-before-proof, publication TTL/runtime qualification, streaming
readback/own result grants, financial precision и static platform capability paths.
Рабочие команды проверки в QUALITY_GATE воспроизводимы; independent OpenAPI
validator проверяет meta-schema и synthetic positive/negative payloads.
Remote exact-head receipt собирается после публикации готового baseline и
указывается в handoff. Он не создаёт release или application acceptance.
Отдельные будущие acceptance states: application build/DB/runtime/provider/browser
not_run. Пакет позволяет начать S1 без доступа к платным providers.

Полный design handoff: DESIGN_SYSTEM/UI_SCREEN_SPEC/UI_FIELD_REFERENCE и
source-bound design evidence. Недостаточно одного source map без rendered checks.

## Актуализация 2026-10-09

Namespace, Admin extraction, per-model preferences и notification contract согласованы. Текущая design QA: 230 geometry / 179 states / 123 flow assertions. [Статус](CURRENT_STATE.md), [evidence](design/evidence.json). Реальные execution gates not_run.
