# Проверка окончательного прототипа

Дата: 2026-10-09. Kind: prototype. Surface: Codex in-app browser.
[Evidence](evidence.json) привязан к окончательным HTML/script hashes.

- 230 geometry / 179 states / 123 flow assertions: 17 маршрутов; 375/1440/1920/2560 во всех трёх темах, 26 дополнительных тарифных размеров/границ.
- 74 native PNG; размеры проверены. Пять снимков просмотрены напрямую: desktop tariffs, light source catalog, mobile model и две mobile price forms. Остальные снимки — захват с geometry checks, не отдельная ручная визуальная приёмка.
- 15 контрастных проверок text tokens ≥4.5; полный WCAG/accessibility audit не выполнялся.
- 70 основных, 25 тарифных и 28 adversarial сценариев. Back/draft, generation/proof, stable-ID isolation, manual scoring, archive, exact export, currency/time/model pricing проверены.
- Unexpected JavaScript errors: 0; provider calls: 0. Complete IAB script прочитан частями по 100000 символов и совпал с локальным.
- Scoped IAB CDP viewport и native PNG; размеры новой вкладки не выводились из browser-wide override.

Разбор всех замечаний: [REVIEW_RESOLUTION](../REVIEW_RESOLUTION.md).
Цены и авторизация синтетические. Backend/API/DB/SSO/providers и actual application acceptance not_run.
Макет не хранит данные через полный reload: URL context/tabs восстанавливаются, in-memory demo records сбрасываются. Production persistence описана в API/DD.
