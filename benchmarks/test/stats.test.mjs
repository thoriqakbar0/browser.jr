import assert from "node:assert/strict";
import test from "node:test";

import { summarize } from "../lib/stats.mjs";

test("summarize reports stable distribution values", () => {
  assert.deepEqual(summarize([30, 10, 20, 40]), {
    samples: 4,
    minMs: 10,
    medianMs: 20,
    p95Ms: 40,
    maxMs: 40,
    meanMs: 25,
    stddevMs: Math.sqrt(125),
  });
});

test("summarize rejects empty and invalid samples", () => {
  assert.throws(() => summarize([]), /at least one/);
  assert.throws(() => summarize([1, Number.NaN]), /finite non-negative/);
  assert.throws(() => summarize([-1]), /finite non-negative/);
});
