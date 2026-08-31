---
title: 'Semantic and reference actions disagree on stylesheet visibility'
severity: 'major'
---

## Expected Behavior

Semantic locators and snapshot references should apply the same actionability evidence.

## Current Behavior

A page containing an embedded `style` block gives different results for the same controls.

`find label "Agent name" fill Benchmark User --exact` blocks because stylesheet visibility is unsupported.

`snapshot -i` followed by `fill @e2 Benchmark User` succeeds on that textbox.

Role-based link clicking blocks for the same reason, while clicking its snapshot reference succeeds.

## Possible Solution

Route semantic and snapshot-reference actions through one actionability policy.

## Minimal Reproducible Example

Run browser.jr session mode against `benchmarks/fixtures/index.html`.

Compare the semantic commands with `snapshot -i`, `fill @e2 Benchmark User`, and `click @e1`.

## Context

The cross-engine benchmark found this while preparing equivalent fill and click scenarios.
