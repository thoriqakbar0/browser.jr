# Browser and JavaScript engine architecture research

Research date: 31 August 2026

## Question

Was there a paper behind browser.jr's current layout engine, and what browser-engine or JavaScript-engine research should guide its next architecture?

The first question has two answers. browser.jr's proposed incremental layout design comes from *Spineless Traversal for Layout Invalidation*. Its current JavaScript architecture does not come from one paper because browser.jr does not execute JavaScript yet.

The more important boundary is easy to miss. A JavaScript engine implements ECMAScript. It does not provide a browser's Web IDL bindings, DOM, HTML task model, resource loading, mutation rules, or layout scheduling.

## Short answer

There is no single paper that specifies how browser.jr should add JavaScript. Three sources materially shape the decision:

1. *Cross-Component Garbage Collection* explains the hardest ownership problem between a JavaScript heap and a browser heap.
2. *Engineering the Servo Web Browser Engine Using Rust* shows why Rust helps a browser engine, but does not remove JavaScript integration work.
3. *The Security Architecture of the Chromium Browser* explains why language safety and process isolation solve different problems.

The standards supply the actual integration contract. ECMAScript defines the language runtime and host hooks. Web IDL defines JavaScript-facing web APIs. DOM defines nodes, events, and mutation observation. HTML connects realms, tasks, microtasks, modules, loading, and rendering opportunities.

The research recommendation is to keep browser.jr as the Rust browser host. It should eventually use one production JavaScript engine for page realms and tooling realms. The current evidence favors a SpiderMonkey and `mozjs` investigation first, with the same small page-script spike built against V8 and `rusty_v8` before selection.

This is a research recommendation, not a product decision. It does not make JavaScript the next implementation slice.

Boa and QuickJS can probe the host contract without defining the production choice. They should remain disposable probes, not a second permanent runtime. Nova remains a research watchlist item. JavaScriptCore does not currently offer a stronger Rust-browser path than SpiderMonkey or V8.

## Facts and inferences

Each section separates source facts from browser.jr recommendations. "The source states" reports primary evidence. "Inference" applies that evidence to this project.

## Was there a paper?

### Cross-Component Garbage Collection

