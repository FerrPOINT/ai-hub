# Трассировка

[traceability.json](traceability.json) — machine-readable planned requirements,
stage DAG и behavioral test definitions; текст и JSON проверяются вместе.
Все 37 requirements принадлежат AI Hub; cross-product участники указаны в handoff.
Полные behavioral oracles сохраняют planned/not_run до полной приёмки.
Реализованные slices и partial evidence отдельно отмечены в
[implementation-evidence.json](implementation-evidence.json); частичный pass
не закрывает весь TC.

| Requirement | Owner specification                                        | Stage | Behavioral oracle |
| ----------- | ---------------------------------------------------------- | ----- | ----------------- |
| FR-001      | [Подключения провайдеров](PROVIDERS.md)                    | S2    | TC-001            |
| FR-002      | [Секреты и managed authorization](SECURITY.md)             | S2    | TC-002            |
| FR-003      | [Проверка возможностей](PROVIDERS.md)                      | S2    | TC-003            |
| FR-004      | [Версии виртуальных моделей](ROUTING.md)                   | S2    | TC-004            |
| FR-005      | [Маршрутизация и fallback](ROUTING.md)                     | S3    | TC-005            |
| FR-006      | [Повтор и неопределённый исход](contracts/INFERENCE_V1.md) | S3    | TC-006            |
| FR-007      | [Inference API](API.md)                                    | S3    | TC-007            |
| FR-008      | [Подписки и квоты](PROVIDERS.md)                           | S4    | TC-008            |
| FR-009      | [Фактическая модель и trace](ROUTING.md)                   | S3    | TC-009            |
| FR-010      | [Usage и цены](ACCOUNTING.md)                              | S3    | TC-010            |
| FR-011      | [Реальные расходы и подписки](ACCOUNTING.md)               | S4    | TC-011            |
| FR-012      | [Бюджет и предупреждения](ACCOUNTING.md)                   | S3    | TC-012            |
| FR-013      | [Статистика по разрезам](ANALYTICS.md)                     | S5    | TC-013            |
| FR-014      | [Качество обслуживания](ANALYTICS.md)                      | S5    | TC-014            |
| FR-015      | [Экспорт](ANALYTICS.md)                                    | S5    | TC-015            |
| FR-016      | [Диагностика](MONITORING.md)                               | S5    | TC-016            |
| FR-017      | [Сравнение для dev/test](EVALUATIONS.md)                   | S6    | TC-017            |
| FR-018      | [Права и атрибуция](SECURITY.md)                           | S1    | TC-018            |
| FR-019      | [Отзыв](SECURITY.md)                                       | S1    | TC-019            |
| FR-020      | [Endpoint и tools boundary](THREAT_MODEL.md)               | S2    | TC-020            |
| FR-021      | [Интеграция Base и Admin](contracts/ADMIN_HANDOFF_V1.md)   | S7    | TC-021            |
| FR-022      | [Сохранность и restore](BACKUP_RESTORE.md)                 | S7    | TC-022            |
| FR-023      | [Аудит управления](SECURITY.md)                            | S1    | TC-023            |
| FR-024      | [UI сценарии](UI_UX.md)                                    | S2    | TC-024            |
| FR-025      | [Валюты и отображение](ACCOUNTING.md)                      | S5    | TC-025            |
| FR-026      | [Ошибки и ограничения](contracts/INFERENCE_V1.md)          | S3    | TC-026            |
| NFR-001     | [Consistency](DATA_MODEL.md)                               | S3    | TC-027            |
| NFR-002     | [Recovery](ARCHITECTURE.md)                                | S3    | TC-028            |
| NFR-003     | [Performance bounds](QUALITY_GATE.md)                      | S7    | TC-029            |
| NFR-004     | [Reproducible build](BASE_INTEGRATION.md)                  | S1    | TC-030            |
| NFR-005     | [Privacy и retention](SECURITY.md)                         | S5    | TC-031            |
| NFR-006     | [Responsive/a11y](UI_UX.md)                                | S5    | TC-032            |
| NFR-007     | [Contract compatibility](API.md)                           | S7    | TC-033            |
| NFR-008     | [Evidence integrity](QUALITY_GATE.md)                      | S7    | TC-034            |

[UI map](ui-routes.json) связывает 16 operational routes и auth /login с target operation IDs,
TC-024 и TC-032; draft OpenAPI покрывает весь перечисленный API.
No missing test автоматически не означает implementation acceptance:
код/DB/target evidence собирается по TESTING/QUALITY_GATE после разработки.

| FR-027 | [Спецификация](contracts/NAMESPACE_V1.md) | S1 | TC-035 |

| FR-028 | [Спецификация](contracts/ADMIN_HANDOFF_V1.md) | S7 | TC-036 |

| FR-030 | [Спецификация](PROJECT_TARIFFS.md) | S5 | TC-038 |
| FR-029 | [Спецификация](OPERATIONS.md) | S5 | TC-037 |

## Дополнение FR-030

FR-030 → TC-038 → S5: [тарифы](PROJECT_TARIFFS.md), OpenAPI PricingSourceInput/ProjectTariffInput/ProjectCharge, own typed dictionary и /tariffs prototype. Admission/ledger snapshots S3, provider qualification S4, UI S5. [Machine traceability](traceability.json).

TC-039 дополняет FR-004/017/021/027/030/NFR-007: [machine traceability](traceability.json), [SERVICE_ADAPTER_V1](contracts/SERVICE_ADAPTER_V1.md), pricing/prototype adversarial checks. Старые критерии сохранены; проверки усиливают varied-ID/name и recovery coverage.

TC-040 дополняет FR-002/004/009/012/013/021/030/NFR-007: stage prerequisites, unknown source, canonical filters, cancellation and protocol boundary. Machine traceability сохраняет предыдущие criteria; actual app acceptance planned.
