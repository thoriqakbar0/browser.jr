import { assertEqual, assertIncludes } from "../assertions.mjs";

export class PlaywrightAdapter {
  driver = "Playwright";
  capabilities = Object.freeze({
    navigation: true,
    interactiveSnapshot: true,
    fill: true,
    click: true,
    javascript: true,
    screenshot: true,
    agentLoop: true,
    fullWorkflow: true,
  });

  #baseUrl;
  #browser;
  #browserType;
  #launchOptions;
  #page;

  constructor({ id, label, browserType, launchOptions = {} }) {
    this.id = id;
    this.label = label;
    this.#browserType = browserType;
    this.#launchOptions = launchOptions;
  }

  async prepare(baseUrl) {
    this.#baseUrl = baseUrl;
    this.#browser = await this.#browserType.launch({ headless: true, ...this.#launchOptions });
    this.#page = await this.#browser.newPage({ viewport: { width: 1280, height: 720 } });
    this.#page.setDefaultTimeout(5_000);
    this.#page.setDefaultNavigationTimeout(5_000);
    return { version: this.#browser.version() };
  }

  supports(capability) {
    return this.capabilities[capability] === true;
  }

  async setupScenario(scenario) {
    if (scenario === "navigate" || scenario === "full-workflow") {
      await this.#page.goto("about:blank");
      return;
    }
    await this.#page.goto(`${this.#baseUrl}/index.html`, { waitUntil: "load" });
  }

  async executeScenario(scenario) {
    const startedAt = performance.now();
    let value = null;
    switch (scenario) {
      case "navigate":
        await this.#page.goto(`${this.#baseUrl}/index.html`, { waitUntil: "load" });
        value = await this.#page.title();
        break;
      case "snapshot":
        value = await this.#snapshot();
        break;
      case "fill":
        await this.#page.getByLabel("Agent name", { exact: true }).fill("Benchmark User");
        break;
      case "click":
        await this.#page.getByRole("link", { name: "Continue", exact: true }).click();
        break;
      case "evaluate":
        value = await this.#page.evaluate(() => `${document.title} ${document.querySelectorAll("li").length}`);
        break;
      case "screenshot":
        value = await this.#page.screenshot({ type: "png" });
        break;
      case "agent-loop":
        value = [
          await this.#snapshot(),
          await this.#page.getByRole("link", { name: "Continue", exact: true }).click(),
          await this.#snapshot(),
        ];
        break;
      case "full-workflow":
        await this.#page.goto(`${this.#baseUrl}/index.html`, { waitUntil: "load" });
        value = { firstSnapshot: await this.#snapshot() };
        await this.#page.getByLabel("Agent name", { exact: true }).fill("Benchmark User");
        await this.#page.getByRole("checkbox", { name: "Accept terms", exact: true }).check();
        await this.#page.getByLabel("Color", { exact: true }).selectOption("blue");
        value.name = await this.#page.locator("#agent-name").inputValue();
        value.checked = await this.#page.locator("#terms").isChecked();
        value.color = await this.#page.locator("#color").inputValue();
        await this.#page.getByRole("link", { name: "Continue", exact: true }).click();
        value.title = await this.#page.title();
        break;
      default:
        throw new Error(`${this.id} does not implement ${scenario}`);
    }
    const durationMs = performance.now() - startedAt;
    const serialized = Buffer.isBuffer(value) ? value : Buffer.from(JSON.stringify(value ?? ""));
    return {
      durationMs,
      value,
      snapshotBytes: scenario === "snapshot" || scenario === "agent-loop"
        ? serialized.byteLength
        : null,
    };
  }

  async verifyScenario(scenario, sample) {
    switch (scenario) {
      case "navigate":
        assertEqual(sample.value, "Browser control benchmark", scenario);
        break;
      case "snapshot":
        this.#assertIndexSnapshot(sample.value);
        break;
      case "fill":
        assertEqual(await this.#page.locator("#agent-name").inputValue(), "Benchmark User", scenario);
        break;
      case "click":
        assertEqual(await this.#page.title(), "Benchmark destination", scenario);
        break;
      case "evaluate":
        assertEqual(sample.value, "Browser control benchmark 3", scenario);
        break;
      case "screenshot":
        if (!Buffer.isBuffer(sample.value) || sample.value.byteLength < 100) {
          throw new Error("screenshot did not return a non-empty PNG");
        }
        break;
      case "agent-loop":
        assertIncludes(sample.value[0], "Continue", scenario);
        assertIncludes(sample.value[2], "Back", scenario);
        break;
      case "full-workflow":
        this.#assertIndexSnapshot(sample.value.firstSnapshot);
        assertEqual(sample.value.name, "Benchmark User", scenario);
        assertEqual(sample.value.checked, true, scenario);
        assertEqual(sample.value.color, "blue", scenario);
        assertEqual(sample.value.title, "Benchmark destination", scenario);
        break;
      default:
        throw new Error(`${this.id} cannot verify ${scenario}`);
    }
  }

  async close() {
    await this.#browser?.close();
  }

  #snapshot() {
    return this.#page.locator("body").ariaSnapshot();
  }

  #assertIndexSnapshot(snapshot) {
    assertIncludes(snapshot, "Continue", "snapshot");
    assertIncludes(snapshot, "Agent name", "snapshot");
    assertIncludes(snapshot, "Accept terms", "snapshot");
    assertIncludes(snapshot, "Color", "snapshot");
  }
}
