import { resolve } from "node:path";

import { SCENARIO_IDS } from "./scenarios.mjs";

export const ADAPTER_IDS = Object.freeze([
  "browser-jr",
  "agent-browser-browser-jr",
  "chrome",
  "firefox",
  "webkit",
  "lightpanda",
  "agent-browser-chrome",
  "agent-browser-lightpanda",
]);

function parseNonNegativeInteger(value, option) {
  if (!/^\d+$/.test(value)) {
    throw new TypeError(`${option} requires a non-negative integer`);
  }
  return Number.parseInt(value, 10);
}

function parseSelection(value, allowed, option) {
  const selected = value.split(",").filter(Boolean);
  if (selected.length === 0 || selected.some((item) => !allowed.includes(item))) {
    throw new TypeError(`${option} contains an unknown value`);
  }
  return [...new Set(selected)];
}

export function parseOptions(argv, cwd = process.cwd()) {
  const options = {
    iterations: 10,
    warmup: 1,
    adapters: [...ADAPTER_IDS],
    scenarios: [...SCENARIO_IDS],
    output: resolve(cwd, "results/latest.json"),
    list: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const value = argv[index + 1];
    if (argument === "--") {
      continue;
    } else if (argument === "--iterations" && value !== undefined) {
      options.iterations = parseNonNegativeInteger(value, argument);
      index += 1;
    } else if (argument === "--warmup" && value !== undefined) {
      options.warmup = parseNonNegativeInteger(value, argument);
      index += 1;
    } else if (argument === "--engines" && value !== undefined) {
      options.adapters = parseSelection(value, ADAPTER_IDS, argument);
      index += 1;
    } else if (argument === "--scenarios" && value !== undefined) {
      options.scenarios = parseSelection(value, SCENARIO_IDS, argument);
      index += 1;
    } else if (argument === "--output" && value !== undefined) {
      options.output = resolve(cwd, value);
      index += 1;
    } else if (argument === "--list") {
      options.list = true;
    } else {
      throw new TypeError(`unknown or incomplete option: ${argument}`);
    }
  }

  if (options.iterations < 1 && !options.list) {
    throw new TypeError("--iterations must be at least one");
  }
  return options;
}