Ulan Degenbaev, Jochen Eisinger, Kentaro Hara, Marcel Hlopko, Michael Lippautz, and Hannes Payer. 2018. *Cross-Component Garbage Collection*. Proceedings of the ACM on Programming Languages 2, OOPSLA, article 151, pages 1-24. [Google Research publication and paper](https://research.google/pubs/cross-component-garbage-collection/).

Publication status: peer-reviewed OOPSLA paper.

What it does:

- The paper studies objects that reference each other across two separately managed heaps.
- It identifies leaks from cross-heap cycles and dangling references from premature reclamation.
- It presents cross-component tracing in Chrome, where V8 is embedded in Blink.

Evidence and limits:

- The implementation joined reachability across V8's JavaScript heap and Blink's C++ heap.
- The paper reports lower garbage-collection latency and removal of long-standing leaks on real sites.
- The result does not prescribe a Rust DOM representation or a general Web IDL layer.
- The implementation evidence comes from V8 and Blink, not browser.jr.

Inference for browser.jr:

- Garbage collection is an architecture boundary, not an adapter detail.
- A JavaScript wrapper and its Rust DOM object need one explicit liveness protocol.
- A generic `JsValue` trait cannot hide tracing, rooting, finalization, weak references, and cross-heap cycles safely.
- Engine selection must include a retained-object and forced-GC test, not only script conformance.

### Engineering the Servo Web Browser Engine Using Rust

Brian Anderson, Lars Bergstrom, Manish Goregaokar, Josh Matthews, Keegan McAllister, Jack Moffitt, and Simon Sapin. 2016. *Engineering the Servo Web Browser Engine Using Rust*. ICSE Companion 2016, pages 81-89. [DOI 10.1145/2889160.2889229](https://doi.org/10.1145/2889160.2889229). [Servo's research index](https://github.com/servo/servo/wiki/Browser-Engine-Research).

Publication status: peer-reviewed ICSE Companion experience paper.

What it does:

- The paper explains Servo's use of Rust for memory safety and parallel browser work.
- It reports engineering patterns and problems from building a new engine in Rust.
- It treats the browser engine as a system of cooperating subsystems, not a JavaScript virtual machine.

Evidence and limits:

- The paper supports Rust ownership and typed concurrency as browser-engine tools.
- It predates Servo's current SpiderMonkey integration report and later Web IDL cleanup.
- It does not compare current embeddable JavaScript engines.
- It does not define browser.jr's DOM, event loop, or layout boundaries.

Inference for browser.jr:

- Keep DOM, loading, layout, and evidence ownership in Rust.
- Do not infer that a Rust host makes foreign-engine integration safe by itself.
- Keep unsafe engine handles inside a small binding boundary.

### The Security Architecture of the Chromium Browser

Adam Barth, Collin Jackson, Charles Reis, and the Google Chrome Team. 2008. *The Security Architecture of the Chromium Browser*. Stanford technical report. [Author-hosted paper](https://seclab.stanford.edu/websec/chromium/chromium-security-architecture.pdf).

Publication status: primary architecture report, not a peer-reviewed paper.

What it does:

- The report separates a privileged browser kernel from a restricted renderer process.
- The renderer contains risky web-facing components, including the HTML parser, JavaScript VM, and DOM.
- The browser kernel brokers operating-system and persistent-resource access.

Evidence and limits:

- The paper evaluates exploit containment against its 2008 threat model.
- Current Chromium uses a more detailed multi-process model.
- Process isolation does not define Web IDL behavior, garbage collection, or deterministic scheduling.

Inference for browser.jr:

- Rust memory safety and capability checks do not sandbox untrusted page code.
- Keep network, file, storage, and external actions behind session-owned capabilities.
- Treat process isolation as a future security boundary before loading hostile remote pages.

### What Spineless Traversal covers

Marisa Kirisame, Tiezhi Wang, and Pavel Panchekha. 2025. *Spineless Traversal for Layout Invalidation*. PLDI 2025. [DOI 10.1145/3729322](https://doi.org/10.1145/3729322). The existing [browser-engine research note](browser-engine-and-design-lint-papers.md#spineless-traversal-for-layout-invalidation-1) analyzes it in detail.

Spineless Traversal informs browser.jr's proposed field dependencies, legal recomputation order, dirty state, and incremental layout work. It does not define script execution, DOM wrappers, the HTML event loop, or JavaScript garbage collection.

## Current browser.jr boundary

These are repository facts at commit `2b86278b0051e578e520f63d325d57e89ac6d306`. Uncommitted work in the shared tree may extend other features.

- [README.md](../README.md) says the Rust engine loads bounded loopback HTML and computes a stated layout subset.
- It records JavaScript execution and the REPL as unimplemented.
- Current page actions change native Rust state. They do not dispatch DOM events.
- [architecture.md](../architecture.md) proposes `Session`, page, document, layout, snapshot, rule, and capability ownership.
- That architecture remains a design document where the source does not yet implement its types.

Therefore, this note does not describe an existing JavaScript integration. It describes evidence for a future boundary.

## What a JavaScript engine does not supply

The [ECMAScript specification](https://tc39.es/ecma262/2026/multipage/) defines language execution, objects, realms, jobs, and host hooks. It leaves browser behavior to the host.

The [Servo and SpiderMonkey report](https://github.com/servo/servo/wiki/Servo-and-SpiderMonkey-Report) states the boundary directly. SpiderMonkey implements ECMAScript and WebAssembly. Servo implements the web platform and integrates SpiderMonkey through Rust bindings, generated bindings, and manual browser code.

| Concern | JavaScript engine | browser.jr host |
| --- | --- | --- |
| ECMAScript syntax and execution | Owns | Calls through a bounded engine API |
| JavaScript objects, realms, promises, and garbage collection | Owns runtime mechanics | Selects realm policy and supplies host hooks |
| Web IDL conversions and interface objects | Supplies engine primitives | Implements or generates the binding contract |
| DOM nodes, attributes, events, and mutation observers | Does not supply | Owns |
| HTML parsing and script preparation | Does not supply | Owns |
| Task sources and task queues | Does not supply | Owns |
| Microtask checkpoints | Runs queued jobs when requested | Schedules checkpoints under HTML rules |
| Module URL resolution, fetch, and module maps | Exposes module hooks | Owns loading and caching policy |
| Network, file, storage, and external effects | Does not authorize | Owns capabilities and brokers effects |
| Style invalidation and layout scheduling | Does not supply | Owns |
| Stable inspection identities and snapshots | Does not supply | Owns |

The table is an architectural reading of the specifications. It is not an engine feature comparison.

## Standards-defined integration points

### Realms, globals, and page lifetime

Source facts:

- ECMAScript defines a realm as a set of intrinsic objects plus a global object and global environment.
- HTML connects realms to environment settings objects and browser globals.
- Web IDL defines interface objects, prototypes, platform objects, and their associated realms.
- V8 exposes `Isolate` as an engine instance with its own heap and `Context` as an execution environment. See the [V8 embed guide](https://v8.dev/docs/embed).

Inference for browser.jr:

- Each page generation needs an explicit realm lifecycle.
- Navigation must invalidate old realm handles, DOM wrappers, callbacks, and module records together.
- A tooling REPL should use the chosen production engine unless later evidence proves a separate runtime necessary.
- Tooling code needs a distinct realm and authority set, even when it shares an engine implementation.

### Web IDL bindings and object identity

Source facts:

- [Web IDL](https://webidl.spec.whatwg.org/) defines how web-platform types appear to ECMAScript.
- Its algorithms cover overloads, argument conversion, exceptions, interface objects, prototypes, and platform objects.
- Platform objects carry an associated realm and primary interface.
- `[SameObject]` requires repeated getter calls to return the same JavaScript value.
- Conversion from an interface value returns the corresponding platform-object reference.
- Servo generates much binding code and keeps shared manual utilities under `components/script/dom/bindings`.
- WebKit documents one JavaScript wrapper per native object in each wrapper world. See [WebKit JavaScript wrappers](https://docs.webkit.org/Deep%20Dive/Architecture/JSWrappers.html).

Inference for browser.jr:

- A `NodeId` alone does not satisfy JavaScript object identity.
- The binding layer needs one wrapper registry per realm or wrapper world.
- A live wrapper should point to a generation-checked DOM identity, not a layout box or snapshot node.
- Web IDL conversion and exception behavior should come from generated tables where practical.
- Layout snapshots must stay immutable. Scripts mutate the live document, then browser.jr creates new evidence.

### Tasks, microtasks, events, and layout

Source facts:

- The [HTML event-loop algorithms](https://html.spec.whatwg.org/multipage/webappapis.html#event-loops) give each agent an event loop.
- An event loop has task queues. The microtask queue is separate from those task queues.
- The loop selects a runnable task, performs a microtask checkpoint, then may update rendering.
- HTML's [`HostEnqueuePromiseJob`](https://html.spec.whatwg.org/multipage/webappapis.html#hostenqueuepromisejob) maps ECMAScript promise jobs into HTML microtasks.
- The [DOM dispatch algorithm](https://dom.spec.whatwg.org/#concept-event-dispatch) invokes event listeners and defines propagation behavior.
- DOM mutation observers notify through the microtask mechanism. See [DOM mutation observers](https://dom.spec.whatwg.org/#mutation-observers).
- V8 exposes microtask policies and an explicit checkpoint. See [`v8::MicrotasksPolicy`](https://chromium.googlesource.com/v8/v8.git/+/HEAD/include/v8-microtask.h).
- SpiderMonkey lets the embedding supply a job queue through `JS::SetJobQueue`. See the [SpiderMonkey JSAPI source](https://searchfox.org/mozilla-central/source/js/src/jsapi.cpp).

Inference for browser.jr:

- The Rust host must own the event loop and readiness policy.
- The engine must not decide when a page is settled for inspection.
- A task can mutate the DOM, queue promise work, and trigger more mutation before layout becomes observable.
- browser.jr should perform a microtask checkpoint before it considers a task settled.
- browser.jr should then apply style and layout invalidation before it freezes a snapshot.
- Current native actions should not masquerade as web events when JavaScript arrives.

A useful initial order is:

```text
select one runnable page task
  -> run script or native event dispatch
  -> perform the microtask checkpoint
  -> collect resulting DOM and style invalidations
  -> run clean layout for the first proof
  -> freeze an immutable evidence snapshot
```

This order is a browser.jr recommendation. Full HTML rendering opportunities require more conditions than this first proof.

### Modules and resource loading

Source facts:

- ECMAScript defines module records and evaluation.
- Its [`HostLoadImportedModule`](https://tc39.es/ecma262/2026/multipage/ecmascript-language-scripts-and-modules.html#sec-HostLoadImportedModule) operation is host-defined.
- The host must return a stable module record for repeated loads of the same referencing record and specifier.
- [HTML module scripts](https://html.spec.whatwg.org/multipage/webappapis.html#integration-with-the-javascript-module-system) add base URLs, credentials, fetch, module types, module maps, and import maps.
- V8 and SpiderMonkey expose callbacks for the host parts. They do not choose browser fetch policy.

Inference for browser.jr:

- The page owns a module map keyed by resolved URL and module type.
- Module fetches must use the same bounded loader and capability policy as document loads.
- Deterministic fixtures need recorded bytes, final URLs, redirects, content types, and load outcomes.
- The first page-script spike should use one classic inline script. Modules should follow only after task integration works.

### Garbage collection and Rust ownership

Source facts:

- V8 uses GC-visible handles and handle scopes. Embedders root retained values with persistent handles.
- SpiderMonkey uses rooted handles and tracing APIs. Its [GC documentation](https://firefox-source-docs.mozilla.org/js/gc.html) describes precise, incremental, generational, compacting collection.
- Servo's [DOM documentation](https://doc.servo.org/script/dom/) pairs a Rust DOM object with a SpiderMonkey reflector.
- Servo lets the JavaScript collector participate in Rust DOM lifetime and supplies custom tracing utilities.
- WebKit combines reference-counted native objects with JavaScript wrappers and tracing rules.

Inference for browser.jr:

- Choose one authoritative liveness model before exposing any DOM object to script.
- Keep all engine-managed references in typed rooted wrappers.
- Do not store raw engine pointers in `Session`, layout, snapshots, or rules.
- Test JavaScript-to-DOM, DOM-to-JavaScript, and cyclic references under forced collection.
- Treat finalizers as cleanup signals, not as deterministic browser events.

### Interrupts, budgets, and failure states

Source facts:

- V8 exposes `TerminateExecution` and `RequestInterrupt` in [`v8::Isolate`](https://chromium.googlesource.com/v8/v8.git/+/HEAD/include/v8-isolate.h).
- SpiderMonkey exposes an interrupt callback through [`JS_AddInterruptCallback`](https://searchfox.org/mozilla-central/source/js/public/Interrupt.h).
- QuickJS exposes memory, stack, and interrupt limits in its [embedding API](https://bellard.org/quickjs/quickjs.html).
- Boa exposes [runtime limits](https://docs.rs/boa_engine/latest/boa_engine/vm/struct.RuntimeLimits.html) for loops, recursion, stack size, and backtraces.

Inference for browser.jr:

- Give each script turn explicit time, cancellation, recursion, and memory policy.
- Return `timed_out`, `cancelled`, `resource_limit`, or `engine_failure` as distinct results.
- Test recovery after interruption. A terminated realm must not silently corrupt later inspection.
- Process isolation remains necessary for hostile input because in-process limits are not a security sandbox.

### Determinism

Source facts:

- ECMAScript leaves time-zone behavior and random-number generation implementation-defined. It supplies host hooks for job scheduling and module loading.
- HTML scheduling includes user-agent choice, task sources, microtasks, timers, and rendering opportunities.
- Web content can observe time, randomness, resource timing, and task order.

Inference for browser.jr:

- Determinism is a host policy, not a JavaScript-engine feature switch.
- A deterministic mode needs a virtual clock, seeded randomness, recorded resources, explicit task ordering, and fixed environment data.
- The snapshot should record the script budget, completed task boundary, and remaining runnable work.
- If pending work can change inspected evidence, return `unstable` or `indeterminate` instead of a clean result.

### Isolation and authority

Source facts:

- The Chromium security report places web-facing code in a restricted renderer process.
- Chromium's current [multi-process architecture](https://www.chromium.org/developers/design-documents/multi-process-architecture/) separates browser and renderer responsibilities.
- WebKit2 uses a sandboxed WebContent process and brokered system access. See the [WebKit2 architecture](https://docs.webkit.org/Deep%20Dive/Architecture/WebKit2.html).
- A JavaScript realm isolates language globals. It is not an operating-system sandbox.

Inference for browser.jr:

- Keep current loopback-only loading while JavaScript runs in process.
- Do not expose file, network, process, clipboard, or credential APIs through generic host functions.
- A future remote-page mode should move parsing, DOM, JavaScript, and layout into a restricted worker process.
- The privileged session should broker every externally visible effect.

## Candidate engine evidence

The license column summarizes upstream declarations. It is not legal advice. A release needs a dependency and notice review.

| Candidate | Primary embedding evidence | Main browser.jr concern | Upstream license surface | Research position |
| --- | --- | --- | --- | --- |
| SpiderMonkey through `mozjs` | Firefox JSAPI, Servo's Rust integration, generated Web IDL, DOM tracing | Large C++ dependency and SpiderMonkey-specific unsafe APIs | [`mozjs` declares MPL-2.0](https://github.com/servo/mozjs/blob/main/Cargo.toml) | Leading research candidate for the matched spike |
| V8 through `rusty_v8` | Mature embed API, isolates, contexts, host callbacks, interrupts | Large binary and build; browser DOM integration still belongs to browser.jr | [V8 uses BSD-style terms plus third-party notices](https://github.com/v8/v8/blob/main/LICENSE); [`rusty_v8` uses MIT](https://github.com/denoland/rusty_v8/blob/main/LICENSE) | Required comparison candidate |
| JavaScriptCore | WebKit C API, contexts, protected values, WebKit wrapper design | Public Rust-browser integration path is less developed | [JavaScriptCore has mixed per-file terms](https://github.com/WebKit/WebKit/blob/main/Source/JavaScriptCore/COPYING.LIB), including LGPL and BSD terms | Do not prioritize without a deployment reason |
| QuickJS | Small C embedding API, realms, modules, limits, reference counting | No browser-grade Rust DOM binding or cross-heap evidence | [MIT](https://github.com/bellard/quickjs/blob/master/LICENSE) | Disposable contract probe only |
| Boa | Native Rust `Context`, host hooks, job executor, loader, runtime limits | Project describes itself as experimental; no production browser integration | [MIT or Unlicense](https://github.com/boa-dev/boa/blob/main/Cargo.toml) | Disposable contract probe only |
| Nova | Native Rust engine with embedding goals | Project lists major language limitations and lacks WebAssembly | [MPL-2.0](https://github.com/trynova/nova/blob/main/LICENSE.md) | Watchlist only |

### SpiderMonkey and `mozjs`

Primary sources:

- [SpiderMonkey documentation](https://firefox-source-docs.mozilla.org/js/) describes the engine and JSAPI.
- Servo's [`mozjs` repository](https://github.com/servo/mozjs) provides low-level and higher-level Rust bindings.
- The [Servo integration report](https://github.com/servo/servo/wiki/Servo-and-SpiderMonkey-Report) documents generated bindings, manual utilities, GC integration, and unsafe coupling.

Source facts:

- Servo already integrates SpiderMonkey with a Rust DOM and Web IDL-generated bindings.
- Servo also calls low-level JSAPI from browser code, which created tight coupling and unsafe call sites.
- Servo recommends centralizing low-level APIs behind safe Rust concepts and Web IDL-shaped interfaces.

Inference for browser.jr:

- This is the strongest primary evidence for a Rust browser host with a production JavaScript engine.
- Reuse lessons and binding patterns, not Servo's entire script subsystem.
- Keep SpiderMonkey-specific handles below one `script::engine` and `script::bindings` boundary.

### V8 and `rusty_v8`

Primary sources:

- The [V8 embed guide](https://v8.dev/docs/embed) covers isolates, contexts, handles, and host functions.
- The [`v8::Isolate` API](https://chromium.googlesource.com/v8/v8.git/+/HEAD/include/v8-isolate.h) exposes interrupts, termination, promise hooks, and module callbacks.
- [`rusty_v8`](https://github.com/denoland/rusty_v8) supplies Rust bindings and prebuilt archives for supported releases.

Source facts:

- V8 provides the embedding primitives needed for realms, native callbacks, promises, modules, and interruption.
- `rusty_v8` packages those primitives for Rust. It does not provide DOM or Web IDL bindings.
- V8's handle model requires exact rooting and scope discipline.

Inference for browser.jr:

- V8 is the necessary control because its embedding API and deployment history are strong.
- Compare integration size, retained-object correctness, interrupts, binary size, and build reliability.
- Do not select from standalone JavaScript speed benchmarks.

### JavaScriptCore

Primary sources:

- [JavaScriptCore architecture](https://docs.webkit.org/Deep%20Dive/JSC/JavaScriptCore.html) describes WebKit's ECMAScript engine.
- [`JSContextRef.h`](https://github.com/WebKit/WebKit/blob/main/Source/JavaScriptCore/API/JSContextRef.h) and [`JSValueRef.h`](https://github.com/WebKit/WebKit/blob/main/Source/JavaScriptCore/API/JSValueRef.h) define the C API.
- [WebKit wrapper documentation](https://docs.webkit.org/Deep%20Dive/Architecture/JSWrappers.html) explains native object identity and reachability.

Inference for browser.jr:

- JavaScriptCore remains technically viable.
- It lacks the first-party Rust browser integration evidence that Servo supplies for SpiderMonkey.
- Reconsider it only if Apple-platform distribution or another concrete constraint changes the comparison.

### QuickJS, Boa, and Nova

Primary sources:

- The [QuickJS manual](https://bellard.org/quickjs/quickjs.html) documents its C API, contexts, modules, limits, and collector.
- The [Boa repository](https://github.com/boa-dev/boa) and its [`HostHooks`](https://docs.rs/boa_engine/latest/boa_engine/context/trait.HostHooks.html), [`JobExecutor`](https://docs.rs/boa_engine/latest/boa_engine/job/trait.JobExecutor.html), and [`ModuleLoader`](https://docs.rs/boa_engine/latest/boa_engine/module/trait.ModuleLoader.html) APIs document a Rust host surface.
- The [Nova repository](https://github.com/trynova/nova) lists its embedding goal and current limitations.

Inference for browser.jr:

- QuickJS or Boa can reveal flaws in a proposed host contract without defining the production choice.
- Throw away that probe after the comparison. Do not ship two permanent engines by default.
- Nova's explicit conformance gaps make it unsuitable for page realms now.

## Research recommendation

### Keep one browser host and one eventual JavaScript engine

browser.jr should own these components in Rust:

- page and realm lifecycle
- live DOM identity and mutation
- Web IDL binding definitions and generated glue
- task sources, task queues, and microtask checkpoints
- module resolution, fetching, and module maps
- style invalidation, layout scheduling, and snapshots
- capabilities, budgets, cancellation, and evidence

The selected engine should remain an internal implementation. It should own ECMAScript execution, JavaScript values, promise-job representation, and its heap. The Rust host should decide when HTML rules enqueue and drain those jobs.

One eventual engine should serve page and tooling realms. A split engine would duplicate semantics, security review, build work, tests, and debugging. Adopt two engines only after measured evidence proves that duplication necessary.

### Investigate SpiderMonkey, then compare V8 on the same proof

SpiderMonkey is the leading research candidate because Servo supplies first-party Rust-browser integration evidence. This is not a final product decision.

V8 remains a mandatory comparison. Its embedding surface, termination support, and deployment history can outweigh Servo's closer architectural precedent.

Do not design a broad, engine-neutral trait before either proof. Centralize raw engine calls and expose browser concepts above them. The hard join points will remain engine-specific.

### Do not choose from JIT internals

JIT tiering does not decide browser.jr's present architecture. Embedding correctness matters first:

- Web IDL behavior
- object identity and cross-heap liveness
- host-controlled tasks and microtasks
- module loading hooks
- reliable interruption and recovery
- isolation and capability boundaries
- build, binary, and license cost

## Smallest meaningful page-script spike

The comparison should build the same behavior twice, once with `mozjs` and once with `rusty_v8`.

The fixture contains:

```html
<button id="grow">grow</button>
<div id="target" style="width: 40px">start</div>
<script>
  const target = document.getElementById("target");
  document.getElementById("grow").addEventListener("click", () => {
    target.textContent = "changed";
    Promise.resolve().then(() => {
      target.style.width = "80px";
    });
  });
</script>
```

The host must support only the browser surface needed by this fixture:

1. Create one page realm and global object.
2. Expose `document`, `getElementById`, `textContent`, `style.width`, and event listeners.
3. Preserve one JavaScript wrapper per DOM node in that realm.
4. Dispatch one native `click` through the DOM event path.
5. Queue the promise reaction as a microtask.
6. Perform the microtask checkpoint before inspection settles.
7. Recompute clean layout and freeze a new snapshot.
8. Destroy the page, force collection, and prove no old wrapper remains usable.

The spike excludes modules, external scripts, timers, networking, watch mode, incremental layout, WebAssembly, and a general REPL.

## Comparison tests

Both candidates must pass the same black-box tests.

### Browser behavior

- The inline script runs once in its page realm.
- `getElementById` returns the same JavaScript object on repeated calls.
- Event dispatch invokes the listener with the correct target and `this` value.
- The synchronous text mutation appears before the promise reaction.
- The microtask width mutation appears before the settled snapshot.
- The final fragment width is 80 pixels under the supported layout subset.
- The prior immutable snapshot still reports 40 pixels.

### Lifetime and garbage collection

- A rooted wrapper survives forced collection.
- An unreachable JavaScript-to-DOM cycle is reclaimed.
- A DOM-to-JavaScript listener cycle is reclaimed.
- Navigation invalidates callbacks and wrappers from the old generation.
- Finalization cannot mutate a frozen snapshot.

### Failure and recovery

- An infinite loop reaches the host interrupt and returns `timed_out`.
- Cancellation produces a distinct `cancelled` result.
- An exception returns its name, message, and stable source location.
- A failed script turn does not expose a half-frozen snapshot.
- A fresh page still runs after the previous realm terminates.

### Engineering cost

- Record clean build time, incremental build time, release binary size, and dependency size.
- Count binding code, unsafe lines, raw engine call sites, and generated code.
- Record supported targets, archive provenance, update cadence, and security-release process.
- Audit license files, required notices, and source-distribution obligations.

Use selected Test262 cases for language-host hooks. Use focused Web Platform Tests for Web IDL, events, promises, and node identity. browser.jr's fixture remains the end-to-end acceptance test.

## Decision record after the spike

Choose one engine only after both proofs produce evidence for:

1. Correct browser behavior at the same host boundary.
2. A safe and reviewable Rust ownership model.
3. Reliable interruption and post-failure recovery.
4. Reproducible builds on browser.jr's supported systems.
5. Acceptable binary, update, security, and license cost.

Do not preserve the losing prototype as a supported backend. Keep its measurements and remove its product path.

## Evidence limits

- No candidate has been compiled or benchmarked inside browser.jr for this note.
- No browser.jr Web IDL generator or DOM wrapper design exists yet.
- Engine APIs and release packaging can change after the research date.
- Primary documentation establishes available mechanisms, not integration quality in this repository.
- The license summary is an engineering screen, not legal advice.
- SpiderMonkey leads this research review. Only a matched implementation spike can settle the choice.
