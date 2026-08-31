import assert from "node:assert/strict";
import test from "node:test";

import { runChecked, runCommand } from "../lib/process.mjs";

test("runCommand captures output and exit status", async () => {
  const result = await runCommand(process.execPath, ["-e", "process.stdout.write('ready')"]);
  assert.equal(result.code, 0);
  assert.equal(result.stdout, "ready");
  assert.equal(result.stderr, "");
  assert.ok(result.durationMs >= 0);
});

test("runChecked returns command failures as values translated to errors", async () => {
  await assert.rejects(
    runChecked(process.execPath, ["-e", "process.stderr.write('broken'); process.exit(7)"]),
    /failed: broken/,
  );
});

test("runCommand terminates timed out children", async () => {
  await assert.rejects(
    runCommand(process.execPath, ["-e", "setInterval(() => {}, 1000)"], { timeoutMs: 20 }),
    /timed out after 20ms/,
  );
});
