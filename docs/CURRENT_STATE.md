# Текущее состояние

Дата: 2026-10-09. Stage: DEVELOP_READY для пакета документов и дизайна.

- Замечания R01–R15 обработаны в [реестре](REVIEW_RESOLUTION.md); ограничения main/runtime cohorts описаны отдельно.
- OpenAPI 0.4.0-design; 46 typed таблиц/408 полей; 16 operational routes и /login.
- Final IAB: 230 geometry / 179 states / 123 flow assertions; 74 native PNG, 5 просмотрены; 0 JS errors/реальных provider calls.
- Тарифы: за 1M, default 20%, custom Namespace/profile/currency, automatic OpenRouter и manual Ollama Online/ChatGPT; separate activation, effective intervals и immutable history.
- Fleet/Forge target adapter замкнут по identity, signed grant, exact V2/body, revision/request/idempotency, fencing/revocation/recovery. Offline vectors не runtime qualification.
- SDK baseline, Namespace target и operator tooling независимы. Base/Admin docs публикуются на документационных branches; main integration и Namespace acceptance — отдельные gates.
- Backend/API/SQL migrations/SSO/live adapter/финансовый runtime/cutover не реализованы; actual app acceptance not_run.

Следующая веха: S1 настоящего изолированного приложения. Действующий Admin /ai,
данные, pins и установленный runtime не меняются этой поставкой.
