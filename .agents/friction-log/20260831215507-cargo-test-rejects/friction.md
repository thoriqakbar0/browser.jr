---
title: 'cargo test rejects multiple test filters'
severity: 'minor'
---

### Expected Behavior

A focused test command can select several exact test names.

### Current Behavior

`cargo test` accepts one positional filter and rejects a second filter as an unexpected argument.

### Possible Solution

Run one shared substring filter, separate commands, or the complete relevant test target.

### Minimal Reproducible Example

Run `cargo test --lib first_test second_test`.

### Context

Observed while checking four focused parser and CSS regressions after a refactor. Compilation and debtmap validation still completed.
