---
title: 'CLI test flake can leave cargo test hanging'
severity: 'minor'
issue: 'thoriqakbar0/browser.jr#7'
---

## Expected Behavior

`cargo test --all-targets -- --test-threads=1` should finish after every test result.

## Current Behavior

`failed_label_locator_preserves_current_snapshot_references` failed once during the commit hook. The CLI test process then stayed alive for more than three minutes without completing the suite.

The same test passed alone on the next run.

## Possible Solution

Make the fixture startup deterministic and guarantee child-process cleanup after an assertion failure.

## Minimal Reproducible Example

Run `cargo test --all-targets -- --test-threads=1`. If the named test fails, observe whether `tests/cli.rs` exits. Then run `cargo test --test cli failed_label_locator_preserves_current_snapshot_references -- --exact --nocapture --test-threads=1`.

## Context

The flake interrupted a documentation-only commit after all benchmark checks had passed.
