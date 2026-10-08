# Roadmap

## Начало и граф зависимостей

| Stage                                         | Зависит от             | Observable checkpoint                                                                                                   |
| --------------------------------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| S1 Own foundation и trusted access            | Documentation baseline | Изолированное приложение принимает trusted principal, имеет own schema/audit и живые readiness/SSO проверки.            |
| S2 Connection и virtual model lifecycle       | S1                     | Synthetic provider подключён, capabilities проверены; draft публикует immutable revision через UI/API.                  |
| S3 Inference с financial ledger               | S2                     | Реальный controlled upstream отвечает full/stream/tools через scoped Hub key, retries/fallback/budget/restart доказаны. |
| S4 Нужные внешние providers и subscriptions   | S3                     | Каждый выбранный account/adapter qualified live; own auth и subscription/quota receipts проверены.                      |
| S5 Статистика, расходы и operational UI       | S3, S4                 | Сводки/разрезы/export совпадают с ledger; late facts, partial failures, multi-currency и thresholds проверены.          |
| S6 Воспроизводимые model evaluations          | S4, S5                 | Одинаковый dataset на pinned profiles даёт проверяемые результаты, реальные расходы и cancel/recovery.                  |
| S7 Integration, restore и полный release gate | S6                     | Выбранный consumer проходит exact-target полный путь; own restore, rollout/rollback и итоговая приёмка доказаны.        |

Ready frontier сейчас S1 после успешного [PRE_DEVELOPMENT_GATE](PRE_DEVELOPMENT_GATE.md).
S2 synthetic qualification не платный proof реального аккаунта. S3 проходит
на controlled upstream, S4 добавляет actual providers.
First usable product включает S1–S6 целиком: UI/API/provider routing, accounting/
statistics и evaluations. S7 закрывает интеграцию/restore/release readiness.
Production rollout не следует автоматически из этого roadmap.

## Последующие расширения

Media/embeddings, provider-specific features, pricing import automation/FX,
LLM judges, traffic experiments и client-facing billing — отдельные requirements.
Не добавлять заранее реализации или заглушки. Даты/оценки фиксируются после S1
feedback/actual capability qualification; roadmap не обещает неизмеренные сроки.
