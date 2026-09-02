# browser.jr knowledge graph

This graph maps implemented browser.jr code to runtime flow, state ownership, domain invariants, design decisions, integration boundaries, release work, and recorded verification.

## Choose a view

Start with the view that matches the question you need to answer.

- Use [[architecture]] to find module ownership and the main execution paths.
- Use [[domain]] to find terms and invariants that code must preserve.
- Use [[decisions]] to understand why the current boundaries exist.
- Use [[verification-map]] to connect claims to checks and conflict records.

## Core graph

These pages provide focused routes through the implemented system.

- [[architecture]]
- [[decisions]]
- [[domain]]
- [[runtime-flow]]
- [[session-state]]
- [[network-loading]]
- [[page-pipeline]]
- [[interaction-pipeline]]
- [[evidence-and-snapshots]]
- [[screenshot-pipeline]]
- [[plugin-protocol]]
- [[benchmark-harness]]
- [[release-and-packaging]]
- [[verification-map]]
- [[locator-resolution]]
- [[action-transactions]]
- [[keyboard-state]]
- [[session-wire]]

## Concern map

This table connects each concern to its runtime map, invariant, and design decision.

| Concern | Runtime and ownership | Domain invariant | Governing decision |
| --- | --- | --- | --- |
| Request execution | [[runtime-flow]] | [[domain#Session and current page]] | [[decisions#Requests select their reply types]] |
| Session state | [[architecture#Session ownership]] | [[session-state]] | [[decisions#One session owns mutable browser state]] |
| Loading | [[network-loading]] | [[domain#Document epoch]] | [[decisions#Network authority is explicit and narrow]] |
| Page evidence | [[page-pipeline]] | [[domain#Support state]] | [[decisions#The engine models a supported static subset]] |
| Snapshots | [[evidence-and-snapshots]] | [[domain#Snapshot and evidence]] | [[decisions#Snapshots are immutable and references expire]] |
| Actions | [[interaction-pipeline]] | [[domain#Actionability and action point]] | [[decisions#State changes are transactional]] |
| Screenshots | [[screenshot-pipeline]] | [[domain#Paint scene and raster image]] | [[decisions#Rasterization is lazy and bounded]] |
| Agent integration | [[plugin-protocol]] | [[domain#Plugin session and relay]] | [[decisions#Agent-browser integration uses commands, not CDP]] |
| Performance evidence | [[benchmark-harness]] | [[verification-map]] | [[decisions#One session owns mutable browser state]] |
| Distribution | [[release-and-packaging]] | [[plugin-protocol#Native executable lookup]] | [[decisions#Agent-browser integration uses commands, not CDP]] |

## Other repository sources

These repository documents own product behavior, vocabulary, proposed design, and confirmed conflicts.

The user-facing product contract remains in the repository [README](../README.md). The [glossary](../glossary.md) owns shared definitions. The [architecture draft](../architecture.md) contains proposed designs that are not all implemented. The [bug triage](../bug-triage.md) owns confirmed conflicts between intent and evidence.
