# Приёмка дизайна

Kind: prototype. Surface: Codex in-app browser. Native screenshots без редактирования.
[Evidence](evidence.json) фиксирует hashes и фактическую матрицу.

- 15 operational routes при375/1440/2560 и all routes в dark/gray/light.
- Дополнительные320/390/430/767/768/1023/1024/1279/1280 на wide/reading/detail.
- 102 geometry checks: no document overflow, clipped uncontrolled content или
  непредусмотренных mobile horizontal scrollers.
- 165 state checks: loading/empty/error/403/404/partial/stale/412/pending/
  budget-exhausted/long, including no CRUD at denied/missing scope and pending guards.
- 10 flow records: draft/proof/publication/pinned/bounds, dirty/stale/412,
  exact lookup/neighbor, explicit key issue, typed budget/modal focus, dataset JSON,
  drawer keyboard, neutral login и expenses/prices/subscriptions/currency.
- 15 color-token contrast checks: normal/muted/success/warning/danger over actual
  surface in three themes; min measured ratio >6.4. Это не formal full WCAG audit.
  -43 viewport/tab/state/auth screenshots; long pages интерактивно scrollable.
  Gallery [gallery](gallery.html) и prototype показывают полный content.

## Разделение evidence

Синтетические actors/amounts/clock/models не утверждают actual provider availability
или bill. UI error/unknown/expired proof — simulation. Browser tested artifact
matches final source, но API/DB/auth/SSO real acceptance not_run.
Source-only assertions не подменяют rendered design; future actual-app screenshots
собираются по TC и served build identity отдельно.
