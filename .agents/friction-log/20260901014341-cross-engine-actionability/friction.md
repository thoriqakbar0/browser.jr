---
title: 'cross-engine actionability probes lack a reusable fixture'
severity: 'minor'
---

## Expected Behavior

One deterministic fixture and runner compare pointer actionability across browser.jr, Playwright engines, and agent-browser.

## Current Behavior

Each stability comparison needs a temporary page, HTTP server, and separate browser commands.

## Possible Solution

Add a loopback actionability fixture and one repeatable cross-engine probe command.

## Minimal Reproducible Example

Compare a continuously moving button across browser.jr, Playwright, and agent-browser. The repository has no shared runner.

## Context

The 2026-09-01 stability audit repeated setup already needed by earlier semantic locator comparisons.
