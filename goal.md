# Goal: maintain the browser.jr product description

You are working in the `browser.jr` repository. Read `README.md` and `glossary.md` first. The README defines the product, current status, project notes, and coverage table. This file defines the documentation workflow.

The repository is at `/Users/thor/work/browser.jr`. Read the README's dated status before describing implementation.

Treat unimplemented intent as decided behavior, not runtime evidence.

## Document ownership

- `README.md` owns product identity, scope, current status, document structure, and coverage.
- Each feature document owns its observable behavior.
- `glossary.md` owns shared terms.
- `architecture.md` owns internal design and must not redefine user-visible behavior.
- `inspection/query-elements.md` owns locator types, syntax, matching, selection, resolution, actions, and failure behavior.
- `verification/` records runtime proof and must not define desired behavior.
- `bug-triage.md` records confirmed conflicts between intent and evidence.

Link to the owner when another document needs the same fact. Do not maintain two copies of one rule.

## Source of truth

Use these sources for current behavior, in order:

1. Running-product checks with recorded inputs and environment.
2. Behavior, regression, and web-platform conformance tests.
3. Session, evaluator, page, parsing, style, layout, and rule code.
4. CLI help, diagnostics, and package exports.

Use explicit product decisions in the owning document for intended behavior. When current evidence conflicts with intent, preserve both and add a `bug-triage.md` entry.

If a behavior has no implementation, record it under "Open questions and verification." Do not invent syntax, defaults, numeric limits, or compatibility.

An explicit product decision may define unimplemented behavior. Label it as decided and unverified.

## Change workflow

1. Identify the document that owns the changed behavior.
2. Read its implementation path, tests, and current verification rows.
3. Update the owner document before summaries and indexes.
4. Update shared terms only in `glossary.md`.
5. Update README status, scope, structure, or coverage when affected.
6. Add or revise one observable verification claim.
7. Record conflicting evidence in `bug-triage.md`.
8. Run links, tests, and the relevant hand checks.
9. Record the evidence date and commit when one exists.

Use `implemented`, `verified`, `decided`, and `open` as evidence labels. The README defines their meanings.

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

## Product decisions

The README's scope decisions own product intent. Feature documents own detailed behavior. Do not copy either set into this workflow file.

## Order of work

1. Keep the completed `cli/help.md` pilot synchronized with the runnable binary.
2. Write foundations in README order.
3. Write the loading and inspection documents after their complete state handling can be traced.
4. Write verification, automation, and cross-cutting documents.
5. Build verification checklists from observable claims.
6. Run the link checker and a full consistency pass.
7. Mark a document `verified` only after its important hand checks pass or reach triage.

## Working rules

- Keep the README structure and coverage table synchronized.
- Keep each product fact in one owning document.
- Preserve stable checklist and triage identifiers after their first use.
- Keep changes focused on described behavior.
- Record unknowns and continue with supported facts.
- Keep the implementation and product documents in this repository.
- Commit, push, file issues, deploy, or release only when Thoriq asks.

The description is complete when every coverage-table document exists, links resolve, vocabulary agrees, and verification status matches recorded evidence.
