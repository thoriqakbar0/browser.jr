---
title: 'Debtmap validation prints FAIL before passing'
severity: 'minor'
issue: 'thoriqakbar0/browser.jr#14'
---

## Expected Behavior

A successful validation prints one passing verdict without an earlier failure verdict.

## Current Behavior

The command prints [ERROR] Pass/Fail: FAIL, then [OK] Validation PASSED, and exits zero.

## Possible Solution

Make the summary verdict follow the configured validation thresholds used for the final exit status.

## Minimal Reproducible Example

Run: debtmap validate . --config .debtmap.toml --format terminal

## Context

The contradictory output makes pre-commit evidence hard to interpret.
