# Runtime flow

This page follows one request from a caller to a result. It links the entry points that otherwise require reading the CLI, session, page, and output modules together.

## Entry points

The executable enters through [[src/cli.rs#run_cli_with_input]]. Package callers construct one of the request types exported by `src/lib.rs` and pass it to [[src/session.rs#Session#execute]].

The CLI owns argument parsing, input streams, process exit status, and human or JSON presentation. The session owns browser state and typed request execution. [[architecture]] explains why presentation stays outside the engine.

## Request conversion

One-shot CLI commands become typed session requests before they reach browser state. Session mode first parses a line into [[src/cli_session.rs#SessionCommand]], then converts that command into the same typed request path.

The JSON session adapter in [[src/cli_session_json.rs#run_json_session]] adds lifecycle and sequence envelopes. It does not add a second command model.

```text
argv, package call, or session line
  -> adapter validation
  -> typed SessionRequest
  -> Session::execute
  -> current-page operation
  -> typed reply
  -> human or JSON output
```

## State and presentation boundary

[[src/session.rs#Session]] is the mutable boundary. [[src/cli_output.rs#write_session_command_json]] and the other output functions format replies after execution.

A presentation failure can change the process result, but it does not redefine the session reply. This separation lets package callers use typed values while CLI callers use stable text or JSON.

## Related maps

These pages continue the same execution path from another ownership boundary.

- [[session-state]] describes the mutable state behind `Session::execute`.
- [[network-loading]] follows navigation requests.
- [[page-pipeline]] follows installed HTML through supported evidence.
- [[interaction-pipeline]] follows stateful commands.
- [[evidence-and-snapshots]] follows snapshot and reference replies.
- [[screenshot-pipeline]] follows capture requests.
