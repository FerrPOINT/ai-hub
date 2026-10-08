# Требования AI Hub v1

Дата baseline: 2026-10-08. Владелец всех строк — AI Hub. Статус требований planned:
пользователь согласовал подготовку продукта; реализация/приёмка не выполнены.

## Функциональные и нефункциональные требования

| ID      | Правило                         | Критерий приёмки                                                                                                                       | Stage / test | Owner specification                       |
| ------- | ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ----------------------------------------- |
| FR-001  | Подключения провайдеров         | Оператор создаёт/отключает connection; неизвестный provider/неразрешённый endpoint отклоняется; disable не удаляет историю.            | S2 / TC-001  | [Документ](PROVIDERS.md)                  |
| FR-002  | Секреты и managed authorization | Write-only ключ/authorization, own encrypted key, generation; API/log/export/browser storage не раскрывают secrets.                    | S2 / TC-002  | [Документ](SECURITY.md)                   |
| FR-003  | Проверка возможностей           | Publication требует actual proof точных account/model/generation/adapter/capabilities; catalog alone не sufficient.                    | S2 / TC-003  | [Документ](PROVIDERS.md)                  |
| FR-004  | Версии виртуальных моделей      | Создание slug/draft, CAS save, immutable revision, publication/rollback/archive; active runs не меняют snapshot.                       | S2 / TC-004  | [Документ](ROUTING.md)                    |
| FR-005  | Маршрутизация и fallback        | Ordered fallback только explicit policy и known nonacceptance; никаких смешанных streams; pinned_test один deployment.                 | S3 / TC-005  | [Документ](ROUTING.md)                    |
| FR-006  | Повтор и неопределённый исход   | Одинаковый principal/key/payload даёт readback, changed payload conflict; restart/timeout не повторяет unknown I/O.                    | S3 / TC-006  | [Документ](contracts/INFERENCE_V1.md)     |
| FR-007  | Inference API                   | Models/chat completions/stateless responses, text/stream/function tools/JSON при qualified capability; неподдержанное явно rejected.   | S3 / TC-007  | [Документ](API.md)                        |
| FR-008  | Подписки и квоты                | Own managed login/revoke, отдельная плата за период и quota observations с freshness; недоступная quota null.                          | S4 / TC-008  | [Документ](PROVIDERS.md)                  |
| FR-009  | Фактическая модель и trace      | Request связывает frozen profile и все attempts; actual model confidence виден; provider alias substitution не hidden.                 | S3 / TC-009  | [Документ](ROUTING.md)                    |
| FR-010  | Usage и цены                    | Trusted category facts × immutable rate snapshot; cached/reasoning не двойной счёт, missing usage/pricing unknown.                     | S3 / TC-010  | [Документ](ACCOUNTING.md)                 |
| FR-011  | Реальные расходы и подписки     | Confirmed receipt заменяет связанную estimate; duplicate charge/subscription не меняет totals; correction append-only.                 | S4 / TC-011  | [Документ](ACCOUNTING.md)                 |
| FR-012  | Бюджет и предупреждения         | Atomic reserve по всем scopes до I/O, unknown удерживает reserve; thresholds уведомляют один раз; actual overrun не обрезается.        | S3 / TC-012  | [Документ](ACCOUNTING.md)                 |
| FR-013  | Статистика по разрезам          | UTC range + timezone, dimensions/client/project/user/model/provider, known/unknown counts, snapshot freshness и rebuild.               | S5 / TC-013  | [Документ](ANALYTICS.md)                  |
| FR-014  | Качество обслуживания           | Success/error/cancel/fallback rates, latency/TTFT, availability/quota с sample sizes и explicit denominators.                          | S5 / TC-014  | [Документ](ANALYTICS.md)                  |
| FR-015  | Экспорт                         | Authorized bounded CSV/JSON соответствует dashboard snapshot; decimal/null semantics и formula safety сохранены.                       | S5 / TC-015  | [Документ](ANALYTICS.md)                  |
| FR-016  | Диагностика                     | Own health, bounded metrics, redacted logs, alerts budget/unknown/lag/provider; telemetry failure не стирает facts.                    | S5 / TC-016  | [Документ](MONITORING.md)                 |
| FR-017  | Сравнение для dev/test          | Frozen dataset/runner/scorers/pinned revisions без cache/fallback; evidence, actual cost и cancel/restart без дублирования.            | S6 / TC-017  | [Документ](EVALUATIONS.md)                |
| FR-018  | Права и атрибуция               | Central human/service/client principals различаются; server grants защищают direct URLs, requests, stats и artifacts.                  | S1 / TC-018  | [Документ](SECURITY.md)                   |
| FR-019  | Отзыв                           | Revoked/expired central session/PAT/key/generation запрещает новый admission/attempt; никакого credential fallback.                    | S1 / TC-019  | [Документ](SECURITY.md)                   |
| FR-020  | Endpoint и tools boundary       | SSRF/redirect/private allowlist, payload caps; Hub не исполняет shell/browser/caller function tools.                                   | S2 / TC-020  | [Документ](THREAT_MODEL.md)               |
| FR-021  | Интеграция Base и Admin         | Один AI config owner; transition contracts/rollback, own auth/catalog/nav; прежний runtime не переключается по docs.                   | S7 / TC-021  | [Документ](contracts/ADMIN_HANDOFF_V1.md) |
| FR-022  | Сохранность и restore           | Own DB/vault/keys/provenance restore в изолированный target; нет external I/O/повторного dispatch; сверяются balances/dedupe.          | S7 / TC-022  | [Документ](BACKUP_RESTORE.md)             |
| FR-023  | Аудит управления                | Mutation + append-only audit атомарны, object/revision/actor/reason; secret bodies и transcripts не попадают в audit.                  | S1 / TC-023  | [Документ](SECURITY.md)                   |
| FR-024  | UI сценарии                     | Provider/models/requests/analytics/expenses/budgets/evaluations со states, CAS, preserved drafts, readable provenance.                 | S2 / TC-024  | [Документ](UI_UX.md)                      |
| FR-025  | Валюты и отображение            | Per-currency totals, confirmed/estimated/unknown/reserved отдельно; subscription allocation не повторное cash списание.                | S5 / TC-025  | [Документ](ACCOUNTING.md)                 |
| FR-026  | Ошибки и ограничения            | Typed errors/retry-after/context/output/capability bounds, deadlined attempts; no silent truncation/downgrade.                         | S3 / TC-026  | [Документ](contracts/INFERENCE_V1.md)     |
| NFR-001 | Consistency                     | Facts/settlement/dedupe/revisions атомарны; concurrent admissions/duplicate events не меняют accepted totals.                          | S3 / TC-027  | [Документ](DATA_MODEL.md)                 |
| NFR-002 | Recovery                        | Restart/cancel/ambiguous outcomes readback сохраняет intent и reserves; нет слепого paid replay.                                       | S3 / TC-028  | [Документ](ARCHITECTURE.md)               |
| NFR-003 | Performance bounds              | Controlled load baseline: 20 concurrent requests, own overhead p95≤50ms; 90-day/100k rows stats p95≤2s; receipt включает hardware.     | S7 / TC-029  | [Документ](QUALITY_GATE.md)               |
| NFR-004 | Reproducible build              | Exact Base SHA, locked manifests, isolated own resources; source/build/image/served identity различаются.                              | S1 / TC-030  | [Документ](BASE_INTEGRATION.md)           |
| NFR-005 | Privacy и retention             | Default no transcripts, encrypted bounded replay buffer, evaluation TTL, own deletion/archival сохраняет billing identity.             | S5 / TC-031  | [Документ](SECURITY.md)                   |
| NFR-006 | Responsive/a11y                 | Base shell/themes, keyboard/focus, readable stats/currency и errors; 375/1440/1920/2560, no body overflow.                             | S5 / TC-032  | [Документ](UI_UX.md)                      |
| NFR-007 | Contract compatibility          | Versioned contracts, old clients tested; destructive schema/transport change требует version/migration; no unsupported field dropping. | S7 / TC-033  | [Документ](API.md)                        |
| NFR-008 | Evidence integrity              | Точный source/Base/image/target identity + live consumer/provider сценарии; mock/build/health не substitute user outcome.              | S7 / TC-034  | [Документ](QUALITY_GATE.md)               |

## Покрытие и завершение

[TRACEABILITY](TRACEABILITY.md) и [traceability.json](traceability.json)
связывают каждую строку с stage и проверкой.
В [TESTING](TESTING.md) positive/negative/recovery сценарии; все пока not_run.
Документационная полнота не означает выполнение перечисленных критериев.

Выбранные bounds/retention/toolchain — рабочие defaults v1, утверждённые автономно
для подготовки. Изменения оформляются ADR и обновляют соответствующие contracts/tests.
Реальные account credentials, prices, physical limits и target ports не выдумываются;
их ввод/qualification запланированы на S1/S4/S7 и не блокируют начало S1.
