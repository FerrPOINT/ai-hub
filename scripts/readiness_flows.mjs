/** Readiness regressions through an existing caller-owned Codex IAB tab only. */
import { readFile } from "node:fs/promises";

export async function verifyReadiness(tab, sourceScriptHash) {
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
  const text = () => tab.playwright.locator("#main").innerText();
  const timeline = () =>
    tab.playwright.locator('.table-wrap[aria-label="Версии и активация"]');
  const reset = async (route) => {
    await tab.goto(
      "http://127.0.0.1:53061/design/prototype.html?theme=dark&state=ready&readiness_seed=" +
        route +
        "#" +
        route,
    );
    await observe();
  };
  const closeDraft = async () => {
    await click("Закрыть диалог");
    if (await tab.playwright.locator("#draft-guard[open]").count())
      await click("Отменить изменения");
  };
  const tariff = async (from, input, output) => {
    await click("Настроить тариф");
    await select("project-price-mode", "custom_rates");
    await fill("project-price-input", input);
    await fill("project-price-output", output);
    await fill("project-price-from", from);
    await click("Сохранить черновик тарифа");
    await tab.playwright
      .locator('[data-action="activate-tariff"]:enabled')
      .click();
    await observe();
    await click("Подтвердить активацию");
  };
  const A = "20000000-0000-4000-8000-000000000001",
    B = "20000000-0000-4000-8000-000000000002";
  await reset("/tariffs");
  await select("namespace-select", A);
  await tariff("2026-10-09T13:30", "3", "10");
  expect(
    (await timeline().innerText()).includes("Действует") &&
      (await tab.playwright
        .getByRole("button", { name: "Отменить расписание", exact: true })
        .count()) === 0,
    "active-tariff-not-cancellable",
  );
  await tariff("2026-10-10T00:00", "4", "11");
  expect(
    (await timeline().innerText()).includes("Запланирован") &&
      (await timeline().innerText()).includes("2026-10-10T00:00:00Z"),
    "scheduled-tariff-exact-state-time",
  );
  await select("scene-select", "pending");
  expect(
    !(await tab.playwright
      .getByRole("button", { name: "Отменить расписание", exact: true })
      .isEnabled()),
    "pending-cancellation-no-duplicate-write",
  );
  await select("scene-select", "ready");
  await fill("tariff-at", "2026-10-10T00:00");
  expect(
    (await text()).includes("15 USD"),
    "scheduled-tariff-positive-boundary",
  );
  await select("scene-select", "conflict");
  await click("Отменить расписание");
  await fill("cancel-tariff-reason", "Неверная ставка в будущем");
  await click("Подтвердить отмену");
  expect(
    (await tab.playwright.locator("#dialog-error").innerText()).includes(
      "412",
    ) &&
      (await tab.playwright
        .locator("#cancel-tariff-reason")
        .evaluate((e) => e.value)) === "Неверная ставка в будущем",
    "cancel-cas-preserves-reason",
  );
  await closeDraft();
  await select("scene-select", "ready");
  await click("Отменить расписание");
  await fill("cancel-tariff-reason", "Неверная ставка в будущем");
  await click("Подтвердить отмену");
  expect(
    (await timeline().innerText()).includes("Отменён") &&
      (await timeline().innerText()).includes("demo-user"),
    "scheduled-cancellation-attributed",
  );
  expect(
    (await text()).includes("13 USD") && !(await text()).includes("15 USD"),
    "cancelled-activation-does-not-close-predecessor",
  );
  expect(
    (await text()).includes("default-20-v1") &&
      (await text()).includes("12 USD"),
    "cancellation-keeps-old-charge-snapshot",
  );
  await click("Прочитать отмену");
  const original = await tab.playwright.locator("#modal").innerText();
  expect(
    original.includes("Operation") && original.includes("Policy version"),
    "cancel-durable-witness-visible",
  );
  await click("Закрыть диалог");
  await click("Прочитать отмену");
  expect(
    (await tab.playwright.locator("#modal").innerText()) === original,
    "cancel-readback-original-identity",
  );
  await click("Закрыть диалог");
  // Occupied time remains reserved even after cancellation; a neighbouring time is valid.
  await click("Настроить тариф");
  await select("project-price-mode", "custom_rates");
  await fill("project-price-input", "5");
  await fill("project-price-output", "12");
  await fill("project-price-from", "2026-10-10T00:00");
  await click("Сохранить черновик тарифа");
  await tab.playwright
    .locator('[data-action="activate-tariff"]:enabled')
    .click();
  await observe();
  await click("Подтвердить активацию");
  expect(
    (await tab.playwright.locator("#dialog-error").innerText()).includes(
      "занят",
    ),
    "cancelled-time-cannot-be-reused",
  );
  await fill("tariff-activate-at", "2026-10-10T00:01");
  await click("Подтвердить активацию");
  await fill("tariff-at", "2026-10-10T00:01");
  expect(
    (await text()).includes("17 USD"),
    "cancel-replacement-neighbour-time",
  );
  await select("namespace-select", B);
  expect(
    !(await timeline().innerText()).includes("Неверная ставка"),
    "cancel-witness-namespace-isolation",
  );
  await reset("/tariffs");
  await tab.playwright
    .getByRole("tab", { name: "Себестоимость подключений", exact: true })
    .click();
  await observe();
  expect(
    (
      await tab.playwright
        .locator("#main tbody tr")
        .filter({ hasText: "ChatGPT · собственная подписка · model-a" })
        .innerText()
    ).includes("Не настроена"),
    "unconfigured-source-not-fictional-revision",
  );
  await reset("/clients");
  await tab.playwright
    .getByRole("button", { name: "Доступ", exact: true })
    .first()
    .click();
  await observe();
  await select("client-cost", "cost_unknown_allowed");
  await click("Сохранить");
  await tab.playwright
    .getByRole("button", { name: "Доступ", exact: true })
    .first()
    .click();
  await observe();
  expect(
    (await tab.playwright.locator("#client-cost").evaluate((e) => e.value)) ===
      "cost_unknown_allowed",
    "unknown-cost-explicit-policy-readback",
  );
  await click("Закрыть диалог");
  await reset("/statistics");
  await select("dimension", "model");
  await select("timezone-filter", "UTC");
  await tab.reload();
  await observe();
  expect(
    (await tab.playwright.locator("#dimension").evaluate((e) => e.value)) ===
      "model" &&
      (await tab.playwright
        .locator("#timezone-filter")
        .evaluate((e) => e.value)) === "UTC",
    "dimension-timezone-url-reload",
  );
  await click("Экспорт", tab.playwright.locator(".page-actions"));
  await select("export-format", "json");
  let pending = tab.playwright.waitForEvent("download");
  await click("Скачать пример");
  const json = JSON.parse(await readFile(await (await pending).path(), "utf8"));
  expect(
    json.filters.group_by === "virtual_model" &&
      json.filters.timezone === "UTC",
    "export-canonical-dimension-timezone",
  );
  expect(
    [
      "provider_id",
      "connection_id",
      "actual_model",
      "evaluation_run_id",
      "project_binding",
    ].every((k) => k in json.filters),
    "export-full-filter-contract",
  );
  expect(
    json.payload.buckets.some(
      (b) => b.key === "main-dev" && b.count === 1240,
    ) && json.payload.buckets.every((b) => !b.key.includes("Fleet")),
    "export-model-buckets-match-selection",
  );
  await click("Экспорт", tab.playwright.locator(".page-actions"));
  await select("export-format", "csv");
  pending = tab.playwright.waitForEvent("download");
  await click("Скачать пример");
  const csv = await readFile(await (await pending).path(), "utf8");
  expect(
    csv.includes("filter_echo_json") &&
      csv.includes("virtual_model") &&
      csv.includes("UTC"),
    "csv-retains-canonical-filter-identity",
  );
  await reset("/evaluations");
  await select("namespace-select", B);
  await click("Добавить набор");
  await fill(
    "dataset-content",
    JSON.stringify([
      {
        case_id: "bad-tool",
        input: "demo",
        assertion_kind: "tool_args",
        expected: "{}",
      },
    ]),
  );
  await click("Сохранить версию");
  expect(
    (await tab.playwright.locator("#dialog-error").innerText()).includes(
      "declared",
    ) ||
      (await tab.playwright.locator("#dialog-error").innerText()).includes(
        "объявленный",
      ),
    "tool-assertion-requires-declared-function",
  );
  await fill(
    "dataset-content",
    JSON.stringify([
      {
        case_id: "bad-schema",
        input: "demo",
        assertion_kind: "json_schema",
        expected: JSON.stringify({ $ref: "https://invalid.example/schema" }),
      },
    ]),
  );
  await click("Сохранить версию");
  expect(
    (await tab.playwright.locator("#dialog-error").innerText()).includes(
      "local",
    ),
    "dataset-remote-schema-rejected",
  );
  await fill(
    "dataset-content",
    JSON.stringify([
      {
        case_id: "literal-neighbour",
        input: "demo",
        assertion_kind: "json_schema",
        expected: JSON.stringify({
          const: { $ref: "https://literal.example/value" },
        }),
      },
    ]),
  );
  await click("Сохранить версию");
  expect(
    (await text()).includes("v1 · 1 демонстрационных случаев"),
    "schema-literal-reference-neighbour-accepted",
  );
  return { flows, exported: json, csv };
}
