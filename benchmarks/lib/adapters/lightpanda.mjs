import { spawn } from "node:child_process";
import { createServer } from "node:net";

import puppeteer from "puppeteer-core";

import { assertEqual, assertIncludes } from "../assertions.mjs";
import { runChecked } from "../process.mjs";

async function reservePort() {
  const server = createServer();
  await new Promise((resolvePromise, rejectPromise) => {
    server.once("error", rejectPromise);
    server.listen(0, "127.0.0.1", resolvePromise);
  });
  const address = server.address();
  if (!address || typeof address === "string") {
    server.close();
    throw new Error("could not reserve a Lightpanda port");
  }
  await new Promise((resolvePromise) => server.close(resolvePromise));
  return address.port;
}

async function waitForCdp(endpoint, processState) {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (processState.error) throw processState.error;
    try {
      const response = await fetch(`${endpoint}/json/version`);
      if (response.ok) return;
    } catch {
      // The server may not have bound its port yet.
    }
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 50));
  }
  throw new Error(`Lightpanda did not expose CDP; stderr: ${processState.stderr}`);
}

export class LightpandaAdapter {
  id = "lightpanda";
  label = "Lightpanda";
  driver = "Puppeteer over CDP";
  capabilities = Object.freeze({
    navigation: true,
    interactiveSnapshot: true,
    fill: true,
    click: true,
    javascript: true,
    screenshot: false,
    agentLoop: true,
    fullWorkflow: true,
  });

  #baseUrl;
  #browser;
  #child;
  #context;
  #exitPromise;
  #page;
  #processState = { error: null, stderr: "" };

  async prepare(baseUrl) {
    this.#baseUrl = baseUrl;
    const version = (await runChecked("lightpanda", ["version"])).stdout.trim();
    const port = await reservePort();
    this.#child = spawn("lightpanda", ["serve", "--port", String(port), "--log-level", "error"], {
      stdio: ["ignore", "ignore", "pipe"],
    });
    this.#child.stderr.setEncoding("utf8");
    this.#child.stderr.on("data", (chunk) => {
      this.#processState.stderr += chunk;
    });
    this.#child.once("error", (error) => {
      this.#processState.error = error;
    });
    this.#exitPromise = new Promise((resolvePromise) => {
      this.#child.once("close", (code, signal) => {
        if (code !== 0 && !this.#processState.error) {
          this.#processState.error = new Error(
            `Lightpanda closed with code ${code} and signal ${signal}`,
          );
        }
        resolvePromise();
      });
    });

    const endpoint = `http://127.0.0.1:${port}`;
    await waitForCdp(endpoint, this.#processState);
    this.#browser = await puppeteer.connect({ browserURL: endpoint });
    this.#context = await this.#browser.createBrowserContext();
    this.#page = await this.#context.newPage();
    this.#page.setDefaultTimeout(5_000);
    this.#page.setDefaultNavigationTimeout(5_000);
    return { version };
  }

  supports(capability) {
    return this.capabilities[capability] === true;
  }

  async setupScenario(scenario) {
    if (scenario === "navigate" || scenario === "full-workflow") return;
    await this.#page.goto(`${this.#baseUrl}/index.html`);
  }

  async executeScenario(scenario) {
    const startedAt = performance.now();
    let value = null;
    switch (scenario) {
      case "navigate":
        await this.#page.goto(`${this.#baseUrl}/index.html`);
        value = await this.#page.title();
        break;
      case "snapshot":
        value = await this.#snapshot();
        break;
      case "fill":
        await this.#page.locator("#agent-name").fill("Benchmark User");
        break;
      case "click":
        await this.#clickContinue();
        break;
      case "evaluate":
        value = await this.#page.evaluate(
          () => `${document.title} ${document.querySelectorAll("li").length}`,
        );
        break;
      case "agent-loop":
        value = [await this.#snapshot()];
        await this.#clickContinue();
        value.push(await this.#snapshot());
        break;
      case "full-workflow":
        await this.#page.goto(`${this.#baseUrl}/index.html`);
        value = { firstSnapshot: await this.#snapshot() };
        await this.#page.locator("#agent-name").fill("Benchmark User");
        await this.#page.click("#terms");
        await this.#page.select("#color", "blue");
        value.name = await this.#page.$eval("#agent-name", (element) => element.value);
        value.checked = await this.#page.$eval("#terms", (element) => element.checked);
        value.color = await this.#page.$eval("#color", (element) => element.value);
        await this.#clickContinue();
        value.title = await this.#page.title();
        break;
      default:
        throw new Error(`Lightpanda does not implement ${scenario}`);
    }
    const serialized = Buffer.from(JSON.stringify(value ?? ""));
    return {
      durationMs: performance.now() - startedAt,
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
        assertEqual(
          await this.#page.$eval("#agent-name", (element) => element.value),
          "Benchmark User",
          scenario,
        );
        break;
      case "click":
        assertEqual(await this.#page.title(), "Benchmark destination", scenario);
        break;
      case "evaluate":
        assertEqual(sample.value, "Browser control benchmark 3", scenario);
        break;
      case "agent-loop":
        assertIncludes(JSON.stringify(sample.value[0]), "Continue", scenario);
        assertIncludes(JSON.stringify(sample.value[1]), "Back", scenario);
        break;
      case "full-workflow":
        this.#assertIndexSnapshot(sample.value.firstSnapshot);
        assertEqual(sample.value.name, "Benchmark User", scenario);
        assertEqual(sample.value.checked, true, scenario);
        assertEqual(sample.value.color, "blue", scenario);
        assertEqual(sample.value.title, "Benchmark destination", scenario);
        break;
      default:
        throw new Error(`Lightpanda cannot verify ${scenario}`);
    }
  }

  async close() {
    try {
      await this.#page?.close();
      await this.#context?.close();
      await this.#browser?.disconnect();
    } finally {
      if (this.#child && this.#child.exitCode === null) {
        this.#child.kill("SIGTERM");
        await this.#exitPromise;
      }
    }
  }

  async #snapshot() {
    const client = await this.#page.createCDPSession();
    try {
      const result = await client.send("Accessibility.getFullAXTree", {});
      return result.nodes;
    } finally {
      await client.detach();
    }
  }

  async #clickContinue() {
    await this.#page.click("#continue");
    await this.#page.waitForFunction(() => document.title === "Benchmark destination");
  }

  #assertIndexSnapshot(snapshot) {
    const serialized = JSON.stringify(snapshot);
    assertIncludes(serialized, "Continue", "snapshot");
    assertIncludes(serialized, "Agent name", "snapshot");
    assertIncludes(serialized, "Accept terms", "snapshot");
    assertIncludes(serialized, "Color", "snapshot");
  }
}
