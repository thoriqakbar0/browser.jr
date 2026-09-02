# Agent-browser plugin protocol

The npm package adapts the browser.jr JSON session protocol to `agent-browser.plugin.v1`. It exposes commands, not a CDP browser provider.

## Manifest and commands

[`pluginMain`](../plugin/cli.mjs) returns the plugin manifest or handles one command request. The manifest declares `command.run`, `browserjr.session`, and `browserjr.command`.

`browserjr.session` validates a bounded command list, resolves the native executable, starts one `browser-jr --json session`, appends `exit`, and returns the ordered lifecycle and command envelopes.

`browserjr.command` connects to a warm relay. It sends one token and one command, then returns one native command envelope.

## Native executable lookup

[`resolveBinary`](../plugin/cli.mjs) checks the request's `binary`, `BROWSER_JR_BIN`, a packaged release binary, and `browser-jr` on `PATH`, in that order.

This lookup makes local development and tarball testing possible. It does not solve portable native binary distribution. [[release-and-packaging]] maps that remaining work.

## Relay

[`serve`](../plugin/cli.mjs) starts one native JSON session and one loopback TCP listener. The relay authenticates each request with a random token and serializes commands against the native stdin and stdout stream.

The relay is a transport owner. [[session-state]] remains the browser-state owner.

## Current lifecycle gaps

The current relay has no timeout while it waits for the native `ready` event. Listener setup errors do not always terminate the native child. Relay shutdown sends `SIGTERM` but does not force termination when the child ignores it.

A client socket timeout also does not cancel its queued native command. One stalled command can block later commands, and a timed-out command can complete after its caller has observed failure.

These are confirmed implementation gaps, not intended behavior. [BJR-014](../bug-triage.md#bjr-014) owns the conflict record. [[verification-map]] links the affected check.

## Evidence

[`plugin/test/plugin.test.mjs`](../plugin/test/plugin.test.mjs) covers manifest discovery, batch validation, injection rejection, missing binaries, batch timeout escalation, relay sequencing, and idle socket shutdown.

The missing relay lifecycle cases remain listed in [[verification-map]]. The user-facing request contract remains in [automation/agent-browser-plugin.md](../automation/agent-browser-plugin.md).

## Compatibility boundary

Normal `agent-browser open` and `snapshot` commands continue to use an agent-browser engine. A future `browser.provider` requires CDP and remains a separate design.
