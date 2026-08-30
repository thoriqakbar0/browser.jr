# CLI help

## Summary

CLI help shows the available browser.jr invocation shape. Run `./browser.jr help` or `./browser.jr --help` from the repository.

The output names static-HTML lint, interactive snapshot, session mode, and their supported subsets.

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
| Flags and options | Help and version use their listed forms. Lint accepts viewport and width flags. Snapshot accepts `-i` or `--interactive`. Session takes no invocation flags. | Nothing can change. The help command exits immediately. |
| Project configuration | Help does not read project configuration. | Nothing can change. |
| Target matrix | Help does not create a target matrix. | Nothing can change. |
| Output channel | Help and version use stdout. Errors use stderr. | A write failure prevents successful delivery. |

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

**Rendering compatibility.** Help names the horizontal geometry and interactive semantic subsets.

**Isolation.** Help does not execute page or project code.

**Accessibility inspection.** Help names interactive snapshot support. Help does not inspect a page.

## Edge cases

- No arguments shows help.
- `lint` without a URL reports invalid input.
- `lint <url>` loads a loopback HTTP page with a 1280 CSS pixel viewport.
- `lint <url> --viewport <css-px>` sets a positive viewport width.
- `lint <url> --max-width <element> <css-px>` checks one semantic element against a non-negative project limit.
- `snapshot <url> --interactive` reports supported interactive semantic elements.
- `snapshot <url> -i` selects the same implemented projection.
- `snapshot <url>` reports invalid input because no default projection exists.
- `session` starts the persistent stdin command adapter.
- Session help lists every supported line command, including element inspection and page metadata commands.
- Extra invocation arguments after `session` report invalid input.
- A missing or unsupported width target blocks `max-element-width`.
- Extra or unknown arguments report invalid input.
- A closed stdout prevents successful help delivery.

## Open questions and verification

- Decide whether an installed package exposes the dotted command through a symlink or launcher.
- Decide whether 1280 CSS pixels remains the default viewport.
- Add installed-package checks after packaging exists.

Drafted from the local Rust implementation on 2026-08-31. Unit and compiled-process tests cover current behavior.
