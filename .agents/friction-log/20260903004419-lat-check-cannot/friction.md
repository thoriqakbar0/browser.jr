---
title: 'lat check cannot validate .mjs source links'
severity: 'minor'
target: '1st1/lat.md'
---

## Expected Behavior

`lat check` should validate symbols in `.mjs` files as JavaScript source.

## Current Behavior

The command rejects links such as `[[plugin/cli.mjs#pluginMain]]` because `.mjs` is not a supported source extension. The documentation must use a normal Markdown file link, which cannot validate the symbol.

## Possible Solution

Treat `.mjs` as JavaScript in the source parser and code-reference scanner.

## Minimal Reproducible Example

Add `[[plugin/cli.mjs#pluginMain]]` to a `lat.md/` file, then run `lat check`.

## Context

This occurred while adding the browser.jr knowledge graph on 2026-09-02. The repository uses `.mjs` for its plugin and benchmark adapters.
