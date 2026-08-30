# Architecture synthesis

Date: 28 August 2026.

Implementation evidence updated: 30 August 2026.

This note records the architecture arena that produced the [architecture draft](../architecture.md). It is a design record, not runtime evidence.

## Candidates

Three isolated candidates started from the same product and research grounding.

1. The session-command candidate put one deep `Session` dispatcher at the center.
2. The evidence-first candidate put immutable snapshots and typed rule requirements at the center.
3. The layout-kernel candidate put the declarative field schedule and Spineless Traversal at the center.

## Rubric and scores

The cross-judge scored each criterion from zero to four.

| Criterion | Session command | Evidence first | Layout kernel |
| --- | ---: | ---: | ---: |
| Caller fit | 2 | 3 | 2 |
| Domain model | 3 | 4 | 4 |
| Interface depth | 3 | 3 | 4 |
| Correctness | 3 | 4 | 3 |
| Buildability | 3 | 3 | 3 |
| Reader load | 3 | 3 | 3 |
| Total | 17 | 20 | 19 |

## Base selection

The evidence-first candidate became the base.

Its `ObservationCell<T>` keeps support state beside each observation. Its `Requirements<I>` projector gives each rule typed input. Its target result separates completed comparisons from blocked targets.

These choices protect the main product promise. Missing layout evidence cannot become a clean result.

## Grafts

The final draft takes these parts from the layout-kernel candidate:

- one browser.jr-owned `LayoutKernel`
- one validated `LayoutProgram`
- normalized box and fragment stores
- dirty transition and complete-value change rules
- generation checks for deleted queued work
- ordered bulk work for inserted subtrees
- the same field program for clean and incremental layout

The final draft takes page-scoped snapshot identity and session-owned capability grants from the session-command candidate.

## Rejections

All three candidates used one closed command model. Two candidates returned one unrelated reply enum.

That shape allows a valid command to produce a mismatched reply value. Callers then need an `UnexpectedReply` error for an invalid internal state. The final draft uses typed requests with associated replies.

The first implementation also rejects these ideas:

- a session mailbox or actor without concurrent caller evidence
- a type-erased rule catalog before a second rule input shape exists
- macro generation before the explicit field table repeats
- predefined agent actions before the product defines them
- `Taffy` types in the layout, snapshot, rule, or session contracts
- one element rectangle as the layout model

## Red-flag review

The final shape keeps transport and `Taffy` representations outside the core. It organizes modules by ownership. It gives `Session` and `LayoutKernel` meaningful policy instead of pass-through methods.

The remaining risk is first-slice size. The implementation must begin with one field table, one mutation, one fragment relation, and one rule.

## Consistency result

The synthesis was reviewed against `README.md`, `goal.md`, `glossary.md`, and the design-lint documents. It keeps unavailable evidence from becoming a pass and carries target-matrix context into findings.

The runtime now loads bounded loopback HTML and computes a stated horizontal layout subset. It exercises typed requests, immutable evidence, overflow and project width rules, and clean-equivalent width invalidation. General incremental equivalence remains unproved.
