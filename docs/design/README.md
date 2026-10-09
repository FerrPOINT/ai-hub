# Дизайн AI Hub

[Интерактивный прототип](prototype.html) — 15 operational routes и auth /login с синтетическими
данными; это design artifact, не приложение с provider/DB/SSO.
[design-contract.json](design-contract.json) фиксирует pages/forms/states/API mapping.
Token snapshot взят из exact Base SHA в .base-revision; production потребляет @sdlc/ui,
не копирует CSS прототипа как новый UI-kit.

## Открыть локально

Из корня репозитория, Python 3.11+:

```shell
python -m http.server 53061 --bind 127.0.0.1 --directory docs
```

Открыть http://127.0.0.1:53061/design/prototype.html в Codex in-app browser.
Header account menu меняет тему; footer меняет state. Navigation, forms, confirmation,
draft/proof/publication, quota/unknown/readback и datasets — безопасная simulation.
Никаких реальных credentials, ключей приложения, платежей и внешних model calls.

## Authority и handoff

- [Design system](../DESIGN_SYSTEM.md) — композиция и tokens.
- [Screen specification](../UI_SCREEN_SPEC.md) — hierarchy/actions/states.
- [Field reference](../UI_FIELD_REFERENCE.md) — typed payloads и source/derived поля.
- [Use cases](../USE_CASES.md) — основные paths.
- [DEVELOP_READY](../DEVELOP_READY.md) — критерии полного handoff.
- Screenshot/geometry evidence фиксируется после финального render; не old runtime acceptance.

Prototype routes используют читаемые demo IDs, production API — opaque UUID из DTO.
Не переносить demo names, fixed clock, fixture amounts, native no-op feedback или
QA toolbar в production flows. Product screenshots и design screenshots имеют
разные manifests/qa kinds.

[Повторная UI/UX проверка](UX_RECHECK.md) описывает исправленные ошибки и сценарии регрессии.

Namespace в шапке имеет tab-local URL. Контекст модели сохраняется отдельно по exact connection/model. История/откат и уведомления — simulation; новые требования и источники в [CURRENT_STATE](../CURRENT_STATE.md).

## Тарифы и актуальная приёмка

/tariffs: проекты и себестоимость подключений. [QA](QA.md) и [evidence](evidence.json) привязаны к финальному исходнику. Semantic audit остаётся открытым; rendered PASS не означает DEVELOP_READY или actual application acceptance.
