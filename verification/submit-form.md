# Verification: submit a form

Run these checks against controlled loopback pages. Record each fixture and browser.jr commit.

## interaction/submit-form.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SUBMIT-01 | P1 | package | Submit-button click performs one same-context GET ([The simple case](../interaction/submit-form.md#the-simple-case)). | Serve one named input, submitter, and destination. | Fill, click, and capture the request. | Navigation loads the encoded action URL. | partial: package and compiled-process tests passed, 2026-08-31 |
| SUBMIT-02 | P1 | package | Current supported controls form the query ([While running](../interaction/submit-form.md#while-running)). | Serve text, textarea, hidden, checked, selected, disabled, and unnamed controls. | Mutate supported values, submit, and capture the request. | Current successful values appear once in document order. | partial: package test passed, 2026-08-31 |
| SUBMIT-03 | P1 | package | Encoding uses plus, CRLF, and UTF-8 percent escapes ([While running](../interaction/submit-form.md#while-running)). | Serve named text and textarea controls. | Store spaces, Unicode, and line feeds, then submit. | The request uses form URL encoding and CRLF normalization. | partial: package test passed, 2026-08-31 |
| SUBMIT-04 | P1 | package | Submitter overrides select action and method ([Begin running](../interaction/submit-form.md#begin-running)). | Serve a POST form with a GET submitter override. | Press Enter on the submitter. | The override action receives one GET. | partial: package test passed, 2026-08-31 |
| SUBMIT-05 | P1 | package | External controls use exact form ownership ([While running](../interaction/submit-form.md#while-running)). | Serve a form ID and one external named control. | Submit through Enter and Space. | Both requests include the external control. | partial: package test passed, 2026-08-31 |
| SUBMIT-06 | P1 | pipe | Session click reports navigation and re-resolves the destination ([Finish](../interaction/submit-form.md#finish)). | Serve a form and named destination heading. | Fill by CSS, click by role, then find the heading. | Output reports encoded navigation. Destination resolution succeeds. | partial: parser and compiled-process tests passed, 2026-08-31 |
| SUBMIT-07 | P1 | pipe | Locator Enter reports the same navigation effect ([Invoke](../interaction/submit-form.md#invoke)). | Serve a form with one submit button. | Press Enter by exact role, then find destination text. | Output reports submitter identity and encoded URL. | partial: package and compiled-process tests passed, 2026-08-31 |
| SUBMIT-08 | P1 | package | Unsupported modes preserve current state ([Exit immediately](../interaction/submit-form.md#exit-immediately)). | Serve POST, file, and remote-action forms. | Submit each, then reuse a current reference. | Each typed failure preserves the page and reference. | partial: package test passed, 2026-08-31 |
| SUBMIT-09 | P1 | package | Existing action query fields remain first ([While running](../interaction/submit-form.md#while-running)). | Give the action one existing query field. | Submit named current controls. | Existing data precedes serialized form entries. | partial: package and controlled Chromium evidence passed, 2026-08-31 |
| SUBMIT-10 | P1 | package | Success invalidates old references and records history ([Finish](../interaction/submit-form.md#finish)). | Serve one form and destination. | Capture, submit, then reuse the reference and go back. | The reference is stale. Back loads the form URL. | partial: package test passed, 2026-08-31 |
| SUBMIT-11 | P1 | pipe | Filled text controls submit implicitly through Enter ([Invoke](../interaction/submit-form.md#invoke)). | Serve one GET form with two submitters. | Fill by reference, then press Enter. | The first submitter supplies overrides and its ordered entry. | partial: package and compiled-process tests passed, 2026-08-31 |
| SUBMIT-12 | P1 | package | No-button forms obey the blocking-field count ([Begin running](../interaction/submit-form.md#begin-running)). | Serve forms with one and two blocking inputs. | Press Enter on one input in each form. | One input navigates without a submitter. Two inputs report `Ignored`. | partial: package test and controlled standard comparison passed, 2026-08-31 |
| SUBMIT-13 | P1 | package | A disabled default submitter prevents implicit navigation ([Begin running](../interaction/submit-form.md#begin-running)). | Put a disabled submitter before an enabled submitter. | Press Enter on the form's text control. | The press reports `Ignored`. Current page state remains usable. | partial: package test passed, 2026-08-31 |

Not checkable yet:

- Validation, submit-event records, and page-script event delivery do not exist.
- POST, file upload, image coordinates, and alternate encodings do not exist.
- Complete input-type and disabled-fieldset conformance do not exist.
- Auto-waiting, redirects, and action timeouts do not exist.
