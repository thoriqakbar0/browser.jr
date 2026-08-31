import { mkdir, writeFile } from "node:fs/promises";
import { cpus, platform, arch, release, totalmem } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { createAdapters } from "./lib/adapters/index.mjs";
import { startFixtureServer } from "./lib/fixture-server.mjs";
import { ADAPTER_IDS, parseOptions } from "./lib/options.mjs";
import { runCommand } from "./lib/process.mjs";
import { SCENARIOS } from "./lib/scenarios.mjs";
import { summarize } from "./lib/stats.mjs";

const BENCHMARK_ROOT = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(BENCHMARK_ROOT, "..");

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function round(value) {
  return Math.round(value * 100) / 100;
}

function summarizeBytes(samples) {
  const summary = summarize(samples);
  return {
    samples: summary.samples,
    minBytes: summary.minMs,
    medianBytes: summary.medianMs,
    p95Bytes: summary.p95Ms,
    maxBytes: summary.maxMs,
    meanBytes: summary.meanMs,
    stddevBytes: summary.stddevMs,
  };
}

async function gitMetadata() {
  const commit = await runCommand("git", ["rev-parse", "HEAD"], { cwd: REPO_ROOT });
  const status = await runCommand("git", ["status", "--porcelain"], { cwd: REPO_ROOT });
  const changedFiles = status.stdout.split("\n").filter(Boolean).length;
  return {
    commit: commit.code === 0 ? commit.stdout.trim() : null,
    dirty: changedFiles > 0,
    changedFiles,
  };
}

function hostMetadata() {
  const cpuList = cpus();
  return {
    platform: platform(),
    release: release(),
    architecture: arch(),
    cpu: cpuList[0]?.model ?? null,
    logicalCpuCount: cpuList.length,
    totalMemoryBytes: totalmem(),
    node: process.version,
  };
}

async function runScenario(adapter, scenario, options) {
  if (!adapter.supports(scenario.capability)) {
    return {
      id: scenario.id,
      description: scenario.description,
      status: "unsupported",
      reason: `${adapter.label} does not provide ${scenario.capability}`,
    };
  }

  const durations = [];
  const snapshotSizes = [];
  try {
    for (let index = 0; index < options.warmup; index += 1) {
      await adapter.setupScenario(scenario.id);
      const sample = await adapter.executeScenario(scenario.id);
      await adapter.verifyScenario(scenario.id, sample);
    }
    for (let index = 0; index < options.iterations; index += 1) {
      await adapter.setupScenario(scenario.id);
      const sample = await adapter.executeScenario(scenario.id);
      await adapter.verifyScenario(scenario.id, sample);
      durations.push(sample.durationMs);
      if (sample.snapshotBytes !== null) snapshotSizes.push(sample.snapshotBytes);
    }
    return {
      id: scenario.id,
      description: scenario.description,
      status: "passed",
      correctness: "passed",
      durationMs: durations.map(round),
      latency: Object.fromEntries(
        Object.entries(summarize(durations)).map(([key, value]) => [key, round(value)]),
      ),
      snapshotBytes: snapshotSizes.length > 0 ? summarizeBytes(snapshotSizes) : null,
    };
  } catch (error) {
    return {
      id: scenario.id,
      description: scenario.description,
      status: "failed",
      correctness: "failed",
      error: errorMessage(error),
      completedSamples: durations.length,
    };
  }
}

async function runAdapter(factory, adapterId, fixture, scenarios, options) {
  let adapter;
  let result;
  try {
    adapter = factory();
    const runtime = await adapter.prepare(fixture.baseUrl);
    const results = [];
    for (const scenario of scenarios) {
      process.stdout.write(`  ${scenario.id.padEnd(16)}`);
      const result = await runScenario(adapter, scenario, options);
      results.push(result);
      if (result.status === "passed") {
        console.log(`${result.latency.medianMs.toFixed(2)}ms median`);
      } else {
        console.log(result.status);
      }
    }
    result = {
      id: adapter.id,
      label: adapter.label,
      driver: adapter.driver,
      status: results.some(({ status }) => status === "failed") ? "failed" : "passed",
      runtime,
      capabilities: adapter.capabilities,
      scenarios: results,
    };
  } catch (error) {
    console.log(`  unavailable: ${errorMessage(error)}`);
    result = {
      id: adapterId,
      status: "unavailable",
      error: errorMessage(error),
      scenarios: [],
    };
  } finally {
    if (adapter) {
      try {
        await adapter.close();
      } catch (error) {
        const cleanupError = errorMessage(error);
        console.error(`  cleanup failed: ${cleanupError}`);
        result = {
          ...result,
          status: "failed",
          cleanupError,
        };
      }
    }
  }
  return result;
}

function printList() {
  console.log("engines");
  for (const id of ADAPTER_IDS) console.log(`  ${id}`);
  console.log("scenarios");
  for (const scenario of SCENARIOS) console.log(`  ${scenario.id}`);
}

async function main() {
  const options = parseOptions(process.argv.slice(2), BENCHMARK_ROOT);
  if (options.list) {
    printList();
    return;
  }

  const scenarios = SCENARIOS.filter(({ id }) => options.scenarios.includes(id));
  const factories = await createAdapters(REPO_ROOT);
  const fixture = await startFixtureServer();
  const startedAt = new Date();
  const adapters = [];

  console.log(`browser control benchmark: ${options.iterations} measured, ${options.warmup} warmup`);
  try {
    for (const adapterId of options.adapters) {
      console.log(`\n${adapterId}`);
      const result = await runAdapter(
        factories.get(adapterId),
        adapterId,
        fixture,
        scenarios,
        options,
      );
      adapters.push(result);
    }
  } finally {
    await fixture.close();
  }

  const failed = adapters.some(({ status }) => status !== "passed");
  const result = {
    schemaVersion: 1,
    startedAt: startedAt.toISOString(),
    finishedAt: new Date().toISOString(),
    status: failed ? "failed" : "passed",
    repository: await gitMetadata(),
    host: hostMetadata(),
    config: {
      iterations: options.iterations,
      warmup: options.warmup,
      engines: options.adapters,
      scenarios: options.scenarios,
      fixtureOrigin: "loopback with an ephemeral port",
    },
    adapters,
  };

  await mkdir(dirname(options.output), { recursive: true });
  await writeFile(options.output, `${JSON.stringify(result, null, 2)}\n`, "utf8");
  console.log(`\nresult: ${result.status}`);
  console.log(`json: ${options.output}`);
  if (failed) process.exitCode = 1;
}

try {
  await main();
} catch (error) {
  console.error(`benchmark failed: ${errorMessage(error)}`);
  process.exitCode = 1;
}
