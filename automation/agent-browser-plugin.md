# Run browser.jr through an agent-browser plugin

## Summary

The `agent-browser-plugin-browser-jr` executable implements the `agent-browser.plugin.v1` stdio protocol.

It exposes browser.jr as a namespaced `command.run` plugin. It does not replace agent-browser's Chrome or Lightpanda engine.

The plugin supports one bounded session batch and one authenticated local relay command.

## The simple case

The caller builds browser.jr, packs the local npm artifact, and supplies the native binary through `BROWSER_JR_BIN`.

```sh
cargo build --release --bin browser-jr
npm pack --pack-destination /tmp
export BROWSER_JR_BIN="$PWD/target/release/browser-jr"

agent-browser plugin add \
  file:/tmp/agent-browser-plugin-browser-jr-0.1.0.tgz

agent-browser plugin run browser-jr browserjr.session \
  --payload '{"commands":["open https://example.com","snapshot -i","get title"]}'
```

The plugin starts `browser-jr --json session`, writes the commands in order, appends `exit`, and returns the JSON lifecycle and command envelopes.

Loopback access remains off unless the payload sets `allowLoopback` to `true`.

## The interaction, event by event

### Discover the manifest

agent-browser starts the executable and sends one `plugin.manifest` request.

The plugin returns the name `browser-jr` and these capabilities:

- `command.run`
- `browserjr.session`
- `browserjr.command`

The executable writes one protocol response to stdout. Diagnostics and native-process stderr do not become extra protocol lines.

### Run one session batch

A `browserjr.session` request contains from 1 to 100 commands.

Each command contains from 1 to 8,192 characters. The complete command list contains at most 65,536 bytes.

Commands cannot contain carriage returns or line feeds. The caller cannot include `exit` because the plugin appends it.

The optional `timeoutMs` value ranges from 1 to 120,000 milliseconds. Its default is 30,000 milliseconds.

The plugin resolves the native executable in this order:

1. The request's `binary` value.
2. `BROWSER_JR_BIN`.
3. A packaged `target/release/browser-jr` executable.
4. `browser-jr` on `PATH`.

The plugin returns the complete ordered JSON event list after browser.jr emits both `ready` and `closed`.

### Forward one warm-session command

The executable's `serve` mode owns one `browser-jr --json session` process and listens on a loopback TCP port.

It prints one ready response containing the host, selected port, and authentication token.

A `browserjr.command` request supplies that connection data and one browser.jr command. Relay commands use the same 8,192-character, line-break, and `exit` restrictions as batch commands. The relay serializes commands, writes one line to the native session, and returns the matching command envelope.

The benchmark uses this variant so setup remains outside each timed sample.

## Variants

| Variant | Behavior |
| --- | --- |
| Public page batch | Uses browser.jr's default public-network boundary. |
| Loopback page batch | Requires `allowLoopback: true`. |
| Explicit binary | Uses the request's `binary` path. |
| Environment binary | Uses `BROWSER_JR_BIN` when the request omits `binary`. |
| Warm relay | Uses `serve` plus `browserjr.command` for sequential commands. |
| Unsupported agent-browser provider | Remains unavailable because browser.jr does not expose CDP. |

## Cancel and interrupt

| Interrupt | Behavior |
| --- | --- |
| Plugin process termination | Terminates the current one-shot request. |
| Session timeout | Sends `SIGTERM`, escalates to `SIGKILL` after one second, and returns a plugin failure. |
| Relay `SIGINT` or `SIGTERM` | Closes the listener and asks the native session to exit. |
| Native relay process exit | Closes the listener. Later commands fail to connect. |
| Idle relay client | The relay destroys the socket after 30 seconds or during relay shutdown. |
| Client disconnect | Does not cancel a command already queued by the relay. |

## Interactions with other systems

The plugin preserves browser.jr's network policy. It does not add private-network access, JavaScript, CDP, or broader rendering support.

agent-browser owns plugin discovery, capability confirmation, config storage, and the outer result envelope.

browser.jr owns page state, command parsing, references, loading, actions, inspection, screenshots, and native errors.

The npm package is only the protocol adapter. Native binary distribution is a separate release concern.

## Edge cases

Malformed protocol JSON, unknown request types, invalid command lists, invalid ports, invalid timeouts, failed authentication, unavailable binaries, native nonzero exits, and relay connection failures return one unsuccessful plugin response.

The relay rejects `exit`. Its process lifecycle owns native-session shutdown.

The relay accepts only one JSON request per TCP connection and serializes concurrent commands against the single native session.

A custom agent-browser plugin is not a custom agent-browser engine. Normal commands such as `agent-browser open` and `agent-browser snapshot` continue to use agent-browser's selected built-in engine unless a CDP-compatible `browser.provider` is configured.

## Open questions and verification

Implemented and verified on 2 September 2026:

- installed agent-browser 0.32.4 discovers the local package manifest
- `browserjr.session` opens, snapshots, and reads a public page through a packed npm tarball
- the authenticated relay preserves command sequence across separate plugin requests
- the cross-engine smoke benchmark passes through the warm relay

Open:

- bound relay startup, command execution, and forced shutdown under BJR-014
- block npm publication until the owner selects a license and a native binary distribution policy under BJR-015
- reject malformed relay port tokens under BJR-016
- test protocol compatibility against older agent-browser releases
- decide whether future CDP support should introduce a separate `browser.provider` capability
