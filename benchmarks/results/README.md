# Recorded browser control benchmark

This report makes one complete benchmark run easy to scan. It is a result from one machine, not a universal browser ranking.

Every supported scenario passed its correctness check. Unsupported scenarios have no timing.

## Full workflow

The full workflow opens the fixture, inspects it, changes three controls, reads their state, follows a link, and reads the destination title.

| Adapter | Median | p95 |
| --- | ---: | ---: |
| browser.jr CLI | 0.82 ms | 0.94 ms |
| agent-browser plugin with browser.jr | 651.56 ms | 663.07 ms |
| Google Chrome (Blink) | 162.27 ms | 169.38 ms |
| Firefox (Gecko) | 126.61 ms | 143.81 ms |
| WebKit | 82.06 ms | 98.35 ms |
| Lightpanda | 25.92 ms | 27.14 ms |
| agent-browser with chrome | 445.53 ms | 452.30 ms |
| agent-browser with lightpanda | 357.35 ms | 380.71 ms |

## Basic scenario medians

| Adapter | Navigate | Snapshot | Fill | Click |
| --- | ---: | ---: | ---: | ---: |
| browser.jr CLI | 0.84 ms | 0.04 ms | 0.02 ms | 0.31 ms |
| agent-browser plugin with browser.jr | 129.70 ms | 64.79 ms | 64.86 ms | 65.66 ms |
| Google Chrome (Blink) | 42.94 ms | 10.92 ms | 9.14 ms | 80.62 ms |
| Firefox (Gecko) | 12.30 ms | 14.99 ms | 13.77 ms | 59.12 ms |
| WebKit | 4.02 ms | 13.19 ms | 12.52 ms | 45.04 ms |
| Lightpanda | 1.72 ms | 0.41 ms | 16.44 ms | 5.78 ms |
| agent-browser with chrome | 114.52 ms | 37.48 ms | 38.08 ms | 42.51 ms |
| agent-browser with lightpanda | 72.65 ms | 36.01 ms | 35.41 ms | 35.55 ms |

## Extended scenario medians

| Adapter | Evaluate | Screenshot | Agent loop | Full workflow |
| --- | ---: | ---: | ---: | ---: |
| browser.jr CLI | unsupported | unsupported | 0.32 ms | 0.82 ms |
| agent-browser plugin with browser.jr | unsupported | unsupported | 202.13 ms | 651.56 ms |
| Google Chrome (Blink) | 1.46 ms | 27.17 ms | 94.31 ms | 162.27 ms |
| Firefox (Gecko) | 1.87 ms | 27.25 ms | 77.05 ms | 126.61 ms |
| WebKit | 0.84 ms | 8.53 ms | 61.77 ms | 82.06 ms |
| Lightpanda | 0.15 ms | unsupported | 6.75 ms | 25.92 ms |
| agent-browser with chrome | 36.67 ms | 71.25 ms | 132.98 ms | 445.53 ms |
| agent-browser with lightpanda | 35.19 ms | unsupported | 108.27 ms | 357.35 ms |

## Run facts

- Status: passed
- Time: 2 September 2026, 17:12 to 17:13 UTC
- Source: working tree based on `f1735741a5e745df6fa4469239e5df060ce6daa4` with 19 changed files
- Samples: 10 measured runs after 1 warmup
- Host: Apple M3, 8 logical CPUs, 24 GiB memory, Darwin 25.6.0
- Runtime: Node.js v26.1.0
- Fixture: loopback HTTP with an ephemeral port

| Adapter | Version | Control path |
| --- | --- | --- |
| browser.jr CLI | browser.jr 0.1.0 | browser.jr session stdin |
| agent-browser plugin with browser.jr | agent-browser 0.32.4; browser.jr 0.1.0; plugin 0.1.0 | agent-browser CLI → plugin v1 → browser.jr JSON session |
| Google Chrome (Blink) | 147.0.7727.117 | Playwright |
| Firefox (Gecko) | 153.0 | Playwright |
| WebKit | 26.5 | Playwright |
| Lightpanda | 1.0.0-nightly.8605+23bba947 | Puppeteer over CDP |
| agent-browser with chrome | agent-browser 0.32.4 | agent-browser CLI |
| agent-browser with lightpanda | agent-browser 0.32.4 | agent-browser CLI |

## Read the numbers carefully

- These timings measure browser-control workflows, not rendering-engine internals.
- browser.jr handles the fixture through its current static HTML boundary. It does not support JavaScript evaluation or the benchmark screenshot scene.
- The browser.jr plugin row uses the same native engine as the direct row. Its added latency measures agent-browser CLI, plugin-process, protocol, and relay overhead.
- agent-browser timings include its CLI and daemon overhead. Direct engine timings include their controller library overhead.
- Unsupported work stays unranked. It does not count as slow or failed.
- Compare another run only when its host, versions, fixture, sample count, and warmup count match.

Run `pnpm bench` from `benchmarks/` to produce the ignored raw result at `benchmarks/results/latest.json`.
