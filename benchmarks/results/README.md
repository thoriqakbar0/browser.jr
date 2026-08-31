# Recorded browser control benchmark

This report makes one complete benchmark run easy to scan. It is a result from one machine, not a universal browser ranking.

Every supported scenario passed its correctness check. Unsupported scenarios have no timing.

## Full workflow

The full workflow opens the fixture, inspects it, changes three controls, reads their state, follows a link, and reads the destination title.

| Adapter | Median | p95 |
| --- | ---: | ---: |
| browser.jr | 22.55 ms | 46.39 ms |
| Chrome | 375.36 ms | 559.17 ms |
| Firefox | 643.46 ms | 813.90 ms |
| WebKit | 681.95 ms | 1,348.91 ms |
| Lightpanda | 492.45 ms | 1,461.68 ms |
| agent-browser with Chrome | 7,366.00 ms | 16,838.12 ms |
| agent-browser with Lightpanda | 1,693.90 ms | 3,392.81 ms |

## Basic scenario medians

| Adapter | Navigate | Snapshot | Fill | Click |
| --- | ---: | ---: | ---: | ---: |
| browser.jr | 2.98 ms | 0.30 ms | 0.35 ms | 3.65 ms |
| Chrome | 437.30 ms | 103.72 ms | 112.83 ms | 883.10 ms |
| Firefox | 120.65 ms | 112.84 ms | 162.34 ms | 527.59 ms |
| WebKit | 152.86 ms | 77.78 ms | 82.98 ms | 214.52 ms |
| Lightpanda | 16.97 ms | 6.09 ms | 75.07 ms | 76.20 ms |
| agent-browser with Chrome | 945.86 ms | 287.81 ms | 286.61 ms | 319.01 ms |
| agent-browser with Lightpanda | 633.35 ms | 322.11 ms | 235.85 ms | 148.15 ms |

## Extended scenario medians

| Adapter | Evaluate | Screenshot | Agent loop | Full workflow |
| --- | ---: | ---: | ---: | ---: |
| browser.jr | unsupported | unsupported | 3.99 ms | 22.55 ms |
| Chrome | 17.38 ms | 40.72 ms | 288.86 ms | 375.36 ms |
| Firefox | 43.07 ms | 153.74 ms | 495.09 ms | 643.46 ms |
| WebKit | 8.96 ms | 67.24 ms | 264.39 ms | 681.95 ms |
| Lightpanda | 1.68 ms | unsupported | 93.12 ms | 492.45 ms |
| agent-browser with Chrome | 310.77 ms | 594.03 ms | 4,473.77 ms | 7,366.00 ms |
| agent-browser with Lightpanda | 143.48 ms | unsupported | 447.07 ms | 1,693.90 ms |

## Run facts

- Status: passed
- Time: 31 August 2026, 20:22 to 20:30 WIB
- Source: clean `c779bc2d2680718933993062e56f87c88d0bcccd`
- Samples: 10 measured runs after 1 warmup
- Host: Apple M3, 8 logical CPUs, 24 GiB memory, Darwin 25.5.0
- Runtime: Node.js v26.1.0
- Fixture: loopback HTTP with an ephemeral port

| Adapter | Version | Control path |
| --- | --- | --- |
| browser.jr | 0.1.0 | session stdin |
| Chrome | 147.0.7727.117 | Playwright |
| Firefox | 153.0 | Playwright |
| WebKit | 26.5 | Playwright |
| Lightpanda | 1.0.0-nightly.8605+23bba947 | Puppeteer over CDP |
| agent-browser with Chrome | 0.34.0 | agent-browser CLI and daemon |
| agent-browser with Lightpanda | 0.34.0 | agent-browser CLI and daemon |

## Read the numbers carefully

- These timings measure browser-control workflows, not rendering-engine internals.
- browser.jr handles the fixture through its current static HTML boundary. It does not support JavaScript evaluation or screenshots in this suite.
- agent-browser timings include its CLI and daemon overhead. Direct engine timings include their controller library overhead.
- Unsupported work stays unranked. It does not count as slow or failed.
- Compare another run only when its host, versions, fixture, sample count, and warmup count match.

Run `pnpm bench` from `benchmarks/` to produce the ignored raw result at `benchmarks/results/latest.json`.
