# Повторное ревью документов и дизайна AI Hub

Дата: 2026-10-10. Scope: документация, контрактный dataflow, границы экосистемы
и интерактивный прототип. Новых блокирующих замечаний к этому пакету не найдено.
Следующий этап — S1 по [implementation plan](../../IMPLEMENTATION_PLAN.md).

## Проверенный исходник и границы

AI Hub `28098a573ca99b5f7d8551e3a30f5aef12ea2e82`; OpenAPI 0.5.0-design,
82 операции, 47 таблиц/420 полей, 20 записей карты выделения из Admin.
HTML SHA-256: `6304f472e631fb361c18f261905045b7cd03b884fda90be41c4466c913b58b61`.
Полный script совпал в исходнике и IAB:
`197fd00d4643ab9708e7eb7a8fd917b1ef74c2a0b3c06fbf733d32524b59803a`.

Проверены ownership/isolation, Namespace/ExecutionContextV2, signed service
transport, admission/attempt/recovery, frozen profile/price/tariff, ledger/reserve,
subscription allocation, project charges/margin, canonical statistics/export,
UI/forms/Base shell и порядок S1–S7. Точные источники и current remote main SHA
соседних сервисов сохранены в [review.json](review.json).

Base SDK pin, Namespace target cohort и operator tooling независимы. Документы
Base/Admin находятся в named docs branches; проверка не выполняла main integration.
Hub сохраняет собственные AI config/accounting boundaries; Task lifecycle,
исполнение tools, Namespace registry и central identity остаются у прежних owners.

## Свежие результаты

- Documentation/alignment/semantic/readiness/design и independent OpenAPI/schema gates: PASS.
- 39 Hub regression tests, 13 offline signed vectors и 15 Base mirror/onboarding/hub tests: PASS.
- Base manifest integrity и точное зеркало Hub: PASS на проверенном source.
- IAB: 17 маршрутов × 4 размера (375/1440/1920/2560) × 3 темы = 204 geometry checks;
  146 flow assertions; 0 JS errors и 0 provider calls.
- Источник подтверждён полным script hash; JSON/CSV экспортируют одинаковый snapshot.
- Два новых PNG просмотрены непосредственно: [desktop](tariffs-desktop.png)
  и [mobile](tariffs-mobile.png). Подробности: [browser.json](browser.json).

Прежние 179 state checks подтверждены source-bound design manifest; полностью
заново в этом ревью они не повторялись. Формальный полный accessibility audit
и настоящие application security/billing проверки не заявлены.

Учёт OpenRouter повторно сверён с
[официальным usage accounting](https://openrouter.ai/docs/cookbook/administration/usage-accounting):
usage/cost доступны в ответе, streaming usage приходит в финальном SSE;
account cost и upstream inference cost не суммируются как одно списание.

## Что предстоит при реализации

S1 квалифицирует exact SDK/Namespace owner readers, service scopes и central Auth.
Настоящие backend, SQL constraints/concurrency/recovery, generated clients, SSO,
provider adapters, signed consumer и установленный Hub runtime ещё отсутствуют.
S7 cutover требует nonempty backup, mapping, provider/consumer qualification,
финансовую сверку и rollback; текущий Admin /ai действует до принятого переключения.

Этот evidence относится к [прототипу](../../design/prototype.html), а не к работающему
приложению. Публикация отчёта и обзорных метаданных не меняет проверенный HTML/script.
