# CLI help

## Summary

CLI help shows the available browser.jr invocation shape. Run `./browser.jr help` or `./browser.jr --help` from the repository.

The output names static-HTML lint, page text, accessibility snapshots, screenshots, snapshot JSON, and JSON session mode.

Session help lists role actions, event draining, focused keyboard text, held-key commands, and role-filter options.

## The simple case

The developer runs `./browser.jr help`. browser.jr writes help to stdout and exits with status zero.

The command does not create a session, page, snapshot, or finding.

## The interaction, event by event

```mermaid
stateDiagram-v2
    [*] --> parsing
    parsing --> showing_help : help or no arguments
    parsing --> showing_version : version
    parsing --> rejected : invalid input
    parsing --> loading : valid lint or snapshot request
    parsing --> session_ready : session request
    showing_help --> finished
    showing_version --> finished
    rejected --> finished
    loading --> finished
    session_ready --> session_running
    session_running --> session_running : command result
    session_running --> finished : exit or EOF
```

### Invoke

The shell starts `browser.jr`. The CLI reads all arguments before selecting a local result.

### Exit immediately

Help and version write to stdout. Invalid arguments write to stderr.

### Begin running

Help never begins page-loading work. A valid lint or snapshot command does.

### While running

No long-running work occurs.

### Finish

Help and version exit with status zero. Invalid input exits with status two.

## Variants

| Modifier | Set at invocation | Changed while running |
| --- | --- | --- |
| Flags and options | Help and version use their listed forms. Lint accepts viewport and width flags. Snapshot and session accept JSON. | Nothing can change. The help command exits immediately. |
| Project configuration | Help does not read project configuration. | Nothing can change. |
| Target matrix | Help does not create a target matrix. | Nothing can change. |
| Output channel | Help and version use stdout. Errors use stderr. JSON invocations return envelopes on stdout. | A write failure prevents successful delivery. |

## Cancel and interrupt

| Event | Before running | While running |
| --- | --- | --- |
| Ctrl+C once | The process may exit before writing complete output. | No long-running phase exists. |
| Ctrl+C again before the evaluation stops | The process may exit before writing complete output. | No long-running phase exists. |
| The process receives SIGTERM | The process may exit before writing complete output. | No long-running phase exists. |
| The terminal closes | Output delivery may fail. | No long-running phase exists. |
| stdin or stdout closes | Closed stdin has no effect. Closed stdout prevents help delivery. | No long-running phase exists. |
| The network fails or a request times out | No effect. Help does not use the network. | No long-running phase exists. |
| The inspected page changes | No effect. Help does not load a page. | No long-running phase exists. |
| Another lint run targets the same page | No effect. Help owns no page state. | No long-running phase exists. |
| The process exits outright | No session or page state exists. | No long-running phase exists. |

## Interactions with other systems

**Configuration precedence.** Help does not read project configuration.

**Output and exit status.** Help and version use stdout. Invalid commands use stderr.

**Resource limits.** Help allocates only its argument list and fixed output text.

**Network and storage.** Help does not use the network or persistent storage.

**Rendering compatibility.** Help names geometry, scrolling, screenshot, and static accessibility subsets.

**Isolation.** Help does not execute page or project code.

**Accessibility inspection.** Help names full-tree and interactive snapshot support. Help does not inspect a page.

## Edge cases

- No arguments shows help.
- `lint` without a URL reports invalid input.
- `lint <url>` loads a loopback HTTP page with a 1280 CSS pixel viewport.
- `read <url>` prints normalized static document text.
- `lint <url> --viewport <css-px>` sets a positive viewport width.
- `lint <url> --max-width <element> <css-px>` checks one semantic element against a non-negative project limit.
- `snapshot <url>` reports the supported full accessibility tree.
- `snapshot <url> --interactive` reports agent-oriented reference targets.
- `snapshot <url> -i` selects the same interactive projection.
- `snapshot <url> -s <css>` limits either projection to one strict CSS subtree.
- `snapshot <url> -u` includes resolved URLs for semantic links.
- `snapshot <url> -c -d 2` prunes empty structure and limits full-tree depth.
- Compact and depth controls are neutral for the interactive projection.
- `--json snapshot <url>` writes one JSON result to stdout.
- `snapshot <url> --json` selects the same JSON output.
- JSON failures use stdout and keep the command's documented exit status.
- A snapshot without a URL reports invalid input.
- `session` starts the persistent stdin command adapter.
- `--json session` writes one envelope for each lifecycle event and command.
- `session --json` selects the same line-oriented output.
- Session help lists viewport sizing, screenshots, page text, history, actions, event records, keyboard text, held keys, boxes, reads, HTML, and selections.
- Session rejects invocation arguments other than one `--json` flag.
- A missing or unsupported width target blocks `max-element-width`.
- Extra or unknown arguments report invalid input.
- A closed stdout prevents successful help delivery.

## Open questions and verification

- Decide whether an installed package exposes the dotted command through a symlink or launcher.
- Decide whether 1280 CSS pixels remains the default viewport.
- Add installed-package checks after packaging exists.

Drafted from the local Rust implementation on 2026-08-31. Unit and compiled-process tests cover current behavior.
