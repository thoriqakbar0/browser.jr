import { join } from "node:path";

import { assertIncludes } from "../assertions.mjs";
import { LineProcess } from "../line-process.mjs";
import { runChecked } from "../process.mjs";

const INDEX_REFS = Object.freeze({
  continue: "@e2",
  name: "@e3",
  terms: "@e4",
  color: "@e5",
  save: "@e6",
});

export class BrowserJrAdapter {
  id = "browser-jr";
  label = "browser.jr CLI";
  driver = "browser.jr session stdin";
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
  #process;
  #repoRoot;

  constructor(repoRoot) {
    this.#repoRoot = repoRoot;
    this.#binary = join(repoRoot, "target", "release", "browser-jr");
  }

  async prepare(baseUrl) {
    this.#baseUrl = baseUrl;
    await runChecked("cargo", ["build", "--release", "--bin", "browser-jr"], {
      cwd: this.#repoRoot,
      timeoutMs: 120_000,
    });
    const version = await runChecked(this.#binary, ["--version"]);
    this.#process = new LineProcess(this.#binary, ["--allow-loopback", "session"], {
      cwd: this.#repoRoot,
    });
    const ready = await this.#process.readLine();
    assertIncludes(ready, "session ready", "browser.jr startup");
    return { version: version.stdout.trim() };
  }

  supports(capability) {
    return this.capabilities[capability] === true;
  }

  async setupScenario(scenario) {
    if (scenario === "navigate" || scenario === "full-workflow") return;
    await this.#openIndex();
    if (scenario === "fill" || scenario === "click") {
      await this.#snapshotIndex();
    }
  }

  async executeScenario(scenario) {
    const startedAt = performance.now();
    let lines;
    let snapshotBytes = null;
    switch (scenario) {
      case "navigate":
        lines = [...(await this.#openIndex()), ...(await this.#command("get title", "title="))];
        break;
      case "snapshot":
        lines = await this.#snapshotIndex();
        snapshotBytes = Buffer.byteLength(lines.join("\n"));
        break;
      case "fill":
        lines = await this.#command(`fill ${INDEX_REFS.name} Benchmark User`, "filled ref=");
        break;
      case "click":
        lines = await this.#command(`click ${INDEX_REFS.continue}`, "navigated ref=");
        break;
      case "agent-loop": {
        const firstSnapshot = await this.#snapshotIndex();
        const click = await this.#command(`click ${INDEX_REFS.continue}`, "navigated ref=");
        const secondSnapshot = await this.#snapshotNext();
        lines = [
          ...firstSnapshot,
          ...click,
          ...secondSnapshot,
        ];
        snapshotBytes = Buffer.byteLength([...firstSnapshot, ...secondSnapshot].join("\n"));
        break;
      }
      case "full-workflow":
        lines = [
          ...(await this.#openIndex()),
          ...(await this.#snapshotIndex()),
          ...(await this.#command(`fill ${INDEX_REFS.name} Benchmark User`, "filled ref=")),
          ...(await this.#command(`check ${INDEX_REFS.terms}`, "set checked ref=")),
          ...(await this.#command(`select ${INDEX_REFS.color} blue`, "selected ref=")),
          ...(await this.#command(`get value ${INDEX_REFS.name}`, "value ref=")),
          ...(await this.#command(`is checked ${INDEX_REFS.terms}`, "checked ref=")),
          ...(await this.#command(`get value ${INDEX_REFS.color}`, "value ref=")),
          ...(await this.#command(`click ${INDEX_REFS.continue}`, "navigated ref=")),
          ...(await this.#command("get title", "title=")),
        ];
        break;
      default:
        throw new Error(`browser.jr does not implement ${scenario}`);
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
        const value = (await this.#command(`get value ${INDEX_REFS.name}`, "value ref=")).join("\n");
        assertIncludes(value, '"Benchmark User"', scenario);
        break;
      }
      case "click": {
        const title = (await this.#command("get title", "title=")).join("\n");
        assertIncludes(title, '"Benchmark destination"', scenario);
        break;
      }
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
        throw new Error(`browser.jr cannot verify ${scenario}`);
    }
  }

  async close() {
    await this.#process?.close();
  }

  #command(command, prefix) {
    return this.#process.sendUntil(command, (line) => line.startsWith(prefix));
  }

  #openIndex() {
    return this.#command(`open ${this.#baseUrl}/index.html`, "opened url=");
  }

  #snapshotIndex() {
    return this.#process.sendUntil(
      "snapshot -i",
      (line) => line.includes(`[ref=${INDEX_REFS.save}]`),
    );
  }

  #snapshotNext() {
    return this.#process.sendUntil("snapshot -i", (line) => line.includes("link \"Back\""));
  }

  #assertIndexSnapshot(output) {
    assertIncludes(output, 'link "Continue"', "snapshot");
    assertIncludes(output, 'textbox "Agent name"', "snapshot");
    assertIncludes(output, 'checkbox "Accept terms"', "snapshot");
    assertIncludes(output, 'combobox "Color"', "snapshot");
  }
}
