/** Adversarial review regression through a caller-owned Codex IAB tab only. */
import { readFile } from "node:fs/promises";
export async function verifyReview(tab, sourceScriptHash) {
  const flows = [],
    observe = () => tab.getAXState({ emit: false });
  const expect = (ok, name) => {
    if (!ok) throw Error(name);
    flows.push({ name, result: "PASS", sourceScriptHash });
  };
  const click = async (name, root = tab.playwright) => {
    await root.getByRole("button", { name, exact: true }).click();
    await observe();
  };
  const fill = async (id, value) => {
    await tab.playwright.locator("#" + id).fill(value);
    await observe();
  };
  const select = async (id, value) => {
    await tab.playwright.locator("#" + id).selectOption(value);
    await observe();
  };
  const nav = async (name) => {
    await tab.playwright
      .locator("#desktop-nav")
      .getByRole("link", { name, exact: true })
      .click();
    await observe();
  };
  const text = () => tab.playwright.locator("#main").innerText();
  const value = (id) =>
    tab.playwright.locator("#" + id).evaluate((e) => e.value);
  const visibleRows = () =>
    tab.playwright
      .locator("#main tbody tr")
      .filter({ visible: true })
      .allTextContents();
  const reset = async (route, extra = "") => {
    await tab.goto(
      "http://127.0.0.1:53061/design/prototype.html?theme=dark&state=ready&review_case=" +
        encodeURIComponent(route + extra) +
        extra +
        "#" +
        route,
    );
    await observe();
  };
  const A = "20000000-0000-4000-8000-000000000001",
    B = "20000000-0000-4000-8000-000000000002";
  await reset("/models");
  await tab.playwright
    .getByRole("link", { name: "main-dev", exact: true })
    .click();
  await observe();
  await fill("display-name", "Новый несохранённый ввод");
  await tab.back();
  await observe();
  expect(
    (await tab.playwright.locator("#modal").innerText()).includes(
      "Черновик не сохранён",
    ),
    "native-back-draft-guard",
  );
  await click("Остаться");
  expect(
    (await value("display-name")) === "Новый несохранённый ввод",
    "native-history-keeps-local-draft",
  );
  await click("Проверить черновик");
  expect(
    (await tab.playwright.locator("#model-validation").innerText()).includes(
      "Сохраните",
    ),
    "unsaved-proof-cannot-bypass-history",
  );
  await click("Сохранить черновик");
  await click("Проверить черновик");
  await click("Подтвердить проверку макета");
  expect(
    await tab.playwright
      .getByRole("button", { name: "Опубликовать", exact: true })
      .isEnabled(),
    "saved-version-proof-positive",
  );
  await nav("Провайдеры");
  await tab.playwright.locator('a[href="#/providers/api-a"]').click();
  await observe();
  await click("Изменить подключение");
  await select("endpoint-ref", "openrouter");
  await click("Сохранить");
  await click("Проверить модель");
  await click("Проверить в макете");
  await nav("Модели");
  await tab.playwright
    .getByRole("link", { name: "main-dev", exact: true })
    .click();
  await observe();
  expect(
    !(await tab.playwright
      .getByRole("button", { name: "Опубликовать", exact: true })
      .isEnabled()),
    "endpoint-requalification-not-profile-proof",
  );
  await reset("/evaluations");
  await select("namespace-select", A);
  await click("Добавить набор");
  expect(
    (await value("dataset-project")).includes("Платформа"),
    "dataset-project-derived-from-namespace",
  );
  await fill("dataset-name", "Набор A");
  await fill(
    "dataset-content",
    JSON.stringify([
      {
        case_id: "a1",
        input: "code",
        assertion_kind: "tool_args",
        expected: "{}",
      },
    ]),
  );
  await click("Сохранить версию");
  expect((await text()).includes("Набор A"), "schema-tool-args-accepted");
  await click("Новый запуск");
  await select("eval-scorer", "manual-v1");
  await click("Запустить в макете");
  expect(
    (await text()).includes("manual-v1") &&
      (await text()).includes("Dataset version ID"),
    "run-scorer-budget-and-identities-frozen",
  );
  await click("Добавить оценку");
  await fill("score-reason", "Ручная синтетическая приёмка");
  await click("Сохранить");
  expect(
    (await text()).includes("demo-user"),
    "manual-score-attributed-and-visible",
  );
  await nav("Тестирование");
  await select("namespace-select", B);
  expect(
    !(await text()).includes("Набор A"),
    "dataset-panel-namespace-isolation",
  );
  await click("Добавить набор");
  await fill("dataset-name", "Новое сравнение · 26");
  await click("Сохранить версию");
  await click("Новый запуск");
  await select("eval-dataset", "dataset-2");
  await click("Запустить в макете");
  await nav("Тестирование");
  expect(
    (await visibleRows()).some((x) => x.startsWith("Новое сравнение · 27")),
    "same-name-run-remains-in-own-namespace",
  );
  await select("namespace-select", A);
  expect(
    !(await visibleRows()).some((x) => x.startsWith("Новое сравнение · 27")),
    "same-name-cannot-cross-namespace",
  );
  await select("namespace-select", "20000000-0000-4000-8000-000000000003");
  expect(
    (await text()).includes("Архивный запуск"),
    "archive-history-readable",
  );
  expect(
    !(await tab.playwright
      .getByRole("button", { name: "Новый запуск", exact: true })
      .isEnabled()),
    "archive-new-admission-closed",
  );
  await reset("/expenses");
  await tab.playwright
    .getByRole("tab", { name: "Подписки", exact: true })
    .click();
  await observe();
  await tab.reload();
  await observe();
  expect(
    (await tab.playwright
      .locator('[role="tab"][aria-selected="true"]')
      .innerText()) === "Подписки",
    "expense-tabs-reload",
  );
  await tab.playwright
    .getByRole("tab", { name: "Списания", exact: true })
    .click();
  await observe();
  await click("Экспорт");
  await select("export-format", "json");
  const pending = tab.playwright.waitForEvent("download");
  await click("Скачать пример");
  const downloaded = await pending;
  const exported = JSON.parse(await readFile(await downloaded.path(), "utf8"));
  expect(
    exported.payload.expenses.every(
      (x) =>
        typeof x.confirmed_amount === "string" &&
        !x.confirmed_amount.includes("$"),
    ),
    "export-canonical-decimal",
  );
  expect(
    exported.filters.binding === "all" &&
      exported.filters.from.endsWith("Z") &&
      exported.filters.to.endsWith("Z"),
    "export-full-scope-utc-echo",
  );
  expect(
    exported.payload.usage.cached_input_tokens === null,
    "export-null-not-zero",
  );
  await reset("/tariffs");
  await select("namespace-select", A);
  await click("Настроить тариф");
  await select("project-price-mode", "custom_rates");
  await fill("project-price-input", "3");
  await fill("project-price-output", "10");
  await click("Сохранить черновик тарифа");
  expect(
    (await text()).includes("Черновик") && (await text()).includes("12 USD"),
    "tariff-save-not-activation",
  );
  await click("Активировать");
  await click("Подтвердить активацию");
  expect((await text()).includes("13 USD"), "tariff-explicit-activation");
  expect(
    (await text()).includes("default-20-v1") &&
      (await text()).includes("12 USD"),
    "tariff-history-original-snapshot",
  );
  await select("tariff-profile", "app-test");
  expect(
    (await text()).includes("20% наценки") &&
      !(await text()).includes("13 USD"),
    "tariff-profile-key-independent",
  );
  await select("tariff-currency", "EUR");
  expect((await text()).includes("Неизвестно"), "tariff-no-currency-fallback");
  await reset("/tariffs");
  await select("namespace-select", A);
  await tab.playwright
    .getByRole("tab", { name: "Себестоимость подключений", exact: true })
    .click();
  await observe();
  await tab.playwright
    .locator(
      '[data-action="edit-source-price"][data-source="api-a"][data-model="model-a"][data-currency="USD"]',
    )
    .click();
  await observe();
  await fill("source-price-input", "3");
  await fill("source-price-output", "10");
  await fill("source-price-from", "2026-10-10T00:00");
  await fill("source-price-to", "2026-10-11T00:00");
  await click("Сохранить новую версию");
  await tab.playwright
    .getByRole("tab", { name: "Проекты", exact: true })
    .click();
  await observe();
  expect((await text()).includes("12 USD"), "future-source-not-applied-early");
  await fill("tariff-at", "2026-10-10T00:00");
  expect(
    (await text()).includes("15.6 USD"),
    "effective-source-rate-at-boundary",
  );
  await select("tariff-profile", "app-test");
  expect(
    (await text()).includes("12 USD") && !(await text()).includes("15.6 USD"),
    "source-price-model-neighbor",
  );
  await select("tariff-profile", "main-dev");
  await fill("tariff-at", "2026-10-11T00:00");
  expect(
    (await text()).includes("Неизвестно"),
    "expired-source-not-old-rate-fallback",
  );
  await select("tariff-currency", "EUR");
  await tab.playwright
    .getByRole("tab", { name: "Себестоимость подключений", exact: true })
    .click();
  await observe();
  await tab.playwright
    .locator(
      '[data-action="edit-source-price"][data-source="api-a"][data-model="model-a"][data-currency="USD"]',
    )
    .click();
  await observe();
  await select("source-price-currency", "EUR");
  await fill("source-price-input", "1");
  await fill("source-price-output", "4");
  await fill("source-price-from", "2026-10-09T13:30");
  await click("Сохранить новую версию");
  await tab.playwright
    .getByRole("tab", { name: "Проекты", exact: true })
    .click();
  await observe();
  expect((await text()).includes("6 EUR"), "currency-rate-exact-tuple-no-fx");
  return flows;
}
