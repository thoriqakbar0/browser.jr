---
title: 'benchmark --list requires installed Playwright'
severity: 'minor'
---

## Expected Behavior

`pnpm bench -- --list` should list engines and scenarios without needing Playwright, or the setup docs should state that dependencies are required even for `--list`.

## Current Behavior

In a fresh Monkey worktree without `benchmarks/node_modules`, the command fails before option parsing because `run.mjs` eagerly imports the Playwright adapters.

## Possible Solution

Move adapter imports after the `--list` early return, or document that `pnpm install --frozen-lockfile` is required for `--list`.

## Minimal Reproducible Example

From a fresh worktree, run `cd benchmarks && pnpm bench -- --list` before installing dependencies. It fails with `ERR_MODULE_NOT_FOUND` for `playwright`.

## Context

This occurred while adding an agent-browser plugin adapter on September 2, 2026. The benchmark test suite passed because it uses only Node built-ins.
