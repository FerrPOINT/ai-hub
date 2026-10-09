# Приёмка окончательного прототипа перед реализацией

Дата: 2026-10-09. Kind: prototype. Surface: Codex in-app browser.
[Evidence](evidence.json) содержит hashes финального HTML/script, совпавшего с полным IAB DOM script.

- 230 geometry / 179 states / 146 flow assertions: 17 routes, 375/1440/1920/2560 × dark/gray/light; 26 дополнительных тарифных/source размеров и границ 320/374/376/767/768/1279/1280.
- 70 основных, 25 тарифных, 28 прежних semantic и 23 readiness assertions. Cancellation lifecycle, active boundary/pending/CAS/original readback/time identity/namespace; source absence, native cost-policy enum, group/timezone reload, exact JSON/CSV, malformed tool/schema и literal neighbour проверены.
- 77 native PNG с проверенными размерами. Семь просмотрены напрямую: mobile schedule/cancel form/cancelled, desktop cancellation/readback, light source catalog, mobile model. Остальные — захват с geometry/AX checks, не отдельная ручная pixel-by-pixel приёмка.
- 15 normal-text token contrast checks ≥4.5; formal full WCAG audit не выполнялся.
- 0 unexpected JS errors; 0 provider calls. Complete script read в chunks по 100000 chars, не truncated summary. Viewport applied/captured через scoped IAB CDP, overrides reset.

READY-01–05 и точные implementation semantics: [READINESS_AUDIT](../READINESS_AUDIT.md).
Scorer suite/knownness/accounting — planned production behavior, not fake live result.
In-memory demo records не persist через reload; URL/tab state persist. Реальные API/DB/SSO/provider/cash/consumer acceptance not_run.
