# Browser engine and design lint research

Research date: 27 August 2026

## Question

What should browser.jr learn from Lightpanda and prior research on browser engines, layout computation, responsive failures, cross-browser testing, and structured visual evidence?

browser.jr needs a smaller target than a general browser. It must load a page, compute enough CSS layout to inspect the result, and return findings that explain measurable design failures. The first version does not need pixels.

## Short answer

Lightpanda is useful because it shows how to make one machine-oriented engine serve a CLI, protocol clients, scripts, and agents. Its decisive optimization cannot carry over. Lightpanda omits style resolution and real layout, while browser.jr needs both.

The strongest research path combines four ideas:

1. Stop the rendering pipeline after a queryable fragment tree. Do not build paint, compositing, screenshots, or a browser window in the first version.
2. Represent responsive layout as relationships that vary over viewport intervals. Use interval search around changes, as ReDeCheck does.
3. Express design rules as predicates over boxes, fragments, computed styles, and relationships. Keep the rule result and its evidence separate.
4. Add visual confirmation later for findings that geometry alone cannot classify. Viser shows why this second stage matters.

The practical first implementation should be a Rust engine with a small supported CSS set, deterministic snapshots, WPT-backed conformance, and built-in spatial rules. Initial page layout may recompute everything. The first watch-mode implementation uses Spineless Traversal for incremental invalidation.

## Facts and inferences

Each source below separates reported facts from the recommendation for browser.jr. "The paper reports" means the claim comes from the cited source. "Inference" means this document applies that evidence to browser.jr.

## Latest work, 2024-2026

Recent work is useful, but it does not replace the older foundations. The strongest recent papers improve incremental layout, failure localization, test stability, renderer fuzzing, and rule presentation. ReDeCheck, VizAssert, Cassius, and X-PERT still define the more direct algorithms for responsive relationships, layout predicates, formal CSS reasoning, and alignment graphs.

### An Empirical Study of Web Visual Flakiness: Characterisation and Fix Strategies

