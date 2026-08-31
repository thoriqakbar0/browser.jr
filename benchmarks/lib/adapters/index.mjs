import { access, readdir } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";

import { chromium, firefox, webkit } from "playwright";

import { AgentBrowserAdapter } from "./agent-browser.mjs";
import { BrowserJrAdapter } from "./browser-jr.mjs";
import { LightpandaAdapter } from "./lightpanda.mjs";
import { PlaywrightAdapter } from "./playwright.mjs";

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function findChromeExecutable() {
  const explicit = process.env.BROWSER_JR_BENCH_CHROME_PATH;
  if (explicit && (await exists(explicit))) return explicit;

  const system = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
  if (await exists(system)) return system;

  const cache = join(homedir(), ".agent-browser", "browsers");
  let entries = [];
  try {
    entries = await readdir(cache);
  } catch {
    return null;
  }
  const candidates = entries
    .filter((entry) => entry.startsWith("chrome-"))
    .sort()
    .reverse()
    .map((entry) =>
      join(cache, entry, "Google Chrome for Testing.app", "Contents", "MacOS", "Google Chrome for Testing"),
    );
  for (const candidate of candidates) {
    if (await exists(candidate)) return candidate;
  }
  return null;
}

export async function createAdapters(repoRoot) {
  const chromeExecutable = await findChromeExecutable();
  return new Map([
    ["browser-jr", () => new BrowserJrAdapter(repoRoot)],
    [
      "chrome",
      () => {
        if (!chromeExecutable) {
          throw new Error("Chrome was not found; set BROWSER_JR_BENCH_CHROME_PATH");
        }
        return new PlaywrightAdapter({
          id: "chrome",
          label: "Google Chrome (Blink)",
          browserType: chromium,
          launchOptions: { executablePath: chromeExecutable },
        });
      },
    ],
    [
      "firefox",
      () =>
        new PlaywrightAdapter({
          id: "firefox",
          label: "Firefox (Gecko)",
          browserType: firefox,
        }),
    ],
    [
      "webkit",
      () => new PlaywrightAdapter({ id: "webkit", label: "WebKit", browserType: webkit }),
    ],
    ["lightpanda", () => new LightpandaAdapter()],
    ["agent-browser-chrome", () => new AgentBrowserAdapter("chrome")],
    ["agent-browser-lightpanda", () => new AgentBrowserAdapter("lightpanda")],
  ]);
}
