# Проверка актуализированного прототипа

Дата: 2026-10-09. Kind: prototype. Surface: Codex in-app browser.
Canonical [evidence](evidence.json) относится к финальному HTML/script текущих тарифов.

- 230 geometry / 179 states / 94 flow assertions: 17 маршрутов, 375/1440/1920/2560 во всех трёх темах; дополнительные тарифные границы 320/374/376/767/768/1279/1280.
- 71 native PNG конкретной IAB вкладки; размер изображения проверен. Четыре тарифных снимка просмотрены непосредственно; остальные — захват, не отдельная ручная визуальная приёмка.
- 15 token contrast checks; это не formal full WCAG acceptance.
- Unexpected JavaScript errors: 0; provider calls: 0.
- Viewport применяется через scoped IAB CDP. Общий browser viewport не подтверждает размер новой фоновой вкладки; screenshot clip учитывает origin прокрученного viewport.
- 24 тарифных сценария: default 20%, custom per-million, auto/manual, subscription missing rate, exact percentage, Namespace isolation, history, 412 и URL reload/Back/Forward. [Scoped evidence](tariffs-evidence.json).

## Границы приёмки

Structural/rendered gates не закрывают открытый semantic audit proof/Namespace budgets/datasets/exports/consumer transport. Полный статус DEVELOP_READY остаётся на актуализации.
Backend/API/SQL/SSO/live adapters не реализованы. Цены синтетические; автоматический режим в макете не читает OpenRouter и не выполняет платежи.
Prototype показывает представительные USD/main-dev examples; multi-profile/currency/effective interval application forms реализуются по полному [контракту тарифов](../PROJECT_TARIFFS.md).
Provider cash, manual subscription allocation и project charge различаются; неизвестная сумма не бесплатный вызов.
