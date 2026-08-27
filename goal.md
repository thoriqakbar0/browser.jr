# Goal: complete the browser.jr product description

You are working in the `browser.jr` repository. Read `README.md` and `glossary.md` first. The README defines the product, document template, fixed interaction shape, structure, and coverage table.

The repository is at `/Users/thor/work/browser.jr`. It contains an early Rust implementation. Treat unimplemented product intent as requirements, not runtime evidence.

## Source of truth

Use sources in this order once implementation begins:

1. Session state, the evaluator, and the page lifecycle.
2. Parsing, style, layout, and user-agent behavior.
3. Behavior, regression, and web-platform conformance tests.
4. REPL commands, package exports, help output, and diagnostics.
5. Defaults, thresholds, resource budgets, and compatibility declarations.

If a behavior has no implementation, record it under "Open questions and verification." Do not invent syntax, defaults, numeric limits, or compatibility.

## Writing rules

- Follow the README's eight-section template for each feature document.
- Use the fixed variant rows and interrupt rows in their stated order.
- Use the fixed cross-cutting concerns in their stated order.
- Use glossary terms. Define a missing term before using it.
- Describe user input, visible progress, returned values, and lasting state.
- Put necessary mechanisms only in `> Technical note:` blocks.
- Use sentence case and direct, concrete language.
- Link to the document that owns a rule instead of repeating it.
- Include one `stateDiagram-v2` for every evaluation interaction.
- End with open questions and the current evidence state.
- Never write "verified against" until a source commit exists and the behavior was checked.

## Things already established

- browser.jr is a new browser engine, not a predefined wrapper around another engine.
- The product is a small package for fast, programmable interface verification.
- Grid inspection and user-agent comparison are first-class examples.
- `browser.jr lint <url>` is the primary developer workflow.
- Watch mode repeats design lint during the edit-and-refresh loop.
- The REPL investigates findings and supports custom checks.
- AI agents and package callers use the same session and evaluation model.
- The CLI unit of interaction is an invocation.
- The five phases are invoke, exit immediately, begin running, while running, and finish.
- Built-in rules report measurable defects. Project rules express local design decisions.
- Findings identify the rule, severity, target, viewport, expected value, observed value, and evidence.
- The browser, layout, and lint core use Rust.
- An existing embedded JavaScript runtime may power the REPL.
- The first version uses structured layout evidence, not pixel comparison.
- Watch mode uses Spineless Traversal for incremental layout invalidation.
- A clean full layout remains the correctness oracle and recovery path.
- Performance and package size need numeric budgets before they become testable claims.
- Unsupported web behavior must be visible. It cannot silently produce a claimed valid result.

## Order of work

1. Write `cli/help.md` as the pilot after a runnable binary exists.
2. Write foundations in README order. Add proven rules to this file.
3. Write the loading and inspection documents after their complete state handling can be traced.
4. Write verification, automation, and cross-cutting documents.
5. Build verification checklists from observable claims.
6. Run the link checker and a full consistency pass.
7. Mark a document `verified` only after its important hand checks pass or reach triage.

## Working rules

- Keep the README structure and coverage table synchronized.
- Preserve stable checklist and triage identifiers after their first use.
- Keep changes focused on described behavior.
- Record unknowns and continue with supported facts.
- Treat this repository as both the future source and the description location.
- Commit, push, file issues, deploy, or release only when Thoriq asks.

The description is complete when every planned document exists, links resolve, vocabulary agrees, and verification status matches current evidence.
