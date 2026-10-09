/** Synthetic tariff UI regression, using only a caller-supplied Codex IAB tab. */
export async function verifyTariffs(tab, sourceScriptHash) {
  const results = [];
  const observe = () => tab.getAXState({ emit: false });
  const expect = (ok, name) => {
    if (!ok) throw Error(name);
    results.push({ name, result: "PASS", sourceScriptHash });
  };
  const text = () => tab.playwright.locator("#main").innerText();
  const value = (selector) =>
    tab.playwright.locator(selector).evaluate((e) => e.value);
  const click = async (name) => {
    await tab.playwright.getByRole("button", { name, exact: true }).click();
    await observe();
  };
  const select = async (selector, option) => {
    await tab.playwright.locator(selector).selectOption(option);
    await observe();
  };
  const fill = async (selector, input) => {
    await tab.playwright.locator(selector).fill(input);
    await observe();
  };
  const switchTab = async (name) => {
    await tab.playwright.getByRole("tab", { name, exact: true }).click();
    await observe();
  };
  const priceRow = (label) =>
    tab.playwright.locator("#main tbody tr").filter({
      hasText:
        label === "ChatGPT"
          ? "ChatGPT · собственная подписка · model-a"
          : label + " · model-a",
    });
  await tab.goto(
    "http://127.0.0.1:53061/design/prototype.html?theme=dark&state=ready#/tariffs",
  );
  await observe();
  expect((await text()).includes("12 USD"), "tariff-default-20-percent");
  await switchTab("Себестоимость подключений");
  expect((await tab.url()).includes("tariff_tab=sources"), "tariff-tab-url");
  await tab.reload();
  await observe();
  expect(
    (await text()).includes("ChatGPT · собственная подписка"),
    "tariff-tab-reload",
  );
  await tab.back();
  await observe();
  expect((await text()).includes("Тарифы проектов"), "tariff-tab-back");
  await tab.forward();
  await observe();
  expect(
    (await text()).includes("Себестоимость подключений · установка"),
    "tariff-tab-forward",
  );
  await priceRow("ChatGPT")
    .getByRole("button", { name: "Настроить цену", exact: true })
    .click();
  await observe();
  expect(
    (await tab.playwright.locator("#source-price-mode option").count()) === 1,
    "subscription-manual-only",
  );
  expect(
    (await value("#source-price-input")) === "",
    "subscription-missing-price-not-zero",
  );
  await fill("#source-price-input", "2");
  await fill("#source-price-output", "8");
  await click("Сохранить новую версию");
  expect(
    (await priceRow("ChatGPT").innerText()).includes("r1"),
    "subscription-manual-revision",
  );
  await priceRow("OpenRouter")
    .getByRole("button", { name: "Настроить цену", exact: true })
    .click();
  await observe();
  expect(
    (await value("#source-price-mode")) === "provider_auto",
    "openrouter-default-auto",
  );
  await select("#source-price-mode", "manual");
  await fill("#source-price-input", "3");
  await fill("#source-price-output", "10");
  await click("Сохранить новую версию");
  expect(
    (await priceRow("OpenRouter").innerText()).includes("Введено вручную"),
    "openrouter-manual-switch",
  );
  await switchTab("Проекты");
  expect(
    (await text()).includes("12 USD") && !(await text()).includes("15.6 USD"),
    "source-model-price-isolation",
  );
  await switchTab("Себестоимость подключений");
  await priceRow("OpenRouter")
    .getByRole("button", { name: "Настроить цену", exact: true })
    .click();
  await observe();
  await select("#source-price-mode", "provider_auto");
  await fill("#source-price-from", "2026-10-09T13:31");
  await click("Сохранить новую версию");
  expect(
    (await priceRow("OpenRouter").innerText()).includes("Автоматически"),
    "openrouter-auto-restore",
  );
  await switchTab("Проекты");
  await select("#namespace-select", "20000000-0000-4000-8000-000000000001");
  await click("Настроить тариф");
  await select("#project-price-mode", "custom_rates");
  await fill("#project-price-input", "0.0000000000001");
  await fill("#project-price-output", "10");
  await click("Сохранить черновик тарифа");
  expect(
    (await tab.playwright.locator("#dialog-error").innerText()).length > 0,
    "tariff-rate-scale-rejected",
  );
  expect(
    (await value("#project-price-input")) === "0.0000000000001",
    "tariff-invalid-keeps-draft",
  );
  await fill("#project-price-input", "3");
  await click("Сохранить черновик тарифа");
  expect((await text()).includes("12 USD"), "tariff-draft-not-active");
  await click("Активировать");
  await click("Подтвердить активацию");
  expect((await text()).includes("13 USD"), "tariff-custom-per-million");
  expect(
    (await text()).includes("default-20-v1") &&
      (await text()).includes("12 USD"),
    "tariff-history-immutable",
  );
  await select("#namespace-select", "20000000-0000-4000-8000-000000000002");
  expect(
    (await text()).includes("20% наценки") &&
      !(await text()).includes("13 USD"),
    "tariff-namespace-independent",
  );
  await select("#namespace-select", "20000000-0000-4000-8000-000000000001");
  expect((await text()).includes("13 USD"), "tariff-namespace-readback");
  await click("Настроить тариф");
  await select("#project-price-mode", "default_markup");
  await fill("#project-price-bps", "25.01");
  await fill("#project-price-from", "2026-10-09T13:31");
  await click("Сохранить черновик тарифа");
  await tab.playwright
    .locator('[data-action="activate-tariff"]:enabled')
    .click();
  await observe();
  await click("Подтвердить активацию");
  expect((await text()).includes("13 USD"), "tariff-scheduled-not-early");
  await fill("#tariff-at", "2026-10-09T13:31");
  expect((await text()).includes("12.501 USD"), "tariff-decimal-percent-exact");
  await tab.playwright
    .getByRole("tab", { name: "Проекты", exact: true })
    .press("ArrowRight");
  await observe();
  expect(
    (await tab.url()).includes("tariff_tab=sources"),
    "tariff-keyboard-tabs",
  );
  await switchTab("Проекты");
  await select("#scene-select", "conflict");
  await click("Настроить тариф");
  await fill("#project-price-bps", "30");
  await click("Сохранить черновик тарифа");
  expect(
    (await tab.playwright.locator("#dialog-error").innerText()).includes("412"),
    "tariff-cas-conflict",
  );
  expect(
    (await value("#project-price-bps")) === "30",
    "tariff-cas-keeps-draft",
  );
  await click("Закрыть диалог");
  await click("Отменить изменения");
  await select("#scene-select", "ready");
  await select("#namespace-select", "unbound");
  expect(
    (await text()).includes("наценка установки 20%") &&
      (await tab.playwright
        .getByRole("button", { name: "Настроить тариф", exact: true })
        .count()) === 0,
    "unbound-no-project-tariff",
  );
  return results;
}
