---
title: 'compiled CLI navigation test hangs'
severity: 'major'
---

## Expected Behavior

cargo test --all-targets completes or fails within a bounded time.

## Current Behavior

The run stalls in session_mode_clicks_a_link_by_role_and_re_resolves_the_new_document for more than 50 seconds. It emits no assertion or timeout and leaves cargo alive.

## Possible Solution

Read each request through the blank header line before writing and closing the response.

## Minimal Reproducible Example

Run cargo test --all-targets -- --test-threads=1.

## Context

Observed twice on 2026-08-31 in the feat/css worktree while verifying CSS and DOM changes.
Resolved by consuming request headers in the CLI and package test fixtures. The full all-targets suite now passes.
