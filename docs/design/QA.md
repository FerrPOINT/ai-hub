# Приёмка актуализированного дизайна

Дата: 2026-10-09. Kind: prototype. Surface: Codex in-app browser.
Канонический [evidence](evidence.json) привязан к окончательным HTML/script hashes.

- 219 проверок геометрии: 16 маршрутов, dark/gray/light, 375/1440/1920/2560 и адаптивные границы.
- 168 проверок loading/empty/error/403/404/partial/stale/long/412/pending/budget-exhausted; auth ready/loading/error отдельно.
- 73 behavioral assertions: предыдущие регрессии, Namespace URL/вкладки, per-model context, OpenRouter unknown limits, managed readback, multiple profiles, actor ACK без финансовых изменений, snapshots/rollback/archive.
- 48 нативных viewport снимков без редактирования; 20 просмотрены непосредственно. Остальные — захват, не отдельная ручная визуальная приёмка.
- 15 color-token contrast checks в трёх темах; не formal full WCAG audit.
- Unexpected JavaScript errors: 0; provider calls: 0.

## Границы

HTML — self-contained simulation. Нет настоящего API/DB/SSO/провайдера, billing или миграции.
SDK token snapshot и Namespace source reference независимы. Нативные снимки actual
приложения и full-page README evidence собираются после реальной реализации.
Fixture время: 2–8 октября; 30-дневный cash total включает подписку 1 октября на уровне установки.
Дневные confirmed суммы читаются из одного dataset графиками и breakdown. Namespace
и unbound samples входят в общую сводку; scope budgets не суммируются между собой.

Регрессионный runner: scripts/design_flows.mjs принимает существующий IAB tab,
не запускает другой браузер. Actual application/provider acceptance остаётся not_run.
