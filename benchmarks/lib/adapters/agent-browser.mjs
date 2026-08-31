import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { assertIncludes } from "../assertions.mjs";
import { runChecked, runCommand } from "../process.mjs";

export class AgentBrowserAdapter {
  driver = "agent-browser CLI";

  #baseUrl;
  #engine;
  #namespace = "browser-jr-bench";
  #session;

  constructor(engine) {
    this.#engine = engine;
    this.id = `agent-browser-${engine}`;
    this.label = `agent-browser with ${engine}`;
    this.#session = `browser-jr-bench-${engine}-${process.pid}`;
    this.capabilities = Object.freeze({
      navigation: true,
      interactiveSnapshot: true,
      fill: true,
      click: true,
      javascript: true,
      screenshot: engine === "chrome",
      agentLoop: true,
      fullWorkflow: true,
    });
  }

  async prepare(baseUrl) {
    this.#baseUrl = baseUrl;
    const version = (await runChecked("agent-browser", ["--version"])).stdout.trim();
    await this.#run(["open", "about:blank"]);
    return { version };
  }

  supports(capability) {
    return this.capabilities[capability] === true;
  }

  async setupScenario(scenario) {
    if (scenario === "navigate" || scenario === "full-workflow") return;
    await this.#run(["open", `${this.#baseUrl}/index.html`]);
  }

  async executeScenario(scenario) {
    const startedAt = performance.now();
    let output = "";
    let snapshotBytes = null;
    switch (scenario) {
      case "navigate":
        output += (await this.#run(["open", `${this.#baseUrl}/index.html`])).stdout;
        output += (await this.#run(["get", "title"])).stdout;
        break;
      case "snapshot":
        output = (await this.#run(["snapshot", "-i"])).stdout;
        snapshotBytes = Buffer.byteLength(output);
        break;
      case "fill":
        output = (await this.#run(["fill", "#agent-name", "Benchmark User"])).stdout;
        break;
      case "click":
        output = (await this.#run(["click", "#continue"])).stdout;
        break;
      case "evaluate":
        output = (await this.#run([
          "eval",
          "document.title + ' ' + document.querySelectorAll('li').length",
        ])).stdout;
        break;
      case "screenshot": {
        const directory = await mkdtemp(join(tmpdir(), "browser-jr-bench-"));
        const path = join(directory, "viewport.png");
        try {
          output = (await this.#run(["screenshot", path])).stdout;
          const image = await readFile(path);
          output += `\nscreenshot-bytes=${image.byteLength}`;
        } finally {
          await rm(directory, { recursive: true, force: true });
        }
        break;
      }
      case "agent-loop": {
        const firstSnapshot = (await this.#run(["snapshot", "-i"])).stdout;
        output += firstSnapshot;
        output += (await this.#run(["click", "#continue"])).stdout;
        const secondSnapshot = (await this.#run(["snapshot", "-i"])).stdout;
        output += secondSnapshot;
        snapshotBytes = Buffer.byteLength(firstSnapshot) + Buffer.byteLength(secondSnapshot);
        break;
      }
      case "full-workflow":
        output += (await this.#run(["open", `${this.#baseUrl}/index.html`])).stdout;
        output += (await this.#run(["snapshot", "-i"])).stdout;
        output += (await this.#run(["fill", "#agent-name", "Benchmark User"])).stdout;
        output += (await this.#run(["check", "#terms"])).stdout;
        output += (await this.#run(["select", "#color", "blue"])).stdout;
        output += (await this.#run(["get", "value", "#agent-name"])).stdout;
        output += (await this.#run(["is", "checked", "#terms"])).stdout;
        output += (await this.#run(["get", "value", "#color"])).stdout;
        output += (await this.#run(["click", "#continue"])).stdout;
        output += (await this.#run(["get", "title"])).stdout;
        break;
      default:
        throw new Error(`${this.id} does not implement ${scenario}`);
    }
    return {
      durationMs: performance.now() - startedAt,
      output,
      snapshotBytes,
    };
  }

  async verifyScenario(scenario, sample) {
    switch (scenario) {
      case "navigate":
        assertIncludes(sample.output, "Browser control benchmark", scenario);
        break;
      case "snapshot":
        this.#assertIndexSnapshot(sample.output);
        break;
      case "fill": {
        const value = await this.#run(["get", "value", "#agent-name"]);
        assertIncludes(value.stdout, "Benchmark User", scenario);
        break;
      }
      case "click": {
        const title = await this.#run(["get", "title"]);
        assertIncludes(title.stdout, "Benchmark destination", scenario);
        break;
      }
      case "evaluate":
        assertIncludes(sample.output, "Browser control benchmark 3", scenario);
        break;
      case "screenshot":
        assertIncludes(sample.output, "screenshot-bytes=", scenario);
        break;
      case "agent-loop":
        assertIncludes(sample.output, "Continue", scenario);
        assertIncludes(sample.output, "Back", scenario);
        break;
      case "full-workflow":
        this.#assertIndexSnapshot(sample.output);
        assertIncludes(sample.output, "Benchmark User", scenario);
        assertIncludes(sample.output, "true", scenario);
        assertIncludes(sample.output, "blue", scenario);
        assertIncludes(sample.output, "Benchmark destination", scenario);
        break;
      default:
        throw new Error(`${this.id} cannot verify ${scenario}`);
    }
  }

  async close() {
    await runCommand("agent-browser", [...this.#prefix(), "close"], { timeoutMs: 15_000 });
  }

  #prefix() {
    return [
      "--namespace",
      this.#namespace,
      "--session",
      this.#session,
      "--engine",
      this.#engine,
      "--json",
    ];
  }

  #run(args) {
    return runChecked("agent-browser", [...this.#prefix(), ...args], { timeoutMs: 30_000 });
  }

  #assertIndexSnapshot(output) {
    assertIncludes(output, "Continue", "snapshot");
    assertIncludes(output, "Agent name", "snapshot");
    assertIncludes(output, "Accept terms", "snapshot");
    assertIncludes(output, "Color", "snapshot");
  }
}
