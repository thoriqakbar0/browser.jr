import assert from "node:assert/strict";
import test from "node:test";

import { startFixtureServer } from "../lib/fixture-server.mjs";

test("fixture server exposes the actionability probe page", async () => {
  const fixture = await startFixtureServer();
  try {
    const response = await fetch(`${fixture.baseUrl}/actionability.html`);
    const body = await response.text();

    assert.equal(response.status, 200);
    assert.match(body, /id="moving"/);
    assert.match(body, /window\.actionabilityProbe/);
  } finally {
    await fixture.close();
  }
});