Yu Pei, Jeongju Sohn, and Mike Papadakis. 2026. Journal of Systems and Software 237, article 112826. [DOI 10.1016/j.jss.2026.112826](https://doi.org/10.1016/j.jss.2026.112826). [Institutional full-text record](https://orbilu.uni.lu/handle/10993/68138).

Publication status: peer-reviewed journal article, published in 2026.

What it does:

- The paper studies visual flakiness, where identical web test intent produces unstable structure, layout, or style across runs.
- It collects 262 cases. The set includes 144 cases from 31 open-source projects and 118 Chromium cases.
- It classifies 59.9 percent as structure-related and 40.1 percent as style-related.

Evidence and limits:

- This is an empirical taxonomy and repair study. It does not propose a layout engine or a visual oracle.
- Its cases come from reported and fixed failures. Their distribution may differ from a new engine's failures.
- The categories show what makes test evidence unstable. They do not decide which visual output is correct.

Inference for browser.jr:

- A lint run needs a stability gate before it reports a regression.
- Record font inputs, resource completion, animation state, viewport, scale, and event completion with every snapshot.
- Add `unstable` beside pass, fail, unsupported, and indeterminate. Do not turn inconsistent reruns into a design finding.

### RTFuzz: Fuzzing Browsers via Efficient Render Tree Mutation

Yishun Zeng, Yue Wu, Xicheng Lu, and Chao Zhang. 2026. Computers & Security 161, article 104756. [DOI 10.1016/j.cose.2025.104756](https://doi.org/10.1016/j.cose.2025.104756). [Publisher record](https://www.sciencedirect.com/science/article/pii/S0167404825004456).

Publication status: peer-reviewed journal article in the February 2026 issue. The DOI contains 2025 because the publisher registered it before the issue year.

What it does:

- RTFuzz mutates DOM and CSSOM inputs to exercise browser render trees.
- It prunes low-yield HTML and CSS combinations using observed correlations.
- It spreads mutations across rendering cycles so browser update coalescing cannot hide deeper layout and paint execution.

Evidence and limits:

- The paper reports 20 real defects from long-running tests.
- Against Domato, FreeDom, and Minerva, it reports 43.1, 28.7, and 75.7 percent more unique crashes. It says 83.3 percent of its unique crashes occur in the rendering pipeline.
- Its oracle is robustness and crash behavior, not layout conformance or design intent.
- The implementation drives Chromium, Firefox, and WebKit with Selenium. It does not provide a reusable Rust layout component.

Inference for browser.jr:

- Generate conformance cases around combinations of HTML structure, CSS properties, and timed DOM changes.
- Force separate rendering cycles when testing invalidation. One final snapshot cannot test every intermediate update.
- Use crash fuzzing and semantic differential testing as separate suites.

### MIXVRT: MIX Visual Regression Testing Tool Which Mixes Image Comparison and HTML Code Comparison

Naoki Aridome, Nobuya Takahashi, Tetsuro Katayama, Yoshihiro Kita, Hisaaki Yamaba, Kentaro Aburada, and Naonobu Okazaki. 2026. Journal of Robotics, Networking and Artificial Life 11(3), pages 214-219. [DOI 10.57417/jrnal.11.3_214](https://doi.org/10.57417/jrnal.11.3_214). [Publisher paper](https://www.jstage.jst.go.jp/article/jrnal/11/3/11_8/_pdf).

Publication status: peer-reviewed open-access journal article, published in 2026.

What it does:

- MIXVRT combines screenshot differencing with HTML source comparison.
- It highlights image differences and the related changed HTML so a tester can locate layout defects faster.

Evidence and limits:

- The publisher reports faster defect location than image-only comparison, with no missed or extra detections in its experiment.
- The article is six pages. Its public metadata does not establish broad evaluation across sites, browsers, fonts, or dynamic pages.
- Source-text changes do not provide computed-style or layout provenance.

Inference for browser.jr:

- Pair every visual region with structured source evidence.
- Prefer computed declarations and fragment provenance over raw HTML diffs.
- Treat MIXVRT as confirmation that mixed evidence helps people. Do not treat its evaluation as a sufficient oracle design.

### Spineless Traversal for Layout Invalidation

Marisa Kirisame, Tiezhi Wang, and Pavel Panchekha. 2025. PLDI 2025. [DOI 10.1145/3729322](https://doi.org/10.1145/3729322).

Publication status: peer-reviewed conference paper, published in 2025. A full analysis appears under [engine and layout papers](#spineless-traversal-for-layout-invalidation-1).

Current implication:

- Store layout-field dependencies and recomputation order from the first implementation.
- Use its priority-queue invalidation when watch mode begins.
- Keep clean full layout as a differential correctness oracle and recovery path.

### A Heuristic Approach to Localize CSS Properties for Responsive Layout Failures

Tasmia Zerin, B. M. Mainul Hossain, and Kazi Sakib. 2025. 20th International Conference on Evaluation of Novel Approaches to Software Engineering, pages 292-303. [DOI 10.5220/0013477500003928](https://doi.org/10.5220/0013477500003928). [Publisher paper](https://www.scitepress.org/Papers/2025/134775/134775.pdf).

Publication status: peer-reviewed ENASE conference paper, published in 2025. The authors uploaded an arXiv copy in 2026, but the publication year remains 2025.

What it does:

- LocaliCSS starts with a detected responsive layout failure and its affected elements.
- It uses failure direction and relative alignment to find nearby elements.
- It ranks element and CSS-property pairs that may have caused the failure.

Evidence and limits:

- The paper reports 45.2 percent top-1 and 92.86 percent top-7 localization accuracy.
- Its ranking matches experienced developers' preferred properties in 42.86 percent of top-1 cases and 90.48 percent of top-7 cases.
- It depends on a prior failure detector. It does not improve layout correctness or determine design intent.
- Its ranked property set comes partly from public question-and-answer data. That can miss new CSS and project-specific causes.

Inference for browser.jr:

- A diagnostic should distinguish the affected fragment from the likely causal declaration.
- Rank causes by layout dependency first. Use heuristic property priors only as a fallback.
- Show several candidates with evidence. Do not present the top candidate as proven.

### DesignChecker: Visual Design Support for Blind and Low Vision Web Developers

Mina Huh and Amy Pavel. 2024. 37th Annual ACM Symposium on User Interface Software and Technology. [DOI 10.1145/3654777.3676369](https://doi.org/10.1145/3654777.3676369). [Author preprint](https://arxiv.org/abs/2407.17681).

Publication status: peer-reviewed UIST conference paper, published in 2024.

What it does:

- DesignChecker lets blind and low-vision developers compare a page with guidelines, one reference page, or similar sites.
- It identifies relevant HTML elements and suggests CSS changes.
- Its interface exposes visual findings through a screen-reader-accessible browser extension.

Evidence and limits:

- The authors interviewed nine developers and analyzed 20 sites before building the tool.
- Eight participants evaluated the result. They found more visual errors with DesignChecker than with their normal workflow.
- This is a small user study. It does not validate a general definition of good visual design.
- Reference-site and trend comparisons can reproduce undesirable conventions.

Inference for browser.jr:

- Findings must work without a screenshot. Text should name the element, violated relation, observed value, expected source, and proposed action.
- Keep guideline, reference, and project-policy rules visibly distinct.
- Explanations and suggested changes need accessible structured output, not only terminal color or overlays.

### Current standards and engine work

These are current primary engineering sources, not research papers.

- [CSS Snapshot 2025](https://www.w3.org/TR/css-2025/) is a W3C Group Note from 18 September 2025. It classifies the modules that formed CSS in 2025. A Group Note is not a W3C Recommendation or member endorsement.
- [Interop 2025's final report](https://webkit.org/blog/17808/interop-2025-review/) was published by WebKit on 6 February 2026. It reports a 97 percent combined pass rate for the selected tests. The project included Flexbox, Grid, and Subgrid in its Layout focus area.
- [A Dive into the Servo Layout System](https://servo.org/files/2025-09-13-a-dive-into-the-servo-layout-system.pdf) is an official Servo technical presentation dated 13 September 2025. It is not peer-reviewed.
- Servo's [August 2025 engineering report](https://servo.org/blog/2025/08/22/this-month-in-servo/) records incremental box-tree construction, more precise invalidation, and cached layout results across replaced, Flexbox, Grid, table, and inline content. It is a first-party implementation report, not a controlled evaluation.

Inference for browser.jr:

- Use CSS Snapshot 2025 to define the standards map, then cite each normative module and test its behavior through WPT.
- Use Interop test subsets to prioritize high-value layout compatibility. Do not treat one aggregate score as browser.jr's compatibility score.
- Inspect current Servo code and WPT results before copying an architectural lesson from its older papers.

### Recent work screened but not prioritized

[Repairing Responsive Layout Failures Using Retrieval Augmented Generation](https://arxiv.org/abs/2511.00678) is a 2025 arXiv preprint. It reports LLM-generated CSS repairs after a detector finds a responsive failure. It is downstream of browser.jr's current problem, lacks a peer-reviewed publication in the sources checked, and does not improve layout computation or evidence. It should not drive the first architecture.

Recent papers on generating UI code from screenshots were also excluded. They optimize code generation similarity, not browser layout conformance or explainable lint findings.

## Connecting agents and interfaces

There are two distinct research directions. Browser.jr may eventually support both, but they need separate contracts.

1. An agent observes an existing interface and performs actions.
2. An agent emits a structured description that a trusted client renders as an interface.

The first direction belongs in browser.jr's initial agent API. The second is useful later for reports and interactive findings.

### BrowserGym: a typed observation and action boundary

- **Year:** 2024 preprint and open research ecosystem
- **Primary source:** [paper](https://arxiv.org/abs/2412.05467)
- **Official implementation:** [ServiceNow/BrowserGym](https://github.com/ServiceNow/BrowserGym)

BrowserGym unifies web-agent benchmarks behind explicit observation and action spaces. Its observations can include accessibility trees, screenshots, element geometry, and page state. Its actions include navigation, element interaction, scrolling, typing, and script execution.

**Implication for browser.jr:** expose one versioned observation schema and one small action schema. Keep model prompts outside the engine. This lets a REPL, an AI agent, and a test runner use the same browser contract.

### WebLINX: prune the interface before sending it to the model

- **Year:** 2024, ICML
- **Primary source:** [PMLR paper and dataset](https://proceedings.mlr.press/v235/lu24e.html)

WebLINX contains 100,000 interactions from 2,300 demonstrations across more than 150 websites. The work ranks relevant HTML elements before giving them to a model. It combines selected elements with screenshots and action history.

**Implication for browser.jr:** do not serialize the whole DOM by default. Let the agent query relevant nodes, then request computed style, fragments, or nearby context.

### WebOlympus: translate grounded actions into browser events

- **Year:** 2024, EMNLP system demonstration
- **Primary source:** [ACL Anthology](https://aclanthology.org/2024.emnlp-demo.20/)

WebOlympus separates page observations, action grounding, executable browser events, and safety monitoring. It also supports human supervision for risky actions.

**Implication for browser.jr:** resolve an agent's target into a stable element identifier before execution. Return the resulting state change and evidence. Require explicit approval classes for actions with external effects.

### UGround: screenshots can provide a second grounding path

- **Year:** 2025, ICLR oral
- **Primary source:** [ICLR proceedings](https://proceedings.iclr.cc/paper_files/paper/2025/hash/4ca0e369689dadb25a5345ba9755ad6f-Abstract-Conference.html)

UGround maps interface descriptions to pixel coordinates using screenshots alone. Its released training set covers 10 million interface elements across 1.3 million screenshots. The paper reports gains across web, desktop, and mobile grounding benchmarks.

**Implication for browser.jr:** use structural identifiers as the normal action target. Add coordinates only as evidence or fallback. A hybrid observation can catch controls missing from the accessibility tree.

### WebArena: judge outcomes, not plausible action traces

- **Year:** 2024, ICLR
- **Primary source:** [ICLR proceedings](https://proceedings.iclr.cc/paper_files/paper/2024/hash/4410c0711e9154a7a2d26f9b3816d1ef-Abstract-Conference.html)

WebArena evaluates long browser tasks by their final functional state. Its strongest reported GPT-4 baseline completed 14.41 percent of tasks. Humans completed 78.24 percent.

**Implication for browser.jr:** record actions for diagnosis, but verify the final page state. A successful click does not prove task success.

### A2UI: the reverse direction from agent to rendered interface

- **Year:** 2025 to 2026
- **Type:** protocol specification, not a peer-reviewed paper
- **Primary source:** [A2UI v0.9.1](https://a2ui.org/specification/v0.9.1-a2ui/)
- **Candidate specification:** [A2UI v1.0](https://github.com/a2ui-project/a2ui/blob/main/specification/v1_0/docs/a2ui_protocol.md)

A2UI lets an agent stream declarative JSON messages to a trusted renderer. The renderer chooses native components and sends typed user actions back. The protocol avoids executing arbitrary model-generated HTML or JavaScript.

**Implication for browser.jr:** this is suitable for an interactive lint report. The agent can select approved report components, but browser.jr should own validation, rendering, styling, and action permissions.

### Recommended browser.jr agent loop

```text
agent
  | observe(query)
  v
compact semantic tree + fragments + optional screenshot
  | action(target_id, operation, arguments)
  v
browser.jr validates target and permission
  | execute
  v
state delta + action result + evidence
  | verify(predicate)
  v
pass | fail | unsupported | indeterminate | unstable
```

The interface boundary should use stable identifiers, typed actions, capability negotiation, and structured errors. Coordinates should remain a fallback. Raw JavaScript should require an explicit capability.

For design lint, the agent does not need unrestricted control. It needs read-only observations, viewport changes, deterministic settling, rule execution, and evidence retrieval. This smaller boundary is easier to test and safer to expose through a REPL.

## Starting comparison: Lightpanda

### Official architecture and source

Primary sources: [Lightpanda architecture overview](https://lightpanda.io/docs/core-concepts/architecture-overview), [Lightpanda browser repository](https://github.com/lightpanda-io/browser), [PandaScript reference](https://lightpanda.io/docs/reference/pandascript), [agent and REPL documentation](https://lightpanda.io/docs/usage/agent)

Facts:

- Lightpanda is a Zig browser for automation and agents.
- It loads HTML, executes JavaScript with V8, and implements a DOM and selected Web APIs.
- It deliberately omits style resolution, real layout, paint, and compositing.
- Element positions are simulated from DOM depth and sibling order. Screenshot capture returns a placeholder.
- One native engine serves several entry points. These include one-shot fetches, CDP, an MCP server, an interactive agent, and reproducible scripts.
- Its runtime hierarchy is `App -> Browser -> Session -> Page -> Frame`. A browser owns one V8 isolate. A session owns cookies and Web Storage.
- Its HTML parser is html5ever. Its network layer uses libcurl.
- The source repository uses AGPL-3.0. Studying its architecture does not grant permission to copy its code into a differently licensed product.

Limits:

- Lightpanda cannot provide truthful overflow, clipping, grid, or alignment evidence because it does not compute visual layout.
- Its CDP compatibility exposes a familiar automation interface. CDP does not define the grid tracks, layout causes, or lint evidence browser.jr needs.

Inference for browser.jr:

- Keep the single-engine, multiple-entry-point shape.
- Keep explicit ownership for process, session, page, and document state.
- Replace Lightpanda's DOM-only stopping point with a fragment-tree stopping point.
- Define a native observation protocol first. Add CDP only when an actual client requires it.
- Do not copy Lightpanda code without a product-level AGPL decision.

## Engine and layout papers

### Engineering the Servo Web Browser Engine Using Rust

Brian Anderson, Lars Bergstrom, Manish Goregaokar, Josh Matthews, Keegan McAllister, Jack Moffitt, and Simon Sapin. 2016. ICSE 2016 Companion. [DOI 10.1145/2889160.2889229](https://doi.org/10.1145/2889160.2889229). [Author preprint](https://arxiv.org/abs/1505.07383).

What it does:

- The paper reports on building Servo as a Rust browser engine with task and data parallelism.
- It describes ownership, message passing, and separate browser pipeline components.
- It reports zero use-after-free bugs in safe Rust during the observed development period.

Evidence and limits:

- This is an engineering experience report, not proof that Rust makes an engine correct or fast.
- Its measurements and architecture describe early Servo. Current Servo has changed.
- The current [Servo architecture](https://github.com/servo/servo/wiki/Design) still separates script, layout, and composition. Layout builds a box tree and a fragment tree before it builds a display list.
- The 2023 [Servo layout engines report](https://github.com/servo/servo/wiki/Servo-Layout-Engines-Report) explains why separate box and fragment trees better match CSS specifications and fragmentation than Servo's older combined flow tree.

Inference for browser.jr:

- Rust is a defensible choice for a long-lived engine with explicit ownership and concurrent callers.
- Use distinct DOM, box-tree, and fragment-tree types. A DOM element can generate zero, one, or several fragments.
- Do not copy Servo's full process architecture. browser.jr can begin in one process with clear ownership boundaries.

### Fast and Parallel Webpage Layout

Leo A. Meyerovich and Rastislav Bodik. 2010. 19th International Conference on World Wide Web, pages 711-720. [Primary WWW 2010 paper](https://archives.iw3c2.org/www2010/proceedings/www/p711.pdf).

What it does:

- The paper presents algorithms for selector matching, layout solving, and font rendering.
- It models layout with attribute grammars and derives parallel tree traversals.
- It separates layout, which computes shapes and positions, from painting, which converts those shapes to pixels.

Evidence and limits:

- The paper reports speedups up to 80 times across its components. Its selector evaluation reports a reduction from 204 ms to 3.5 ms on the tested hardware and pages.
- The prototype handled 99.9 percent of CSS rules encountered in its selector benchmark. This is selector coverage, not full CSS layout conformance.
- The work predates modern Flexbox, Grid, containment, container queries, and current browser architectures.
- Its asymptotic parallel-layout result applies to the paper's attribute-grammar formulation. It is not a bound for all current CSS.

Inference for browser.jr:

- The useful boundary is layout before paint. That matches browser.jr's structured-evidence goal.
- Store layout as explicit inputs and derived attributes. This makes provenance and later parallel evaluation possible.
- Do not optimize for parallel layout yet. First measure a correct serial implementation on representative design-lint pages.

### Automated Reasoning for Web Page Layout

Pavel Panchekha and Emina Torlak. 2016. OOPSLA 2016. [DOI 10.1145/2983990.2984010](https://doi.org/10.1145/2983990.2984010). [Author paper](https://sandcat.cs.washington.edu/papers/Torlak-cassius-oopsla-2016.pdf). [Cassius source](https://github.com/uwplse/Cassius).

What it does:

- Cassius formalizes a substantial CSS fragment as quantifier-free linear real arithmetic.
- It maps an HTML tree and CSS stylesheet to a box layout through constraints.
- The authors built verification, debugging, and CSS synthesis prototypes on that model.
- Its efficient encoding grows linearly with the layout problem size.

Evidence and limits:

- The paper validates its semantics against browser behavior and uses fragments of real sites.
- Cassius models CSS 2.1 features such as block, inline, floats, positioning, margins, and text alignment.
- The current source documentation still lists only a CSS 2.1 fragment. It does not cover modern Grid or Flexbox.
- SMT solving helps explore possible layouts. It is heavier than evaluating one concrete page snapshot.

Inference for browser.jr:

- Copy Cassius's conceptual split between layout facts and assertions, not its runtime architecture.
- Keep concrete lint fast. Reserve symbolic range proofs for a later optional verifier.
- Record the layout inputs behind each derived value. This gives diagnostics a path toward "which declaration caused this box?"

### Verifying That Web Pages Have Accessible Layout

Pavel Panchekha, Adam T. Geller, Michael D. Ernst, Zachary Tatlock, and Shoaib Kamil. 2018. PLDI 2018. [DOI 10.1145/3192366.3192407](https://doi.org/10.1145/3192366.3192407). [Author paper](https://homes.cs.washington.edu/~mernst/pubs/verify-layout-pldi2018.pdf).

What it does:

- VizAssert introduces Visual Logic for geometric properties of page layouts.
- It uses finitization reductions and an SMT solver to check properties over screen sizes, fonts, and user settings.
- Example rules cover onscreen controls, non-overlapping text, minimum text size, line width, and tab order.

Evidence and limits:

- The paper reports 14 assertions checked on 62 professionally designed pages.
- It reports 64 distinct page errors and 13 false-positive warnings.
- VizAssert handles only HTML and CSS. It does not model JavaScript DOM changes.
- It supports a substantial CSS fragment, not all CSS. The paper identifies tables as a hard missing layout feature.
- Solver-backed whole-page verification does not scale cheaply to large pages.

Inference for browser.jr:

- A compact spatial rule language is worth designing early, even if v1 runs it on concrete snapshots.
- Useful core predicates include containment, overlap, order, visibility, minimum size, alignment, and scroll bounds.
- A finding should contain a counterexample snapshot and the boxes that falsified the predicate.
- Avoid claiming a rule holds for every width unless browser.jr performs interval or symbolic analysis.

### Modular Verification of Web Page Layout

Pavel Panchekha, Michael D. Ernst, Zachary Tatlock, and Shoaib Kamil. 2019. Proceedings of the ACM on Programming Languages 3, OOPSLA, article 151. [DOI 10.1145/3360577](https://doi.org/10.1145/3360577). [Author paper](https://homes.cs.washington.edu/~mernst/pubs/verify-layout-modular-oopsla2019.pdf).

What it does:

- Troika divides a page into components.
- Each component has rely and guarantee conditions. Tools verify component properties, then Troika composes them into whole-page results.
- Different components can use different verification methods.

Evidence and limits:

- The case study reports 13 to 1469 times speedups over monolithic VizAssert checks.
- The paper verifies a page about one order of magnitude larger than prior whole-page examples.
- A proof author must identify components and write specifications.
- It inherits the CSS coverage limits of the verification tools it invokes.

Inference for browser.jr:

- Treat page components as first-class lint scopes.
- Cache observations and rule results by component in watch mode.
- Let project rules declare assumptions, such as a component width range, without making all users write proofs.

### Spineless Traversal for Layout Invalidation

Marisa Kirisame, Tiezhi Wang, and Pavel Panchekha. 2025. Proceedings of the ACM on Programming Languages 9, PLDI, pages 1791-1813. [DOI 10.1145/3729322](https://doi.org/10.1145/3729322). [Open preprint](https://arxiv.org/abs/2411.10659).

What it does:

- Incremental browser layout normally marks changed nodes dirty, then traverses the tree to find them.
- Spineless Traversal puts dirty work in a priority queue. It avoids visiting many clean auxiliary nodes and reduces cache traffic.

Evidence and limits:

- The final PLDI paper reports wins on 83.0 percent of 2,216 benchmarks and a mean 1.80 times speedup.
- The largest gains concern latency-sensitive mutations such as typing, hovering, and animation.
- This optimizes repeated layout. It does not simplify CSS semantics or make a partial engine conformant.
- The public arXiv abstract contains older evaluation numbers. This note uses the final PLDI figures.

Inference for browser.jr:

- Full invalidation is acceptable for the first correct lint run.
- Design layout fields with explicit dependencies and a fixed legal recomputation order.
- Use one dirty bit for each packed group of fields computed together.
- Enqueue a dirty group only when its bit changes from clean to dirty.
- Order queued work with Spineless Traversal's priority queue and order-maintenance labels.
- Initialize inserted subtrees as ordered bulk work. Skip queued work whose node was deleted.
- Compare incremental results with clean full layouts during development and conformance testing.

## Design lint and responsive failure papers

### Testing Web Applications Through Layout Constraints

Sylvain Hallé, Nicolas Bergeron, Francis Guérin, and Gabriel Le Breton. 2015. IEEE International Conference on Software Testing, Verification and Validation. [DOI 10.1109/ICST.2015.7102635](https://doi.org/10.1109/ICST.2015.7102635). A longer account appears as [Declarative Layout Constraints for Testing Web Applications](https://doi.org/10.1016/j.jlamp.2016.04.001), 2016.

What it does:

- Cornipickle defines human-readable assertions over DOM content, computed CSS data, and element relationships.
- It samples relevant page state after user events and evaluates temporal and first-order properties.
- The paper classifies more than 90 layout bugs found across 35 real websites and applications.

Evidence and limits:

- Cornipickle is a functional proof of concept.
- It checks browser-produced DOM and CSS observations. It does not implement or validate a layout engine.
- The event probe collects only properties needed by the active assertions. This reduces transfer volume.
- Its browser communication mechanism is obsolete, but its selective observation design remains useful.

Inference for browser.jr:

- Compile each rule into an observation plan. Do not serialize the full DOM and fragment tree for every rule.
- Add temporal rules only after static snapshots work. Watch mode can later express "this control never disappears after input".
- Keep the REPL assertion language close to the structured observation protocol.

### Automated Layout Failure Detection for Responsive Web Pages Without an Explicit Oracle

Thomas A. Walsh, Gregory M. Kapfhammer, and Phil McMinn. 2017. ISSTA 2017, pages 192-202. [DOI 10.1145/3092703.3092712](https://doi.org/10.1145/3092703.3092712). [Author paper and metadata](https://eprints.whiterose.ac.uk/id/eprint/116989/).

What it does:

- ReDeCheck extracts a responsive layout graph from element relationships across viewport widths.
- It detects five responsive failure types without a reference screenshot or hand-written oracle.
- It samples widths at declared layout-change points and uses binary search to locate unannounced relationship changes.

Evidence and limits:

- The study found 33 distinct failures in 16 of 26 production pages.
- It separates true visible failures from false positives and non-observable DOM-level issues.
- Geometry can report a collision or protrusion that users cannot see because the affected pixels are transparent or otherwise unchanged.
- The original implementation drives Firefox through Selenium. Its measurements do not show that a new layout engine agrees with established browsers.

Inference for browser.jr:

- Build a responsive relationship graph from containment, overlap, alignment, order, and viewport membership.
- Sample declared breakpoints, then search intervals where relationships change unexpectedly.
- Report a geometry-only finding as "structural" until a pixel-aware stage confirms visible damage.
- Preserve the exact width interval in evidence. A single named mobile width will miss narrow failures.

### Automatic Visual Verification of Layout Failures in Responsively Designed Web Pages

Ibrahim Althomali, Gregory M. Kapfhammer, and Phil McMinn. 2019. IEEE ICST 2019, pages 183-193. [DOI 10.1109/ICST.2019.00027](https://doi.org/10.1109/ICST.2019.00027). [Author paper and metadata](https://eprints.whiterose.ac.uk/id/eprint/230100/).

What it does:

- Viser receives a geometry finding from ReDeCheck.
- It changes the opacity of implicated elements, renders again, and checks whether affected pixels change.
- This distinguishes visible failures from overlaps or protrusions with no visible effect.

Evidence and limits:

- Viser classified all 117 ReDeCheck reports from 20 pages in its evaluation.
- It disagreed with prior manual classification on 28 reports. The authors confirmed three as false positives from the manual process.
- Viser requires a working pixel renderer and controlled rerendering.
- Visual confirmation says whether pixels changed. It does not determine the intended design.

Inference for browser.jr:

- Keep pixel confirmation outside the v1 engine.
- Define finding evidence now so a later renderer can confirm or downgrade the same finding.
- Use three states: measurable failure, visually confirmed failure, and non-observable structural warning.

### WebSee: A Tool for Debugging HTML Presentation Failures

Sonal Mahajan and William G. J. Halfond. 2015. IEEE ICST 2015. [DOI 10.1109/ICST.2015.7102638](https://doi.org/10.1109/ICST.2015.7102638). [Author paper](https://viterbi-web.usc.edu/~halfond/papers/mahajan15icst-tool.pdf).

What it does:

- WebSee compares a rendered page with a reference image.
- It maps changed image regions back to HTML elements through rendering maps.
- It ranks the elements most likely to cause the presentation failure.

Evidence and limits:

- The tool paper reports high localization accuracy on its evaluated real-world pages.
- It requires an appearance oracle, such as a mockup or prior screenshot.
- Dynamic regions need explicit handling to avoid noisy differences.
- Pixel differences capture text rasterization, platform fonts, and antialiasing noise that structured layout avoids.

Inference for browser.jr:

- Every fragment should retain a stable link to its source element.
- Later pixel regions should link to fragments, elements, and relevant style provenance.
- Screenshot diffing should remain an optional oracle mode, not the default design lint model.

## Cross-browser testing paper

### X-PERT: Accurate Identification of Cross-Browser Issues in Web Applications

Shauvik Roy Choudhary, Mukul R. Prasad, and Alessandro Orso. 2013. 35th International Conference on Software Engineering, pages 702-711. [DOI 10.1109/ICSE.2013.6606616](https://doi.org/10.1109/ICSE.2013.6606616). [Author paper](https://shauvik.com/public/pubs/roychoudhary13icse_cr.pdf).

What it does:

- X-PERT classifies cross-browser differences by application behavior, text content, individual visual appearance, and relative layout.
- Its alignment graph represents page elements as nodes.
- Graph edges record containment or sibling relationships, qualified by facts such as above, left of, and edge alignment.
- X-PERT compares alignment graphs from two real browser executions to find relative-layout incompatibilities.

Evidence and limits:

- The paper derives its categories from a study of real cross-browser failures, then evaluates X-PERT against known failures.
- Relative relationships avoid duplicate reports when one displaced parent moves many descendants.
- Graph equality still needs stable matching between corresponding page states and elements.
- The alignment graph captures rectangles and relations. It does not represent text fragments, grid track sizing, clipping pixels, or design intent.

Inference for browser.jr:

- Make the alignment graph the base of the responsive relationship graph.
- Add fragment identity and viewport intervals, which X-PERT does not need for one-width browser comparison.
- Report the smallest changed relationship. Do not emit one displacement finding for every descendant.
- Store the absolute boxes too. Relations explain structure, while coordinates measure the failure.

### CrossCheck: Combining Crawling and Differencing to Better Detect Cross-browser Incompatibilities in Web Applications

Shauvik Roy Choudhary, Mukul R. Prasad, and Alessandro Orso. 2012. IEEE ICST 2012, pages 171-180. [DOI 10.1109/ICST.2012.97](https://doi.org/10.1109/ICST.2012.97). [Author paper](https://shauvik.com/public/pubs/roychoudhary12icst.pdf).

What it does:

- CrossCheck crawls equivalent application states in different browsers.
- It matches screens and DOM nodes, then compares behavior and corresponding rendered elements.
- Its visual classifier uses box size, displacement, area, leaf text, and image histogram features.

Evidence and limits:

- The paper evaluates the approach on real applications with known incompatibilities.
- It needs actual independent browser executions. Changing only a user-agent string cannot reveal engine differences.
- Matching dynamic application states and corresponding elements is a separate hard problem.
- Its image classifier reflects 2012 browser output and feature engineering.

Inference for browser.jr:

- Call browser.jr's built-in variants profiles, not browsers, unless separate engines execute the page.
- For genuine cross-browser testing, provide adapters that import observations from Firefox and Chromium.
- Match elements by stable authored identity first. Use structural matching only when identity is absent.
- Compare structured geometry before pixels. Preserve small numeric differences as evidence, then let rule policy decide their severity.

## First-party implementation references

### Servo's box tree and fragment tree

The current [Servo design documentation](https://github.com/servo/servo/wiki/Design) describes three layout phases: box-tree construction, fragment-tree construction, and display-list construction. Formatting contexts live in typed Rust enums. A fragment tree can contain several fragments for one box because line breaking, columns, and pagination split content.

Inference:

- browser.jr should stop after fragment-tree construction.
- Element rectangles alone are insufficient. Text can split across lines and boxes can split into fragments.

### Taffy

[Taffy](https://docs.rs/crate/taffy/latest) is a Rust library for CSS Block, Flexbox, and Grid layout. It exposes low-level tree traits and supports cached layout. It does not provide a browser DOM, CSS cascade, text shaping, page lifecycle, or full web compatibility.

Inference:

- Taffy is the fastest credible prototype base for Grid and Flexbox experiments.
- Treat it as one layout algorithm provider behind browser.jr's types.
- Run WPT-derived tests before calling its output browser-compatible.
- Check whether it exposes enough intermediate Grid evidence. Forking or extending it may be necessary for track sizing explanations.
- Do not assume cached whole-node layout implements Spineless Traversal.
- A Spineless adapter must expose field dependencies, dirty propagation, and legal recomputation order.
- If Taffy cannot expose those contracts, keep it only for clean-layout comparison and early prototypes.

### CSS specifications and Web Platform Tests

Primary conformance sources:

- [CSS 2.2 visual formatting model](https://www.w3.org/TR/CSS22/visuren.html)
- [CSS Flexible Box Layout Module Level 1](https://www.w3.org/TR/css-flexbox-1/)
- [CSS Grid Layout Module Level 2](https://www.w3.org/TR/css-grid-2/)
- [CSSOM View Module](https://www.w3.org/TR/cssom-view-1/)
- [Web Platform Tests repository](https://github.com/web-platform-tests/wpt)

Inference:

- Specifications own semantics. Existing engines are differential oracles, not the specification.
- Each supported property needs positive tests, edge tests, and an explicit unsupported result.
- Use the matching WPT directories as the acceptance suite for every declared feature.

## Proposed architecture

```text
CLI, REPL, package, AI protocol
                |
             session
                |
 page + deterministic resource loader
                |
        HTML parser and DOM
                |
        CSS parser and cascade
                |
             box tree
                |
          fragment tree
                |
       structured snapshot
          /           \
 concrete rules    responsive graph
          \           /
        findings and evidence
                |
 optional external-browser or pixel confirmation
```

### Data model

The snapshot should include:

- stable element and fragment identifiers
- parent, child, and source-element links
- border, padding, content, scroll, and ink bounds where available
- fragment order and writing direction
- computed values used by active rules
- formatting context and containing block
- viewport and profile inputs
- support status for every observation
- provenance links from a derived value to relevant declarations and layout decisions

The snapshot should not serialize every engine detail by default. Each rule should request the observations it needs.

### Incremental invalidation

Spineless Traversal is the chosen invalidation algorithm for watch mode. The internal layout model needs:

- stable layout-node and packed-field-group identifiers
- one dirty bit for each packed field group
- generated or declared dependency propagation between field groups
- a fixed recomputation order matching clean layout
- an order-maintenance label for each queued field group
- a packed minimum-priority queue without duplicate dirty entries
- explicit inserted-subtree and deleted-node states

The paper assumes static field dependencies and a schedule that already respects them. Browser.jr must encode or generate both. A priority queue without those contracts is not Spineless Traversal and can recompute fields in the wrong order.

A mutation marks directly affected field groups dirty. The first clean-to-dirty transition adds one queue entry. Recomputing a changed value marks its dependents dirty. Processing continues in recomputation order until the queue becomes empty.

The engine must retain a clean full-layout path. Tests compare its snapshot with the incremental snapshot after every mutation class. A mismatch is an engine defect, never an acceptable approximation.

### Rule result

A result needs more than pass or fail:

```text
rule id
status: pass | fail | unsupported | indeterminate | unstable
severity
target elements and fragments
viewport or viewport interval
expected predicate
observed values
layout evidence
support warnings
optional visual-confirmation status
```

`unsupported` must remain distinct from `pass`. A partial browser engine becomes dangerous when missing layout behavior looks valid.

### Execution strategy

1. Compute a clean full layout for the initial document and concrete viewport.
2. Run structural rules on the snapshot.
3. Build relationships needed for responsive comparison.
4. Evaluate declared breakpoints and target widths.
5. Search between adjacent samples when a relationship changes unexpectedly.
6. Queue changed field groups through Spineless Traversal during watch mode.
7. Reuse unaffected observations and rule results after the incremental snapshot is complete.
8. Send ambiguous structural findings to an optional real-browser or pixel verifier.

## What to build first

### Milestone 1: truthful static layout

- Parse supplied HTML and CSS.
- Support block, Flexbox, and Grid through an explicit compatibility list.
- Produce box and fragment snapshots.
- Store field dependencies and recomputation order needed by Spineless Traversal.
- Return `unsupported` for every encountered feature outside that list.
- Implement horizontal overflow, containment, overlap, alignment, and minimum-target rules.

### Milestone 2: responsive relationships

- Build a relationship graph at each width.
- Sample breakpoints and requested widths.
- Add interval search around unexpected changes.
- Report the smallest known failing width interval.

### Milestone 3: scripts and watch mode

- Give the REPL and package the same observation types.
- Compile rules into selective observation plans.
- Implement Spineless Traversal for field-level layout invalidation.
- Differentially test every mutation class against clean full layout.
- Add component-level rule caching after incremental layout is correct.

### Later work

- external Chromium and Firefox adapters
- pixel confirmation and screenshot oracles
- symbolic range verification with Visual Logic or SMT
- automated CSS repair
- paint and compositing

## Ranked reading order

1. **2025.** [Spineless Traversal for Layout Invalidation](https://doi.org/10.1145/3729322). This is the best current algorithm for fast repeated layout.
2. **2025.** [A Heuristic Approach to Localize CSS Properties for Responsive Layout Failures](https://doi.org/10.5220/0013477500003928). This is the closest recent work on explainable responsive diagnostics.
3. **2026.** [An Empirical Study of Web Visual Flakiness](https://doi.org/10.1016/j.jss.2026.112826). This defines evidence browser.jr must stabilize before reporting findings.
4. **2024.** [DesignChecker](https://doi.org/10.1145/3654777.3676369). This shows how structured findings can work without visual inspection.
5. **2026.** [RTFuzz](https://doi.org/10.1016/j.cose.2025.104756). This informs render-tree mutation, invalidation tests, and deep engine fuzzing.
6. **2025 to 2026.** [CSS Snapshot 2025](https://www.w3.org/TR/css-2025/) and the [Interop 2025 final report](https://webkit.org/blog/17808/interop-2025-review/). These define the current standards map and valuable WPT subsets.
7. **2025.** [A Dive into the Servo Layout System](https://servo.org/files/2025-09-13-a-dive-into-the-servo-layout-system.pdf). Use this official project presentation with current source.
8. **2026 snapshot.** [Lightpanda architecture overview](https://lightpanda.io/docs/core-concepts/architecture-overview). Read this to fix the machine-oriented lifecycle and decide what browser.jr cannot omit.
9. **2017.** [Automated Layout Failure Detection for Responsive Web Pages Without an Explicit Oracle](https://doi.org/10.1145/3092703.3092712). This older paper remains the closest detection algorithm for browser.jr's first product.
10. **2018.** [Verifying That Web Pages Have Accessible Layout](https://doi.org/10.1145/3192366.3192407). This older paper still gives the strongest rule and counterexample model.
11. **2023.** [Servo layout engines report](https://github.com/servo/servo/wiki/Servo-Layout-Engines-Report). This explains the box-tree and fragment-tree split in engineering terms.
12. **2015.** [Testing Web Applications Through Layout Constraints](https://doi.org/10.1109/ICST.2015.7102635). This informs the REPL rule language and selective observations.
13. **2019.** [Automatic Visual Verification of Layout Failures](https://doi.org/10.1109/ICST.2019.00027). This prevents geometry findings from being mistaken for visible defects.
14. **2016.** [Automated Reasoning for Web Page Layout](https://doi.org/10.1145/2983990.2984010). Read this before adding provenance, symbolic checks, or repair.
15. **2016.** [Engineering the Servo Web Browser Engine Using Rust](https://doi.org/10.1145/2889160.2889229). Read this for ownership and concurrency lessons, while checking current Servo documents for drift.
16. **2013.** [X-PERT](https://doi.org/10.1109/ICSE.2013.6606616). This gives the alignment graph that ReDeCheck later extends across viewport widths.
17. **2012.** [CrossCheck](https://doi.org/10.1109/ICST.2012.97). Read this before calling profile comparison cross-browser testing.
18. **2010.** [Fast and Parallel Webpage Layout](https://archives.iw3c2.org/www2010/proceedings/www/p711.pdf). Read this when selector matching or layout traversal becomes measured CPU work.
19. **2019.** [Modular Verification of Web Page Layout](https://doi.org/10.1145/3360577). Read this before scaling proofs or checks across large component libraries.

## Recommendation

Build browser.jr as a layout-observation engine, not a miniature visual browser and not a Lightpanda fork.

Use Rust. Keep a Lightpanda-like session and page lifecycle. Parse HTML with a mature parser. Put CSS cascade, box construction, fragment layout, and support reporting behind browser.jr-owned interfaces. Prototype Block, Flexbox, and Grid with Taffy, but validate every claimed behavior against specifications and WPT.

Use Spineless Traversal for incremental layout once watch mode begins. Store dirty state beside packed layout fields. Keep the queue and order-maintenance machinery private to the layout engine. Preserve a clean full-layout path for differential tests and recovery.

The first differentiator should be a structured snapshot plus a responsive relationship graph. That pair supports fast built-in rules, explainable findings, AI inspection, and later external verification. It also preserves the product's small boundary because paint and compositing stay out.

Do not begin with parallel layout, symbolic proof, screenshot comparison, or a broad CDP implementation. The papers show that each can help. None compensates for a small, truthful, observable layout core.
