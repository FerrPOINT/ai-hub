/** Run only with an existing Codex IAB tab. No browser launch or provider transport. */
export async function verifyDesign(tab, sourceScriptHash) {
  const flows = [];
  const observe = () => tab.getAXState({ emit: false });
  const assert = (condition, name, details) => {
    if (!condition) throw new Error(name + ": " + JSON.stringify(details));
    flows.push({ name, result: "PASS", details, sourceScriptHash });
  };
  const button = (name, root = tab.playwright) =>
    root.getByRole("button", { name, exact: true });
  const click = async (name, root) => {
    await button(name, root).click();
    await observe();
  };
  const select = async (id, value) => {
    await tab.playwright.locator("#" + id).selectOption(value);
    await observe();
  };
  const fill = async (id, value) => {
    await tab.playwright.locator("#" + id).fill(value);
    await observe();
  };
  const mainText = () => tab.playwright.locator("#main").innerText();
  const modalText = () => tab.playwright.locator("#modal").innerText();
  const value = (id) =>
    tab.playwright.locator("#" + id).evaluate((el) => el.value);
  const rows = () =>
    tab.playwright.locator("#main tbody tr").filter({ visible: true }).count();
  const close = async () => {
    await click("Закрыть диалог");
    if (await tab.playwright.locator("#draft-guard[open]").count())
      await click("Отменить изменения");
  };
  let seed = 0;
  const goto = async (route, extra = "") => {
    await tab.goto(
      "http://127.0.0.1:53061/design/prototype.html?theme=dark&state=ready&qa_seed=" +
        seed +
        extra +
        "#" +
        route,
    );
    await observe();
  };
  const reset = async (route, extra = "") => {
    seed++;
    await goto(route, extra);
  };
  const registry = "10000000-0000-4000-8000-000000000001";
  const platform = "20000000-0000-4000-8000-000000000001";
  const testing = "20000000-0000-4000-8000-000000000002";
  const ns = (id) =>
    "&registry_instance_id=" + registry + "&namespace_id=" + id;
  const nav = async (name) => {
    await tab.playwright
      .locator("#desktop-nav")
      .getByRole("link", { name, exact: true })
      .click();
    await observe();
  };

  await reset("/");
  assert(
    (await tab.playwright.locator("header .brand-name").count()) === 0,
    "single-header-service-name",
    "one service label",
  );
  await click("Сервисы платформы");
  assert(
    !(await button("AI Hub · текущий сервис").isEnabled()),
    "current-service-marked",
    "selected service",
  );
  await close();
  await reset("/providers/api-a");
  await click("Изменить подключение");
  assert(
    (await value("connection-name")) === "OpenAI-compatible",
    "connection-edit-prefill",
    "source record",
  );
  await fill("connection-name", "API <соседний случай>");
  await tab.playwright.locator("#connection-name").press("Enter");
  await observe();
  assert(
    (await tab.url()).endsWith("/providers/api-a"),
    "connection-edit-identity",
    "same ID",
  );
  assert(
    (await mainText()).includes("API <соседний случай>"),
    "connection-edit-visible",
    "literal name preserved",
  );
  await click("Отключить");
  await click("Подтвердить в макете");
  assert(
    (await mainText()).includes("Отключён"),
    "connection-disable-visible",
    "history retained",
  );

  await reset("/requests");
  await select("request-status", "failed");
  await select("namespace-select", testing);
  assert(
    (await rows()) === 0,
    "combined-request-filters",
    "testing + failed empty",
  );
  await select("namespace-select", platform);
  assert(
    (await rows()) === 1,
    "combined-request-filters-neighbor",
    "platform + failed one",
  );
  assert(
    (await tab.playwright.locator("#project-filter").count()) === 0,
    "namespace-no-duplicate-selector",
    "one context selector",
  );
  await tab.back();
  await observe();
  assert(
    (await value("namespace-select")) === testing,
    "namespace-back",
    "testing restored",
  );
  await tab.forward();
  await observe();
  assert(
    (await value("namespace-select")) === platform,
    "namespace-forward",
    "platform restored",
  );
  await tab.reload();
  await observe();
  assert(
    (await value("namespace-select")) === platform,
    "namespace-reload",
    "URL pair preserved",
  );
  await reset(
    "/requests",
    "&registry_instance_id=10000000-0000-4000-8000-000000000099&namespace_id=" +
      platform,
  );
  assert(
    (await mainText()).includes("Неверный контекст") && (await rows()) === 0,
    "namespace-foreign-registry",
    "no all-scope fallback",
  );
  await reset("/requests", ns("20000000-0000-4000-8000-000000000003"));
  assert(
    (await mainText()).includes("Проект архивирован"),
    "namespace-archived",
    "writes closed",
  );
  await reset("/requests", ns("20000000-0000-4000-8000-000000000004"));
  assert(
    (await mainText()).includes("Проект недоступен"),
    "namespace-unavailable",
    "identity retained",
  );
  await reset("/requests", "&binding=unbound");
  assert(
    (await rows()) === 1 && (await mainText()).includes("req_h53"),
    "namespace-unbound",
    "explicit unbound sample",
  );

  await reset("/statistics", ns(testing));
  await select("currency-filter", "EUR");
  assert(
    !(await tab.playwright.locator("#main table").innerText()).includes("$"),
    "statistics-currency",
    "EUR only",
  );
  await select("dimension", "provider");
  assert(
    (await mainText()).includes("По провайдерам"),
    "statistics-dimension",
    "table changes",
  );
  assert(
    (await mainText()).includes("Ollama Cloud"),
    "statistics-project-neighbor",
    "own provider",
  );
  await select("dimension", "day");
  assert((await rows()) === 7, "statistics-day", "seven daily facts");
  await reset("/expenses");
  await select("currency-filter", "EUR");
  assert(
    !(await tab.playwright.locator("#main table").innerText()).includes("$"),
    "expense-ledger-currency",
    "EUR rows",
  );
  await select("namespace-select", testing);
  assert(
    (await tab.playwright.locator("#main table").innerText()).includes(
      "Нет операций",
    ),
    "expense-ledger-scope",
    "known empty",
  );
  await reset("/audit");
  await fill("audit-query", "main-dev");
  await select("audit-action", "publish");
  assert((await rows()) === 1, "audit-combined-filter", "query retained");
  await select("audit-action", "access");
  assert((await rows()) === 0, "audit-filter-neighbor", "neighbor empty");

  await reset("/providers/api-a");
  await click("Контекст модели");
  await fill("context-budget", "100000");
  await select("context-model", "model-b-small");
  assert(
    (await value("context-budget")) === "64000",
    "model-context-neighbor",
    "second model preference",
  );
  await select("context-model", "model-a");
  assert(
    (await value("context-budget")) === "100000",
    "model-context-unsaved-switch",
    "local input retained",
  );
  await click("Сохранить");
  await click("Контекст модели");
  assert(
    (await value("context-budget")) === "100000",
    "model-context-saved",
    "exact preference",
  );
  await close();
  await reset("/providers/openrouter");
  assert(
    (await mainText()).includes("OpenRouter") &&
      (await mainText()).includes("Предел не подтверждён"),
    "openrouter-explicit-unknown-bounds",
    "preset and no 256K physical fallback",
  );
  await reset("/providers/chatgpt");
  await tab.playwright
    .getByRole("tab", { name: "Авторизация", exact: true })
    .click();
  await observe();
  await click("Войти через подписку");
  const firstLogin = await modalText();
  await click("Показать неопределённый исход");
  assert(
    (await modalText()).includes("unknown"),
    "managed-login-unknown",
    "uncertain state",
  );
  await click("Прочитать состояние");
  const loginId = firstLogin.match(/[a-f0-9-]{36}/)?.[0];
  assert(
    loginId && (await modalText()).includes(loginId),
    "managed-login-readback-identity",
    "same operation",
  );
  await close();
  await click("Войти через подписку");
  assert(
    (await modalText()).includes(loginId),
    "managed-login-no-duplicate",
    "no second intent",
  );
  await click("Подтвердить получение результата");
  assert(
    (await mainText()).includes("Авторизован"),
    "managed-login-visible",
    "qualified separately",
  );

  await reset("/budgets");
  const before = await mainText();
  await click("Уведомления");
  await button("Подтвердить").first().click();
  await observe();
  assert(
    (await button("Подтверждено").count()) === 1,
    "notification-ack",
    "actor acknowledgement",
  );
  await close();
  assert(
    (await mainText()) === before,
    "notification-no-financial-effect",
    "same budget/reserve",
  );
  await button("Изменить").first().click();
  await observe();
  assert(
    (await tab.playwright.locator("#budget-target").innerText()).includes(
      "Установка",
    ),
    "budget-edit-prefill",
    "installation",
  );
  await fill("budget-limit", "60,00");
  await click("Сохранить");
  assert(
    (await mainText()).includes("12,03"),
    "budget-edit-decimal",
    "60-47.25-.72",
  );
  await click("Новый бюджет");
  await select("budget-scope", "project");
  assert(
    !(await tab.playwright.locator("#budget-target").innerText()).includes(
      "CI тесты",
    ),
    "budget-scope-target",
    "correct resource kind",
  );
  await fill("warn-first", "96");
  await click("Сохранить");
  assert(
    (await tab.playwright.locator("#dialog-error").innerText()).includes(
      "Первый порог",
    ),
    "budget-threshold-validation",
    "input retained",
  );
  await close();

  await reset("/clients", ns(platform));
  await button("Доступ").first().click();
  await observe();
  assert(
    (await value("client-namespace")) === platform,
    "client-existing-form",
    "exact Namespace",
  );
  await select("client-profile", ["main-dev", "app-test"]);
  await click("Сохранить");
  assert(
    (
      await tab.playwright.locator("#main tbody tr").first().innerText()
    ).includes("main-dev, app-test"),
    "client-multiple-profiles",
    "array saved",
  );
  assert(
    (await tab.playwright.locator("#modal[open]").count()) === 0,
    "client-save-no-key-issue",
    "rights only",
  );
  await button("Доступ").first().click();
  await observe();
  await select("client-namespace", testing);
  await click("Сохранить");
  assert(
    (await tab.playwright.locator("#dialog-error").innerText()).includes(
      "Отзовите",
    ),
    "client-binding-requires-revocation",
    "no key scope expansion",
  );
  await close();
  await button("Ключи").first().click();
  await observe();
  await click("Выдать новый ключ");
  assert(
    (await modalText()).includes("DEMO-NOT-A-REAL-KEY"),
    "explicit-key-issue",
    "synthetic one-time value",
  );
  await close();
  await button("Отключить").first().click();
  await observe();
  assert(
    (
      await tab.playwright.locator("#main tbody tr").first().innerText()
    ).includes("Отключён"),
    "client-disable",
    "new admissions closed",
  );

  await reset("/models/main-dev");
  await button("Изменить").last().click();
  await observe();
  assert(
    !(await tab.playwright.locator("#deployment-model").innerText()).includes(
      "model-a",
    ),
    "deployment-catalog-bound",
    "own connection catalog",
  );
  await select("deployment-model", "model-b-small");
  await click("Сохранить");
  assert(
    (
      await tab.playwright
        .locator("#deployment-list .deployment")
        .last()
        .innerText()
    ).includes("model-b-small"),
    "deployment-edit-selected-row",
    "second row changed",
  );
  assert(
    !(await button("Опубликовать").isEnabled()),
    "changed-route-invalidates-proof",
    "publication disabled",
  );
  await fill("input-limit", "127000");
  await click("Сохранить черновик");
  await click("Проверить черновик");
  assert(
    (await tab.playwright.locator("#modal[open]").count()) === 0,
    "invalid-bounds-block-proof",
    "no proof for invalid/unsaved configuration",
  );
  await fill("input-limit", "40000");
  await fill("output-limit", "4000");
  await fill("context-limit", "64000");
  await click("Сохранить черновик");
  await click("Проверить черновик");
  await click("Подтвердить проверку макета");
  await click("Опубликовать");
  await click("Опубликовать в макете");
  assert(
    (await mainText()).includes("Опубликована r13"),
    "profile-publication",
    "immutable next revision",
  );
  await click("История версий");
  assert(
    (await modalText()).includes("r13") && (await modalText()).includes("r12"),
    "profile-history-current-record",
    "own snapshots",
  );
  await close();
  await nav("Запросы");
  await tab.playwright
    .getByRole("link", { name: "req_a17d", exact: true })
    .click();
  await observe();
  assert(
    (await mainText()).includes("main-dev r12"),
    "request-frozen-revision",
    "old request unchanged",
  );
  await nav("Модели");
  await tab.playwright
    .getByRole("link", { name: "main-dev", exact: true })
    .click();
  await observe();
  await click("История версий");
  await button("Откатить").first().click();
  await observe();
  assert(
    (await mainText()).includes("Опубликована r12"),
    "profile-rollback",
    "explicit active pointer",
  );
  await click("Отключить");
  await click("Подтвердить");
  await click("Архивировать");
  await click("Подтвердить");
  assert(
    !(await button("Сохранить черновик").isEnabled()),
    "profile-archive-readonly",
    "history retained, edits closed",
  );
  await reset("/models/app-test");
  await click("История версий");
  assert(
    (await modalText()).includes("r4"),
    "profile-history-neighbor",
    "own revision",
  );
  await close();
  assert(
    (await tab.playwright.locator("#deployment-list .deployment").count()) ===
      1,
    "pinned-route",
    "one target",
  );

  await reset("/evaluations", ns(testing));
  await click("Добавить набор");
  await fill("dataset-content", "{}");
  await click("Сохранить версию");
  assert(
    (await tab.playwright.locator("#dialog-error").innerText()).includes(
      "JSON-массив",
    ),
    "dataset-validation",
    "invalid JSON shape retained",
  );
  await fill(
    "dataset-content",
    '[{"case_id":"one","input":"demo","assertion_kind":"exact_match","expected":"OK"}]',
  );
  await click("Сохранить версию");
  assert(
    (await mainText()).includes("v1 · 1 случаев"),
    "dataset-visible",
    "version saved",
  );
  await click("Запустить");
  await select("eval-dataset", "dataset-1");
  await click("Запустить в макете");
  assert(
    (await tab.url()).endsWith("/evaluations/run-26"),
    "new-run-identity",
    "new run",
  );
  assert(
    (await mainText()).includes("В очереди"),
    "new-run-no-fake-results",
    "not old results",
  );
  await click("Остановить");
  assert(
    (await mainText()).includes("Отменён"),
    "new-run-cancel",
    "costs retained",
  );
  await nav("Тестирование");
  await click("Запустить");
  await click("Запустить в макете");
  assert(
    (await tab.url()).endsWith("/evaluations/run-27"),
    "new-run-neighbor",
    "second identity",
  );
  await reset("/expenses");
  await tab.playwright
    .getByRole("tab", { name: "Себестоимость", exact: true })
    .click();
  await observe();
  await click("Новый тариф");
  await click("Сохранить");
  assert(
    (await mainText()).includes("Добавлено в этой сессии"),
    "financial-save-visible",
    "no fake receipt",
  );
  await tab.playwright
    .getByRole("tab", { name: "Себестоимость", exact: true })
    .press("ArrowRight");
  await observe();
  assert(
    (await tab.playwright
      .getByRole("tab", { name: "Подписки", exact: true })
      .getAttribute("aria-selected")) === "true",
    "keyboard-tabs",
    "focus/selection",
  );
  await button("Экспорт").first().click();
  await observe();
  await select("export-format", "json");
  const downloaded = tab.playwright.waitForEvent("download");
  await click("Скачать пример");
  const download = await downloaded;
  const fs = await import("node:fs/promises");
  const file = await download.path({ timeoutMs: 10000 });
  const snapshot = JSON.parse(await fs.readFile(file, "utf8"));
  assert(
    snapshot.synthetic === true && snapshot.rows.length > 0,
    "export-downloaded-snapshot",
    { file, rows: snapshot.rows.length },
  );

  await reset("/models/main-dev");
  await fill("display-name", "Несохранённый черновик");
  await select("namespace-select", testing);
  assert(
    (await modalText()).includes("Есть несохранённые изменения"),
    "dirty-navigation",
    "Namespace guard",
  );
  await click("Остаться");
  assert(
    (await value("display-name")) === "Несохранённый черновик",
    "namespace-dirty-stay",
    "input retained",
  );
  await select("scene-select", "conflict");
  assert(
    (await value("display-name")) === "Несохранённый черновик",
    "conflict-preserves-draft",
    "412 retains field",
  );
  await click("Сохранить черновик");
  assert(
    (await tab.playwright.locator("#model-validation").innerText()).includes(
      "Версия изменилась",
    ),
    "conflict-blocks-save",
    "no overwrite",
  );
  await nav("Обзор");
  await click("Выйти без сохранения");
  await reset("/requests/missing");
  assert(
    (await tab.playwright
      .locator('[data-design-state="not_found"]')
      .count()) === 1,
    "missing-entity",
    "404",
  );
  await reset("/requests/b82c");
  assert(
    (await mainText()).includes("app-test r4"),
    "existing-neighbor",
    "valid identity",
  );
  return flows;
}
