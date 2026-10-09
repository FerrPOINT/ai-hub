# Сценарии проверки дизайна

Scope: rendered static prototype through Codex in-app browser, not real auth/billing.
Все fake values явно помечены; backend/API security и provider acceptance остаются TC.

| Case | Steps                                                   | Expected                                                                   |
| ---- | ------------------------------------------------------- | -------------------------------------------------------------------------- |
| D-01 | Open all17 routes at375/1440/2560                       | Correct source pattern/h1/layout, no body overflow; every action reachable |
| D-02 | Dark/gray/light at all routes                           | Same geometry, readable semantic text/status/focus                         |
| D-03 | Mobile drawer open/nav/Escape                           | Named links, modal focus, return to trigger, same product actions          |
| D-04 | Unknown entity ID / permitted existing ID               | 404 vs actual data; no default first-record masking                        |
| D-05 | Create valid draft before proof, alter bounds/name/mode | Save possible; publish disabled until exact verification                   |
| D-06 | Pinned mode / development / invalid context             | One deployment/max_attempts1/no reserve; invalid bounds explained          |
| D-07 | Verify → publish confirmation → history                 | Model revision changes only simulated future new requests                  |
| D-08 | Dirty navigation / stale /412                           | Stay/discard, entered fields retained, conflict not successful mutation    |
| D-09 | Every loading/empty/error/403/404/partial/stale state   | Correct copy/action; missing money not0                                    |
| D-10 | Request completed/failed/cancelled/unknown              | Each source-specific trace and costs, safe cancel/readback                 |
| D-11 | Expenses tabs/currency/receipt/subscription             | Confidence groups and separate currencies; cash basis distinct period      |
| D-12 | Budget editor/limits/pending/denied                     | Correct fields, typed decimals, held reserve/no inferred available balance |
| D-13 | Client metadata → explicit issue/revoke                 | Save rights never hidden issue; no provider secret                         |
| D-14 | Dataset valid/invalid JSON, launch2 distinct runs       | Preserved invalid input; frozen config; partial != success                 |
| D-15 | Modal fields, native keyboard/focus and close           | Named controls, invalid/pending states, no unreachable actions             |
| D-16 | Neutral login / account / themes / notifications        | No branded callback intermediate page, no external auth call               |
| D-17 | Long labels, missing quota/usage/tiny cost              | Readable wrapping and explicit knownness                                   |
| D-18 | Served source bytes/hash + console/network              | Final prototype artifact matches, no external CDN/model/API requests       |

Evidence manifest records actual routes/states/viewport/theme, h1/readiness, geometry,
screenshot file/hash and scenario result. Screenshot itself inspected after capture.
Image baseline is only design; production screenshots under actual UI acceptance later.

## Актуализация 2026-10-09

Namespace: UUID identity/foreign registry, tab isolation, URL Back/Forward/reload, unbound, archived/unavailable и dirty guard. Admin parity: model context switching, OpenRouter, managed operation readback/no duplicate, unknown physical bounds, history/rollback, multiple client profiles и notifications no financial mutation. Матрица включает 375/1440/1920/2560 и auth route.
