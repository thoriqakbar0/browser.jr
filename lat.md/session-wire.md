# Session wire format

Human session mode and JSON session mode use one line command grammar. JSON mode adds lifecycle, sequence, output, and error envelopes around the same command execution.

## Parse once

[[src/cli_session.rs#parse_command]] turns one input line into [[src/cli_session.rs#SessionCommand]]. [[src/cli_session.rs#run_session]] owns the interactive human stream.

[[src/cli_session_json.rs#run_json_session]] owns the JSON stream. [[src/cli_session_json.rs#run_json_command]] executes one parsed line against the same [[src/cli_session.rs#CliSession]] state.

## Lifecycle envelopes

JSON mode emits `ready` before commands. Each command receives a monotonically increasing sequence identifier and one command envelope. `exit` emits the final command result and then `closed`.

[[src/cli_output.rs#write_session_command_json]] owns command envelope formatting. Lifecycle formatting also stays in `src/cli_output.rs`.

## Error behavior

A command parse or execution failure produces an error envelope for that sequence. The session remains available for later input unless the wire or output stream fails.

The accumulated process status records whether any command failed. One failed command does not silently erase later output.

## Adapter use

The agent-browser batch and relay both drive this wire format. They must preserve line boundaries, sequence order, lifecycle events, and stderr separation.

[[plugin-protocol]] owns the outer agent-browser protocol. [[runtime-flow]] owns the conversion from wire command to typed session request.
