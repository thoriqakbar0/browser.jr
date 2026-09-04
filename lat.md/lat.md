This directory defines the high-level concepts, business logic, and architecture of this project using markdown. It is managed by [lat.md](https://www.npmjs.com/package/lat.md) — a tool that anchors source code to these definitions. Install the `lat` command with `npm i -g lat.md` and run `lat --help`.

- [[landing-page]] — The static product page and its evidence boundary.

# Verified action result

A verified action reports the resolved target, applied checks, committed effect, and document generation in one typed result.

## Shared target path

Reference, generic locator, and role targets resolve to one internal target before using the same actionability and mutation path.

## Successful receipt

A successful action returns its resolved match, passed checks, typed effect, and document generation before and after execution.

## Navigation generation

A navigation effect reports different before and after generations, proving that the action installed a replacement document.
