# Benchmark harness

The benchmark compares browser-control paths over one controlled two-page fixture. It measures complete supported workflows, not isolated parser functions.

## Runner lifecycle

[`runAdapter`](../benchmarks/run.mjs) creates one adapter, calls `prepare`, runs supported scenarios, and calls `close` in a `finally` block. The fixture server also closes in a top-level `finally` block.

Each adapter starts once and reuses one warm browser session. Scenario setup runs before the timer. Correctness verification runs after the timed operation.

## Scenario ownership

[`SCENARIOS`](../benchmarks/lib/scenarios.mjs) defines the workload and capability required by each scenario. [`runScenario`](../benchmarks/run.mjs) records only verified samples.

Unsupported capabilities produce no latency. A wrong title, snapshot, value, or state marks the scenario failed.

## browser.jr paths

The direct adapter keeps one native release session and sends commands over its stdin and stdout stream.

The plugin adapter starts the authenticated relay during `prepare`. Each measured command still starts the agent-browser CLI and plugin process, connects to the warm relay, and crosses the plugin protocol.

The two rows use the same browser.jr engine. Their difference measures integration overhead, not a different rendering implementation.

## Timing and output

The runner discards warmups, records measured durations, and calculates min, median, p95, max, mean, and standard deviation in `benchmarks/lib/stats.mjs`.

[`benchmarks/results/README.md`](../benchmarks/results/README.md) is the readable result. The generated JSON records host, repository, configuration, capabilities, correctness, and samples.

## Maintenance note

The direct and plugin adapters currently repeat browser.jr scenario commands and assertions. A shared command plan may reduce drift while keeping transport code separate.
