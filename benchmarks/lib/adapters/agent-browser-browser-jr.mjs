import { randomBytes } from "node:crypto";
import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { createInterface } from "node:readline";

import { assertIncludes } from "../assertions.mjs";
import { runChecked } from "../process.mjs";

const INDEX_REFS = Object.freeze({
  continue: "@e2",
  name: "@e3",
  terms: "@e4",
  color: "@e5",
  save: "@e6",
});

export class AgentBrowserBrowserJrAdapter {
  id = "agent-browser-browser-jr";
  label = "agent-browser plugin with browser.jr";
  driver = "agent-browser CLI → plugin v1 → browser.jr JSON session";
  capabilities = Object.freeze({
    navigation: true,
    interactiveSnapshot: true,
    fill: true,
    click: true,
    javascript: false,
    screenshot: false,
    agentLoop: true,
    fullWorkflow: true,
  });

  #baseUrl;
  #binary;
  #environment;
  #pluginPath;
  #relay;
  #relayConnection;
  #relayExit;
  #repoRoot;

  constructor(repoRoot) {
    this.#repoRoot = repoRoot;
    this.#binary = join(repoRoot, "target", "release", "browser-jr");
    this.#pluginPath = join(repoRoot, "plugin", "cli.mjs");
  }

  async prepare(baseUrl) {
    this.#baseUrl = baseUrl;
    await runChecked("cargo", ["build", "--release", "--bin", "browser-jr"], {
      cwd: this.#repoRoot,
      timeoutMs: 120_000,
    });
    const browserJrVersion = (await runChecked(this.#binary, ["--version"])).stdout.trim();
    const agentBrowserVersion = (await runChecked("agent-browser", ["--version"])).stdout.trim();
    const pluginPackage = JSON.parse(await readFile(join(this.#repoRoot, "package.json"), "utf8"));
    this.#relayConnection = await this.#startRelay();
    this.#environment = {
      ...process.env,
      AGENT_BROWSER_PLUGINS: JSON.stringify([
        {
          name: "browser-jr",
          command: process.execPath,
          args: [this.#pluginPath],
          capabilities: ["command.run", "browserjr.session", "browserjr.command"],
        },
      ]),
    };
    return {
      agentBrowserVersion,
      browserJrVersion,
      pluginVersion: pluginPackage.version,
    };
  }

  supports(capability) {
    return this.capabilities[capability] === true;
  }

  async setupScenario(scenario) {
    if (scenario === "navigate" || scenario === "full-workflow") return;
    await this.#openIndex();
    if (scenario === "fill" || scenario === "click") await this.#snapshotIndex();
  }

  async executeScenario(scenario) {
    const startedAt = performance.now();
    let lines;
    let snapshotBytes = null;
    switch (scenario) {
      case "navigate":
        lines = [await this.#openIndex(), await this.#command("get title")];
        break;
      case "snapshot":
        lines = [await this.#snapshotIndex()];
        snapshotBytes = Buffer.byteLength(lines.join("\n"));
        break;
      case "fill":
        lines = [await this.#command(`fill ${INDEX_REFS.name} Benchmark User`)];
        break;
      case "click":
        lines = [await this.#command(`click ${INDEX_REFS.continue}`)];
        break;
      case "agent-loop": {
        const firstSnapshot = await this.#snapshotIndex();
        const click = await this.#command(`click ${INDEX_REFS.continue}`);
        const secondSnapshot = await this.#command("snapshot -i");
        lines = [firstSnapshot, click, secondSnapshot];
        snapshotBytes = Buffer.byteLength(`${firstSnapshot}\n${secondSnapshot}`);
        break;
      }
      case "full-workflow":
        lines = [
          await this.#openIndex(),
          await this.#snapshotIndex(),
          await this.#command(`fill ${INDEX_REFS.name} Benchmark User`),
          await this.#command(`check ${INDEX_REFS.terms}`),
          await this.#command(`select ${INDEX_REFS.color} blue`),
          await this.#command(`get value ${INDEX_REFS.name}`),
          await this.#command(`is checked ${INDEX_REFS.terms}`),
          await this.#command(`get value ${INDEX_REFS.color}`),
          await this.#command(`click ${INDEX_REFS.continue}`),
          await this.#command("get title"),
        ];
        break;
      default:
        throw new Error(`${this.id} does not implement ${scenario}`);
    }
    return {
      durationMs: performance.now() - startedAt,
      output: lines.join("\n"),
      snapshotBytes,
    };
  }

  async verifyScenario(scenario, sample) {
    switch (scenario) {
      case "navigate":
        assertIncludes(sample.output, 'title="Browser control benchmark"', scenario);
        break;
      case "snapshot":
        this.#assertIndexSnapshot(sample.output);
        break;
      case "fill": {
        await this.#snapshotIndex();
        assertIncludes(await this.#command(`get value ${INDEX_REFS.name}`), '"Benchmark User"', scenario);
        break;
      }
      case "click":
        assertIncludes(await this.#command("get title"), '"Benchmark destination"', scenario);
        break;
      case "agent-loop":
        assertIncludes(sample.output, 'link "Continue"', scenario);
        assertIncludes(sample.output, 'link "Back"', scenario);
        break;
      case "full-workflow":
        assertIncludes(sample.output, 'value ref=@e3 "Benchmark User"', scenario);
        assertIncludes(sample.output, "checked ref=@e4 value=true", scenario);
        assertIncludes(sample.output, 'value ref=@e5 "blue"', scenario);
        assertIncludes(sample.output, 'title="Benchmark destination"', scenario);
        break;
      default:
        throw new Error(`${this.id} cannot verify ${scenario}`);
    }
  }

  async close() {
    if (!this.#relay || this.#relay.exitCode !== null) return;
    this.#relay.kill("SIGTERM");
    await this.#relayExit;
  }

  async #startRelay() {
    const token = randomBytes(24).toString("hex");
    this.#relay = spawn(
      process.execPath,
      [this.#pluginPath, "serve", "--binary", this.#binary, "--allow-loopback", "--token", token],
      { cwd: this.#repoRoot, stdio: ["ignore", "pipe", "pipe"] },
    );
    let stderr = "";
    this.#relay.stderr.setEncoding("utf8");
    this.#relay.stderr.on("data", (chunk) => { stderr += chunk; });
    this.#relayExit = new Promise((resolvePromise) => this.#relay.once("close", resolvePromise));
    const lines = createInterface({ input: this.#relay.stdout, crlfDelay: Infinity });
    const iterator = lines[Symbol.asyncIterator]();
    const ready = await Promise.race([
      iterator.next(),
      new Promise((_, rejectPromise) =>
        setTimeout(() => rejectPromise(new Error(`browser.jr plugin relay timed out: ${stderr}`)), 10_000),
      ),
    ]);
    if (ready.done) throw new Error(`browser.jr plugin relay closed before ready: ${stderr}`);
    const envelope = JSON.parse(ready.value);
    if (!envelope.success || envelope.result?.event !== "ready") {
      throw new Error(envelope.error ?? "browser.jr plugin relay did not become ready");
    }
    return { host: envelope.result.host, port: envelope.result.port, token };
  }

  async #command(command) {
    const payload = JSON.stringify({ ...this.#relayConnection, command });
    const result = await runChecked(
      "agent-browser",
      ["--json", "plugin", "run", "browser-jr", "browserjr.command", "--payload", payload],
      { env: this.#environment, timeoutMs: 30_000 },
    );
    const envelope = JSON.parse(result.stdout);
    if (!envelope.success) throw new Error(envelope.error ?? "agent-browser plugin command failed");
    return envelope.result?.output ?? "";
  }

  #openIndex() {
    return this.#command(`open ${this.#baseUrl}/index.html`);
  }

  #snapshotIndex() {
    return this.#command("snapshot -i");
  }

  #assertIndexSnapshot(output) {
    assertIncludes(output, 'link "Continue"', "snapshot");
    assertIncludes(output, 'textbox "Agent name"', "snapshot");
    assertIncludes(output, 'checkbox "Accept terms"', "snapshot");
    assertIncludes(output, 'combobox "Color"', "snapshot");
  }
}
