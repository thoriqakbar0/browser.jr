# Verification: design lint

Run these checks against a controlled local page. Keep the project configuration and target matrix recorded beside each result.

## verification-features/design-lint.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DLINT-01 | P1 | tty | Design lint loads a supplied local URL ([The simple case](../verification-features/design-lint.md#the-simple-case)). | Serve a valid page locally. | 1. Run `browser.jr lint <url>`.<br>2. Wait for completion. | The command checks the supplied page and reports a final result. | — |
| DLINT-02 | P1 | tty | A page-load failure cannot become a design pass ([Begin running](../verification-features/design-lint.md#begin-running)). | Use an unreachable local URL. | 1. Run design lint.<br>2. Wait for completion. | The result identifies a load failure or blocked target. It does not report a clean pass. | — |
| DLINT-03 | P1 | tty | Every finding carries its required context ([Finish](../verification-features/design-lint.md#finish)). | Serve a page with known horizontal overflow. | 1. Run design lint.<br>2. Inspect the finding. | The finding contains rule, severity, element, viewport, expectation, observed value, and evidence. | — |
| DLINT-04 | P1 | tty | Unsupported rendering behavior cannot produce a pass ([Interactions with other systems](../verification-features/design-lint.md#interactions-with-other-systems)). | Serve a page using one declared unsupported feature. | 1. Run a rule depending on that feature.<br>2. Inspect its result. | The rule becomes blocked or unsupported. It does not pass. | — |
| DLINT-05 | P2 | tty | The run checks every requested target matrix entry ([While running](../verification-features/design-lint.md#while-running)). | Configure two viewports and two profiles. | 1. Run design lint.<br>2. Inspect the summary. | Four distinguishable target results appear. | — |
| DLINT-06 | P2 | pipe | Machine-readable output contains no terminal decoration ([Interactions with other systems](../verification-features/design-lint.md#interactions-with-other-systems)). | Select machine-readable output. | 1. Pipe stdout to a file.<br>2. Parse the file. | The file follows one structured format and contains no progress animation or color codes. | — |
| DLINT-07 | P1 | tty | One Ctrl+C cancels without publishing an incomplete result ([Cancel and interrupt](../verification-features/design-lint.md#cancel-and-interrupt)). | Serve a page with a deliberately slow request. | 1. Start design lint.<br>2. Press Ctrl+C during loading.<br>3. Inspect output. | The invocation stops gracefully. No incomplete result appears as final. | — |
| DLINT-08 | P1 | tty | Repeated Ctrl+C forces the process to stop ([Cancel and interrupt](../verification-features/design-lint.md#cancel-and-interrupt)). | Serve a page with a deliberately slow request. | 1. Start design lint.<br>2. Press Ctrl+C twice before it stops. | The process exits at once. Partial output does not claim completion. | — |
| DLINT-09 | P2 | network | One network failure does not erase a completed matrix result ([Cancel and interrupt](../verification-features/design-lint.md#cancel-and-interrupt)). | Configure two matrix entries. Fail a required request during the second. | 1. Start design lint.<br>2. Wait for the first result.<br>3. Fail the second entry's request. | The first result remains complete. The second entry becomes blocked. | — |
| DLINT-10 | P1 | watch | Watch mode reruns after a relevant page change settles ([While running](../verification-features/design-lint.md#while-running)). | Start a development server and watch mode. | 1. Wait for the first result.<br>2. Change page layout.<br>3. Wait for the next result. | A new run reports the changed rendered state. The first result stays distinguishable. | — |
| DLINT-11 | P1 | watch | Incremental layout matches a clean layout ([While running](../verification-features/design-lint.md#while-running)). | Serve a page with scripted style, text, insertion, and removal changes. | 1. Capture the initial result.<br>2. Apply each change in watch mode.<br>3. Run a fresh process after each change.<br>4. Compare structured evidence. | Each incremental result equals its clean-layout result. | — |

Not checkable by hand yet:

- Numeric package-size, speed, memory, and time budgets remain undefined.
- Exact exit statuses need a product decision.
- Exact page-readiness and change-settling rules need a product decision.
