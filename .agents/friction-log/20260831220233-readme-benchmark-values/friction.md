---
title: 'README benchmark values lack their measurement recipe'
severity: 'minor'
---

## Expected Behavior

The README should explain what each benchmark value measures and which control paths it compares.

## Current Behavior

The README lists full-workflow medians and names the actions. It does not explain timed boundaries, correctness checks, or adapter paths.

## Possible Solution

Add a short comparison and calculation explanation before the result table.

## Minimal Reproducible Example

Read the `Benchmark snapshot` section without opening `benchmarks/README.md` or adapter source.

## Context

This appeared while making the README understandable without requiring benchmark implementation knowledge.
