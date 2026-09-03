---
title: 'semantic role probes lack a reusable fixture'
severity: 'minor'
issue: 'thoriqakbar0/browser.jr#12'
---

## Expected Behavior

One deterministic fixture and runner compare semantic roles across browser.jr, Playwright engines, and agent-browser.

## Current Behavior

Each comparison needs a new inline HTTP server and browser script.

## Possible Solution

Add a loopback accessible-name fixture and a repeatable cross-engine probe command.

## Minimal Reproducible Example

Compare an icon-only button across browser.jr, Playwright, and agent-browser. The repository provides no shared fixture or runner.

## Context

The 2026-09-01 role-locator audit repeated this setup for descendant image, label, and reference cases.
