# Verification: agent-browser plugin

Run these checks with a controlled browser.jr binary and the installed agent-browser CLI. Record both versions.

## automation/agent-browser-plugin.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ABPLUGIN-01 | P1 | pipe | agent-browser discovers the plugin manifest. | Pack the npm source and set `BROWSER_JR_BIN` to a release binary. | Add the local tarball without manual capabilities. | The stored plugin is named `browser-jr` and declares all three capabilities. | pass: agent-browser 0.32.4, 2026-09-02 |
| ABPLUGIN-02 | P1 | pipe | One batch preserves browser.jr session state. | Use the packed plugin and a public page. | Run open, snapshot, and get title in one `browserjr.session` request. | The result contains ready, ordered command, and closed envelopes. | pass: packed-tarball smoke check, 2026-09-02 |
| ABPLUGIN-03 | P1 | pipe | Batch and relay commands cannot inject extra session lines. | Start the plugin executable directly. | Send batch and relay commands containing a line feed. | Each request returns one unsuccessful response without forwarding the injected command. | pass: Node protocol tests, 2026-09-02 |
| ABPLUGIN-04 | P1 | pipe | The relay preserves one warm native session. | Start `serve` with a controlled JSON-session binary. | Forward two separate `browserjr.command` requests. | The returned command sequences are one and two. | pass: Node relay test, 2026-09-02 |
| ABPLUGIN-05 | P1 | pipe | The benchmark measures the plugin path without timing setup. | Start the complete benchmark matrix. | Run 10 samples after one warmup. | Every supported scenario passes and the plugin row records latency. | pass: full Apple M3 run, 2026-09-02 |
| ABPLUGIN-06 | P2 | pipe | Loopback access remains explicit. | Use a controlled loopback fixture. | Run a batch without and then with `allowLoopback`. | The first load is blocked. The second load succeeds. | partial: native policy and benchmark relay passed separately, 2026-09-02 |
| ABPLUGIN-07 | P2 | pipe | Native startup and shutdown failures remain bounded. | Use missing, signal-ignoring, and idle-client fixtures. | Start a session or relay, then trigger each failure. | The plugin returns one JSON failure or closes within the stated bound. | pass: Node lifecycle tests, 2026-09-02 |
