# Evidence and snapshots

Snapshots freeze browser observations. Support state says whether browser.jr can make each dependent claim.

## Support state

[[src/snapshot.rs#ObservationCell]] represents available, unsupported, indeterminate, or unstable evidence. [[src/snapshot.rs#Evidenced]] pairs values with their evidence status.

Rules and actions preserve the blocking reason. Unsupported does not mean false, and missing evidence does not become a passing result.

## Snapshot forms

[[src/snapshot.rs#Snapshot]] stores layout observations for rule evaluation. [[src/snapshot.rs#InteractiveSnapshot]] projects agent-oriented targets. [[src/snapshot.rs#AccessibilitySnapshot]] projects the supported accessibility tree.

[[src/snapshot.rs#EvidenceRef]] links rule output to a snapshot and semantic element. Each snapshot remains immutable after capture.

## References

[[src/snapshot.rs#SnapshotCaptureIdentity]] allocates capture identity. [[src/snapshot.rs#InteractiveElementRef]] combines that capture with the document epoch and document-order ordinal shown as `@eN`.

A later interactive capture replaces the usable references. A successful document replacement invalidates references from the previous document. This prevents an old handle from silently selecting a different element.

## Rule consumption

[[src/rules.rs#evaluate_horizontal_overflow]] and [[src/rules.rs#evaluate_max_element_width]] consume immutable snapshots. [[src/rules.rs#RuleResult]] separates completed comparisons from blocked evaluations.

The rule layer does not inspect mutable session state. The session or package caller captures evidence first, then passes the snapshot to a pure rule.

## Related maps

[[session-state]] owns identity lifetime. [[page-pipeline]] produces snapshot inputs. [[decisions#Snapshots are immutable and references expire]] records the design choice.
