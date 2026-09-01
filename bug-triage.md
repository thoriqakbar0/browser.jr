# Bug triage

Evidence date: 1 September 2026.

Six open compatibility defects affect controller comparisons. The current browser.jr runtime slice passes its automated checks.

Missing behavior belongs in the coverage table. A live-page result that conflicts with decided behavior becomes a triage entry.

## Summary

| ID | Title | Severity | Area | Decision needed | Issue |
| --- | --- | --- | --- | --- | --- |
| BJR-005 | Lightpanda closes its CDP response channel during event-observed fill | medium | `agent-browser` compatibility | Confirm upstream scope, then decide whether to file. | not filed |
| BJR-009 | Playwright and Lightpanda compute several accessible names differently | medium | locator compatibility | Define the remaining name subset or add compatibility profiles. | not filed |
| BJR-010 | Lightpanda clicks continuously moving controls without waiting for stability | medium | actionability compatibility | Keep Playwright stability semantics and confirm Lightpanda's intended boundary. | not filed |
| BJR-011 | Lightpanda treats a `pointer-events:none` overlay as a click blocker | medium | actionability compatibility | Keep Playwright pointer-event semantics and confirm Lightpanda's intended boundary. | not filed |
| BJR-012 | Lightpanda omits PointerEvents and hover exit records | medium | event compatibility | Keep Playwright event families and confirm Lightpanda's intended boundary. | not filed |
| BJR-013 | Playwright engines order pointer transitions differently | low | event compatibility | Keep one explicit target-level order until user-agent profiles exist. | not filed |

## Open suspected conflicts

### BJR-005

Two isolated `agent-browser` 0.32.4 sessions used the configured Lightpanda backend.

One session mutated `about:blank`. The other loaded a controlled loopback page.

Both pages installed input-event listeners before `fill`.

Both fills failed with `CDP response channel closed`.

The sessions and loopback server closed after each attempt.

This blocks a Lightpanda transcript comparison. It does not invalidate the Chromium, Firefox, and WebKit sequence agreement.

### BJR-009

Controlled Playwright 1.62.1 checks used Chromium, Firefox, and WebKit.

All three engines combined multiple labels and used a referenced element's `aria-label`.

They excluded hidden descendant text and presentational image `alt` text from content names.

`agent-browser` 0.32.4 used the configured Lightpanda backend on the same loopback fixtures.

Lightpanda kept only the first label and used referenced descendant text instead of the referenced `aria-label`.

It included hidden descendant text and presentational image `alt` text in content names.

Both paths named an icon-only button from a non-presentational descendant image's `alt` text.

browser.jr implements that shared case. Full accessible-name computation remains open.

### BJR-010

A controlled page moved one button through an 80-millisecond alternating CSS animation.

Playwright 1.62.1 Chromium, Firefox, and WebKit each reached `TimeoutError` after 500 milliseconds.

`agent-browser` 0.32.4 used the configured Lightpanda backend with a 700-millisecond action timeout.

Its batch click completed immediately on the same moving control.

browser.jr follows Playwright here. It blocks inline motion declarations because it cannot sample animation frames yet.

### BJR-011

A controlled static page placed a fixed overlay above one button and set `pointer-events:none` on the overlay.

Playwright 1.62.1 Chromium, Firefox, and WebKit clicked the button.

`agent-browser` 0.32.4 used the configured Lightpanda backend on the same shape.

It rejected the click and named the overlay as the covering element.

browser.jr follows Playwright here. Its supported hit-test slice ignores `pointer-events:none` boxes.

### BJR-012

The repository actionability probe clicked one static button and hovered between two buttons.

`agent-browser` 0.34.0 used Lightpanda with a 1,000-millisecond action timeout.

Its click emitted mouse records, then focused the button after `click`.

It emitted no PointerEvents. Its hover transition omitted exit records and related-target identity.

browser.jr follows Playwright's pointer and mouse event families. It keeps page-script delivery unsupported.

### BJR-013

The same probe used Playwright 1.62.1 with Chrome, Firefox, and WebKit.

Chrome and Firefox matched the target-level click sequence through focus and `click`.

WebKit interleaved pointer and mouse enter records. It did not focus the static button.

The engines also interleaved pointer and mouse hover transitions differently.

browser.jr uses Chrome's target-level transition order. Complete user-agent event profiles remain open.

## Resolved conflicts

| ID | Conflict | Resolution | Evidence |
| --- | --- | --- | --- |
| BJR-001 | macOS loopback loads intermittently returned `EINVAL` or controlled-server connection resets. | The loader avoids racing socket options. Controlled fixtures consume complete request headers before replying. | 100 prior suites and 10 consecutive current CLI suites passed after their respective fixes, 2026-08-31. |
| BJR-002 | `agent-browser` Lightpanda implicit submission differed from HTML default-button and blocker rules. | browser.jr follows the living HTML rule and Playwright workflow. Compatibility documents must keep the Lightpanda difference visible. | Controlled `agent-browser` 0.32.4 requests used the form action, submitted two blockers, and bypassed a disabled default, 2026-08-31. |
| BJR-003 | `agent-browser` Lightpanda returned hidden zero boxes, ignored fixed `top`, collapsed tall blocks, misplaced normal blocks, and kept boxes unchanged after scrolling. | browser.jr follows Playwright's hidden `null` and viewport-relative box contracts. It returns complete boxes only from supported evidence. | Controlled `agent-browser` 0.32.4 checks returned 5-pixel boxes near the origin and `scrollY=0` after explicit and action scrolling, 2026-08-31. |
| BJR-004 | `agent-browser` Lightpanda appended keyboard text despite a zero selection and changed read-only input values. | browser.jr follows Playwright's focused selection model and preserves native read-only state. | Controlled `agent-browser` 0.32.4 checks reported selection `0:0`, appended both keyboard operations, and changed `readonly` value `locked` to `lockedQW`, 2026-08-31. |
| BJR-006 | Playwright backends disagreed on read-only `keyboard.type()` input events. | browser.jr records only the measured shared sequence for read-only type. | Playwright 1.62.1 Chromium added `beforeinput` for printable ASCII. Firefox and WebKit omitted it. Non-ASCII events also differed, 2026-08-31. |
| BJR-007 | Playwright backends disagreed after held radio or focus edge cases. | browser.jr records shared checked-radio key phases. It requires the original target to own focus at key-up. | Chromium and WebKit omitted checked-radio `click`; Firefox included it. Chromium canceled after focus left and returned; Firefox and WebKit activated, 2026-08-31. |
| BJR-008 | `agent-browser` Lightpanda omitted held button `Space` activation. | browser.jr follows the shared Playwright native timing and keeps the Lightpanda difference visible. | `agent-browser` 0.32.4 recorded only `keydown` and `keyup`. Playwright Chromium, Firefox, and WebKit recorded keypress and key-up activation, 2026-08-31. |
