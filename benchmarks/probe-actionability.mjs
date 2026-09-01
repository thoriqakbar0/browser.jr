import { chromium, firefox, webkit } from "playwright";

import { findChromeExecutable } from "./lib/adapters/index.mjs";
import { startFixtureServer } from "./lib/fixture-server.mjs";
import { runChecked, runCommand } from "./lib/process.mjs";

const ACTION_TIMEOUT_MS = 1_000;

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function eventRecords(value) {
  return value.records.map(({ type, target, relatedTarget, bubbles, composed }) => ({
    type,
    target,
    relatedTarget,
    bubbles,
    composed,
  }));
}

async function probePlaywright(profile, baseUrl) {
  const browser = await profile.browserType.launch({
    headless: true,
    ...profile.launchOptions,
  });
  try {
    const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
    page.setDefaultTimeout(ACTION_TIMEOUT_MS);
    await page.goto(`${baseUrl}/actionability.html`, { waitUntil: "load" });

    await page.evaluate(() => window.actionabilityProbe.reset());
    await page.locator("#static").click();
    const staticClick = await page.evaluate(() => window.actionabilityProbe.read());

    await page.mouse.move(700, 500);
    await page.evaluate(() => window.actionabilityProbe.reset());
    await page.locator("#static").hover();
    await page.evaluate(() => window.actionabilityProbe.clear());
    await page.locator("#other").hover();
    const hoverTransition = await page.evaluate(() => window.actionabilityProbe.read());

    await page.evaluate(() => window.actionabilityProbe.reset());
    const startedAt = performance.now();
    let movingClick;
    try {
      await page.locator("#moving").click({ timeout: ACTION_TIMEOUT_MS });
      movingClick = { status: "clicked" };
    } catch (error) {
      movingClick = {
        status: error?.name === "TimeoutError" ? "timed-out" : "failed",
        error: error?.name ?? errorMessage(error),
      };
    }
    movingClick.durationMs = Math.round((performance.now() - startedAt) * 100) / 100;
    movingClick.state = await page.evaluate(() => window.actionabilityProbe.read());

    return {
      id: profile.id,
      controller: "Playwright 1.62.1",
      browserVersion: browser.version(),
      staticClick: {
        clickCount: staticClick.clickCount,
        events: eventRecords(staticClick),
      },
      hoverTransition: {
        events: eventRecords(hoverTransition),
      },
      movingClick,
    };
  } finally {
    await browser.close();
  }
}

function agentBrowserPrefix(namespace, session) {
  return [
    "--namespace",
    namespace,
    "--session",
    session,
    "--engine",
    "lightpanda",
    "--json",
  ];
}

function agentBrowserResult(result) {
  const envelope = JSON.parse(result.stdout);
  if (!envelope.success) {
    throw new Error(envelope.error ?? "agent-browser returned an unknown error");
  }
  return envelope.data?.result;
}

async function probeAgentBrowser(baseUrl) {
  const namespace = `browser-jr-actionability-${process.pid}`;
  const session = "lightpanda";
  const prefix = agentBrowserPrefix(namespace, session);
  const environment = {
    ...process.env,
    AGENT_BROWSER_DEFAULT_TIMEOUT: String(ACTION_TIMEOUT_MS),
  };
  const run = (args, options = {}) =>
    runCommand("agent-browser", [...prefix, ...args], {
      env: environment,
      timeoutMs: options.timeoutMs ?? 5_000,
    });
  const runSuccess = async (args) => {
    const result = await run(args);
    if (result.code !== 0) {
      throw new Error(result.stderr.trim() || result.stdout.trim() || `exit ${result.code}`);
    }
    return result;
  };

  try {
    const version = (await runChecked("agent-browser", ["--version"])).stdout.trim();
    await runSuccess(["open", `${baseUrl}/actionability.html`]);
    await runSuccess(["eval", "actionabilityProbe.reset()"]);
    await runSuccess(["click", "#static"]);
    const staticClick = agentBrowserResult(
      await runSuccess(["eval", "actionabilityProbe.read()"]),
    );

    await runSuccess(["eval", "actionabilityProbe.reset()"]);
    await runSuccess(["hover", "#static"]);
    await runSuccess(["eval", "actionabilityProbe.clear()"]);
    await runSuccess(["hover", "#other"]);
    const hoverTransition = agentBrowserResult(
      await runSuccess(["eval", "actionabilityProbe.read()"]),
    );

    await runSuccess(["eval", "actionabilityProbe.reset()"]);
    const movingResult = await run(["click", "#moving"]);
    const movingState = agentBrowserResult(
      await runSuccess(["eval", "actionabilityProbe.read()"]),
    );

    return {
      id: "agent-browser-lightpanda",
      controller: version,
      browserVersion: "Lightpanda through agent-browser",
      staticClick: {
        clickCount: staticClick.clickCount,
        events: eventRecords(staticClick),
      },
      hoverTransition: {
        events: eventRecords(hoverTransition),
      },
      movingClick: {
        status: movingResult.code === 0 ? "clicked" : "failed",
        durationMs: Math.round(movingResult.durationMs * 100) / 100,
        error: movingResult.code === 0
          ? null
          : movingResult.stderr.trim() || movingResult.stdout.trim(),
        state: movingState,
      },
    };
  } finally {
    await runCommand("agent-browser", [...prefix, "close"], {
      env: environment,
      timeoutMs: 5_000,
    });
  }
}

async function main() {
  const chromeExecutable = await findChromeExecutable();
  if (!chromeExecutable) {
    throw new Error("Chrome was not found; set BROWSER_JR_BENCH_CHROME_PATH");
  }
  const fixture = await startFixtureServer();
  try {
    const profiles = [
      {
        id: "chrome",
        browserType: chromium,
        launchOptions: { executablePath: chromeExecutable },
      },
      { id: "firefox", browserType: firefox },
      { id: "webkit", browserType: webkit },
    ];
    const results = [];
    for (const profile of profiles) {
      results.push(await probePlaywright(profile, fixture.baseUrl));
    }
    results.push(await probeAgentBrowser(fixture.baseUrl));
    console.log(JSON.stringify({ actionTimeoutMs: ACTION_TIMEOUT_MS, results }, null, 2));
  } finally {
    await fixture.close();
  }
}

try {
  await main();
} catch (error) {
  console.error(`actionability probe failed: ${errorMessage(error)}`);
  process.exitCode = 1;
}
