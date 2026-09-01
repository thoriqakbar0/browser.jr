# Browser control benchmarks

This suite compares browser.jr with Chrome, Firefox Gecko, WebKit, Lightpanda, and agent-browser.

It measures browser-control workflows, not rendering-engine internals.

[Read the latest recorded result](results/README.md) for a plain-language summary and latency tables.

Chrome, Firefox, and WebKit use Playwright.

Lightpanda uses Puppeteer over CDP, matching Lightpanda's published benchmark.

agent-browser scenarios include CLI and daemon overhead.

Direct engine scenarios include their controller library overhead.

## Fairness rules

- Every engine loads the same loopback fixture.
- Every engine uses one warm browser session.
- Setup runs outside each timed sample.
- Correctness checks run after every sample.
- Raw samples stay in the JSON result.
- Unsupported work stays visible and receives no timing.
- Engines run sequentially to reduce resource interference.

The suite does not rank unsupported work as slow or failed.

## Scenarios

The names follow the upstream agent-browser benchmark when behavior overlaps.

| Scenario | Timed work |
| --- | --- |
| `navigate` | Load the fixture and read its title. |
| `snapshot` | Produce supported accessibility evidence for the fixture controls. |
| `fill` | Fill one labelled field. |
| `click` | Follow one same-context link. |
| `evaluate` | Evaluate one JavaScript expression. |
| `screenshot` | Capture one viewport PNG. |
| `agent-loop` | Snapshot, click, then snapshot. |
| `full-workflow` | Navigate, inspect, mutate controls, read state, click, and read the destination. |

browser.jr reports `evaluate` and `screenshot` as unsupported.

Lightpanda reports `screenshot` as unsupported because it does not paint pixels.

## Install

```sh
cd benchmarks
pnpm install --frozen-lockfile
pnpm browsers:install
```

The Playwright command installs its pinned Firefox and WebKit builds.

The direct Chrome adapter prefers `BROWSER_JR_BENCH_CHROME_PATH`.

It otherwise checks system Chrome and agent-browser's Chrome for Testing cache.

Install `lightpanda` and `agent-browser` separately before running their adapters.

## Run

```sh
cd benchmarks
pnpm bench
```

Run one sample while changing the suite:

```sh
pnpm bench:smoke
```

Focus the matrix when debugging:

```sh
pnpm bench -- --iterations 20 --warmup 2 \
  --engines browser-jr,chrome,firefox,webkit,lightpanda \
  --scenarios snapshot,agent-loop,full-workflow
```

Probe actionability and native event order separately:

```sh
pnpm probe:actionability
```

The probe uses one moving target and two static targets.

It reports Playwright Chrome, Firefox, WebKit, and agent-browser Lightpanda results without hiding conflicts.

Use `pnpm bench -- --list` to print accepted engine and scenario names.

The default result path is `benchmarks/results/latest.json`.

The result records versions, host facts, git state, assertions, and latency distributions.

The repository ignores raw JSON results. Update the recorded Markdown result only after a complete run passes.

Any unavailable engine or incorrect result makes the command exit nonzero.

## Read results

Compare medians only when host, versions, fixture, iterations, and warmups match.

Do not compare these results as pure rendering-engine timings.

Use p95 and standard deviation to identify unstable control paths.

Snapshot bytes measure returned evidence size, not semantic quality or token count.

Snapshot shapes follow each controller API and may contain different evidence breadth.

Do not use these numbers to claim complete web compatibility.

The fixture covers browser.jr's current static HTML boundary.

Add conformance cases before claiming support for JavaScript, events, painting, or remote pages.

## Source benchmarks

- [agent-browser daemon benchmarks](https://github.com/vercel-labs/agent-browser/tree/main/benchmarks)
- [Lightpanda benchmark protocol](https://github.com/lightpanda-io/demo/blob/main/BENCHMARKS.md)
- [Playwright browser installation](https://playwright.dev/docs/browsers)
