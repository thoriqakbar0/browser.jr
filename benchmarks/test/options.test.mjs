import assert from "node:assert/strict";
import test from "node:test";

import { ADAPTER_IDS, parseOptions } from "../lib/options.mjs";
import { SCENARIO_IDS } from "../lib/scenarios.mjs";

test("parseOptions returns the complete default matrix", () => {
  const options = parseOptions([], "/tmp/bench");
  assert.equal(options.iterations, 10);
  assert.equal(options.warmup, 1);
  assert.deepEqual(options.adapters, ADAPTER_IDS);
  assert.deepEqual(options.scenarios, SCENARIO_IDS);
  assert.equal(options.output, "/tmp/bench/results/latest.json");
});

test("parseOptions accepts focused engine and scenario selections", () => {
  const options = parseOptions(
    [
      "--",
      "--iterations",
      "3",
      "--warmup",
      "0",
      "--engines",
      "browser-jr,webkit",
      "--scenarios",
      "snapshot,full-workflow",
      "--output",
      "results/focused.json",
    ],
    "/tmp/bench",
  );
  assert.equal(options.iterations, 3);
  assert.equal(options.warmup, 0);
  assert.deepEqual(options.adapters, ["browser-jr", "webkit"]);
  assert.deepEqual(options.scenarios, ["snapshot", "full-workflow"]);
  assert.equal(options.output, "/tmp/bench/results/focused.json");
});

test("parseOptions rejects invalid input", () => {
  assert.throws(() => parseOptions(["--iterations", "0"]), /at least one/);
  assert.throws(() => parseOptions(["--warmup", "-1"]), /non-negative/);
  assert.throws(() => parseOptions(["--engines", "unknown"]), /unknown value/);
  assert.throws(() => parseOptions(["--wat"]), /unknown or incomplete/);
});
