use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum FieldGroup {
    X,
    Width,
    Right,
}

impl FieldGroup {
    const ALL: [Self; 3] = [Self::X, Self::Width, Self::Right];
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldSpec {
    pub(crate) field: FieldGroup,
    pub(crate) dependencies: Vec<FieldGroup>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LayoutProgram {
    order: Vec<FieldGroup>,
    ranks: BTreeMap<FieldGroup, usize>,
    dependents: BTreeMap<FieldGroup, Vec<FieldGroup>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LayoutProgramError {
    DuplicateField(FieldGroup),
    MissingField(FieldGroup),
    MissingDependency {
        field: FieldGroup,
        dependency: FieldGroup,
    },
    IllegalOrder {
        field: FieldGroup,
        dependency: FieldGroup,
    },
}

impl LayoutProgram {
    pub(crate) fn compile(fields: &[FieldSpec]) -> Result<Self, LayoutProgramError> {
        let positions = Self::field_positions(fields)?;
        Self::validate_dependencies(fields, &positions)?;
        Self::validate_outputs(&positions)?;
        let dependents = Self::dependent_fields(fields);

        Ok(Self {
            order: fields.iter().map(|spec| spec.field).collect(),
            ranks: positions,
            dependents,
        })
    }

    fn field_positions(
        fields: &[FieldSpec],
    ) -> Result<BTreeMap<FieldGroup, usize>, LayoutProgramError> {
        let mut positions = BTreeMap::new();
        for (position, spec) in fields.iter().enumerate() {
            if positions.insert(spec.field, position).is_some() {
                return Err(LayoutProgramError::DuplicateField(spec.field));
            }
        }
        Ok(positions)
    }

    fn validate_dependencies(
        fields: &[FieldSpec],
        positions: &BTreeMap<FieldGroup, usize>,
    ) -> Result<(), LayoutProgramError> {
        for (position, spec) in fields.iter().enumerate() {
            for dependency in &spec.dependencies {
                let Some(dependency_position) = positions.get(dependency) else {
                    return Err(LayoutProgramError::MissingDependency {
                        field: spec.field,
                        dependency: *dependency,
                    });
                };
                if dependency_position >= &position {
                    return Err(LayoutProgramError::IllegalOrder {
                        field: spec.field,
                        dependency: *dependency,
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_outputs(positions: &BTreeMap<FieldGroup, usize>) -> Result<(), LayoutProgramError> {
        for field in FieldGroup::ALL {
            if !positions.contains_key(&field) {
                return Err(LayoutProgramError::MissingField(field));
            }
        }
        Ok(())
    }

    fn dependent_fields(fields: &[FieldSpec]) -> BTreeMap<FieldGroup, Vec<FieldGroup>> {
        let mut dependents = BTreeMap::<FieldGroup, Vec<FieldGroup>>::new();
        for spec in fields {
            for dependency in &spec.dependencies {
                dependents.entry(*dependency).or_default().push(spec.field);
            }
        }
        dependents
    }

    fn rank(&self, field: FieldGroup) -> usize {
        self.ranks[&field]
    }

    pub(crate) fn initial() -> Self {
        Self::compile(&[
            FieldSpec {
                field: FieldGroup::X,
                dependencies: vec![],
            },
            FieldSpec {
                field: FieldGroup::Width,
                dependencies: vec![],
            },
            FieldSpec {
                field: FieldGroup::Right,
                dependencies: vec![FieldGroup::X, FieldGroup::Width],
            },
        ])
        .expect("the built-in field program must be valid")
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SemanticElementId(String);

impl SemanticElementId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, LayoutError> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(LayoutError::EmptyElementId)
        } else {
            Ok(Self(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ElementLayout {
    Supported { x: i64, width: u64 },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementInput {
    pub(crate) id: String,
    pub(crate) layout: ElementLayout,
}

impl ElementInput {
    pub fn supported(id: impl Into<String>, x: i64, width: u64) -> Self {
        Self {
            id: id.into(),
            layout: ElementLayout::Supported { x, width },
        }
    }

    pub fn unsupported(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            layout: ElementLayout::Unsupported {
                reason: reason.into(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutInput {
    pub viewport_width: u64,
    pub elements: Vec<ElementInput>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundingBox {
    pub x: i64,
    pub y: i64,
    pub width: u64,
    pub height: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutMutation {
    SetX { element: String, x: i64 },
    SetWidth { element: String, width: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Fragment {
    pub(crate) element: SemanticElementId,
    pub(crate) x: i64,
    pub(crate) width: u64,
    pub(crate) right: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LayoutObservation {
    Available(Fragment),
    Unsupported {
        element: SemanticElementId,
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LayoutSnapshot {
    pub(crate) viewport_width: u64,
    pub(crate) observations: Vec<LayoutObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutError {
    EmptyElementId,
    DuplicateElementId(String),
    CoordinateOverflow { element: String },
    NoLayout,
    UnknownElement(String),
    UnsupportedElement(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FieldStore {
    elements: Vec<SemanticElementId>,
    unsupported_reasons: Vec<Option<String>>,
    x: Vec<i64>,
    width: Vec<u64>,
    right: Vec<i64>,
}

impl FieldStore {
    fn from_input(input: &LayoutInput) -> Result<Self, LayoutError> {
        let mut seen = BTreeSet::new();
        let capacity = input.elements.len();
        let mut elements = Vec::with_capacity(capacity);
        let mut unsupported_reasons = Vec::with_capacity(capacity);
        let mut x = Vec::with_capacity(capacity);
        let mut width = Vec::with_capacity(capacity);
        let mut right = Vec::with_capacity(capacity);
        for element in &input.elements {
            let id = SemanticElementId::parse(element.id.clone())?;
            if !seen.insert(id.clone()) {
                return Err(LayoutError::DuplicateElementId(id.0));
            }
            elements.push(id);
            unsupported_reasons.push(match &element.layout {
                ElementLayout::Supported { .. } => None,
                ElementLayout::Unsupported { reason } => Some(reason.clone()),
            });
            x.push(0);
            width.push(0);
            right.push(0);
        }
        Ok(Self {
            elements,
            unsupported_reasons,
            x,
            width,
            right,
        })
    }

    fn recompute(
        &mut self,
        input: &LayoutInput,
        address: FieldAddress,
    ) -> Result<bool, LayoutError> {
        let source = &input.elements[address.element_index];
        if self.unsupported_reasons[address.element_index].is_some() {
            debug_assert!(matches!(source.layout, ElementLayout::Unsupported { .. }));
            return Ok(false);
        }
        let ElementLayout::Supported {
            x: source_x,
            width: source_width,
        } = source.layout
        else {
            unreachable!("layout support cannot change through a field mutation")
        };

        let changed = match address.field {
            FieldGroup::X => replace_if_changed(&mut self.x[address.element_index], source_x),
            FieldGroup::Width => {
                replace_if_changed(&mut self.width[address.element_index], source_width)
            }
            FieldGroup::Right => {
                let element = &self.elements[address.element_index];
                let signed_width =
                    i64::try_from(self.width[address.element_index]).map_err(|_| {
                        LayoutError::CoordinateOverflow {
                            element: element.0.clone(),
                        }
                    })?;
                let next = self.x[address.element_index]
                    .checked_add(signed_width)
                    .ok_or_else(|| LayoutError::CoordinateOverflow {
                        element: element.0.clone(),
                    })?;
                replace_if_changed(&mut self.right[address.element_index], next)
            }
        };
        Ok(changed)
    }

    fn snapshot(&self, viewport_width: u64) -> LayoutSnapshot {
        let observations = (0..self.elements.len())
            .map(|index| match &self.unsupported_reasons[index] {
                Some(reason) => LayoutObservation::Unsupported {
                    element: self.elements[index].clone(),
                    reason: reason.clone(),
                },
                None => LayoutObservation::Available(Fragment {
                    element: self.elements[index].clone(),
                    x: self.x[index],
                    width: self.width[index],
                    right: self.right[index],
                }),
            })
            .collect();
        LayoutSnapshot {
            viewport_width,
            observations,
        }
    }
}

fn replace_if_changed<T: Eq>(current: &mut T, next: T) -> bool {
    if *current == next {
        false
    } else {
        *current = next;
        true
    }
}

#[derive(Debug)]
pub(crate) struct LayoutKernel {
    program: LayoutProgram,
    current: Option<LayoutState>,
    #[cfg(test)]
    last_recomputed: Vec<FieldAddress>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LayoutState {
    input: LayoutInput,
    fields: FieldStore,
    snapshot: LayoutSnapshot,
    dirty: BTreeSet<FieldAddress>,
    pending: BTreeSet<PendingWork>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FieldAddress {
    element_index: usize,
    field: FieldGroup,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PendingWork {
    rank: usize,
    order_label: usize,
    field: FieldGroup,
}

impl LayoutState {
    fn mark_dirty(&mut self, program: &LayoutProgram, address: FieldAddress) {
        if self.dirty.insert(address) {
            let inserted = self.pending.insert(PendingWork {
                rank: program.rank(address.field),
                order_label: address.element_index,
                field: address.field,
            });
            debug_assert!(inserted);
        }
    }

    fn pop_pending(&mut self) -> Option<FieldAddress> {
        let work = self.pending.pop_first()?;
        let address = FieldAddress {
            element_index: work.order_label,
            field: work.field,
        };
        let was_dirty = self.dirty.remove(&address);
        debug_assert!(was_dirty);
        Some(address)
    }
}

impl LayoutKernel {
    pub(crate) fn new(program: LayoutProgram) -> Self {
        Self {
            program,
            current: None,
            #[cfg(test)]
            last_recomputed: Vec::new(),
        }
    }

    pub(crate) fn clean_layout(
        &mut self,
        input: LayoutInput,
    ) -> Result<LayoutSnapshot, LayoutError> {
        let mut fields = FieldStore::from_input(&input)?;
        for field in self.program.order.iter().copied() {
            for element_index in 0..input.elements.len() {
                fields.recompute(
                    &input,
                    FieldAddress {
                        element_index,
                        field,
                    },
                )?;
            }
        }
        let snapshot = fields.snapshot(input.viewport_width);
        self.current = Some(LayoutState {
            input,
            fields,
            snapshot: snapshot.clone(),
            dirty: BTreeSet::new(),
            pending: BTreeSet::new(),
        });
        #[cfg(test)]
        self.last_recomputed.clear();
        Ok(snapshot)
    }

    pub(crate) fn apply_mutation(
        &mut self,
        mutation: LayoutMutation,
    ) -> Result<LayoutSnapshot, LayoutError> {
        self.apply_mutations([mutation])
    }

    pub(crate) fn apply_mutations(
        &mut self,
        mutations: impl IntoIterator<Item = LayoutMutation>,
    ) -> Result<LayoutSnapshot, LayoutError> {
        let mut candidate = self.current.clone().ok_or(LayoutError::NoLayout)?;
        for mutation in mutations {
            Self::apply_input_mutation(&self.program, &mut candidate, mutation)?;
        }

        #[cfg(test)]
        let mut recomputed = Vec::new();
        while let Some(address) = candidate.pop_pending() {
            #[cfg(test)]
            recomputed.push(address);
            let changed = candidate.fields.recompute(&candidate.input, address)?;
            if changed {
                let dependents = self
                    .program
                    .dependents
                    .get(&address.field)
                    .cloned()
                    .unwrap_or_default();
                for field in dependents {
                    candidate.mark_dirty(
                        &self.program,
                        FieldAddress {
                            element_index: address.element_index,
                            field,
                        },
                    );
                }
            }
        }

        debug_assert!(candidate.dirty.is_empty());
        candidate.snapshot = candidate.fields.snapshot(candidate.input.viewport_width);
        let snapshot = candidate.snapshot.clone();
        self.current = Some(candidate);
        #[cfg(test)]
        {
            self.last_recomputed = recomputed;
        }
        Ok(snapshot)
    }

    fn apply_input_mutation(
        program: &LayoutProgram,
        state: &mut LayoutState,
        mutation: LayoutMutation,
    ) -> Result<(), LayoutError> {
        let (element, field) = match mutation {
            LayoutMutation::SetX { element, x } => {
                let index = Self::supported_element_index(state, &element)?;
                let ElementLayout::Supported { x: current_x, .. } =
                    &mut state.input.elements[index].layout
                else {
                    unreachable!("supported_element_index validates layout support")
                };
                if *current_x == x {
                    return Ok(());
                }
                *current_x = x;
                (index, FieldGroup::X)
            }
            LayoutMutation::SetWidth { element, width } => {
                let index = Self::supported_element_index(state, &element)?;
                let ElementLayout::Supported {
                    width: current_width,
                    ..
                } = &mut state.input.elements[index].layout
                else {
                    unreachable!("supported_element_index validates layout support")
                };
                if *current_width == width {
                    return Ok(());
                }
                *current_width = width;
                (index, FieldGroup::Width)
            }
        };
        state.mark_dirty(
            program,
            FieldAddress {
                element_index: element,
                field,
            },
        );
        Ok(())
    }

    fn supported_element_index(state: &LayoutState, element: &str) -> Result<usize, LayoutError> {
        let index = state
            .input
            .elements
            .iter()
            .position(|candidate| candidate.id == element)
            .ok_or_else(|| LayoutError::UnknownElement(element.into()))?;
        if matches!(
            state.input.elements[index].layout,
            ElementLayout::Unsupported { .. }
        ) {
            Err(LayoutError::UnsupportedElement(element.into()))
        } else {
            Ok(index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ElementInput, FieldAddress, FieldGroup, FieldSpec, LayoutError, LayoutInput, LayoutKernel,
        LayoutMutation, LayoutProgram, LayoutProgramError,
    };

    #[derive(Clone, Copy)]
    struct MutationCase {
        initial_x: i64,
        initial_width: u64,
        final_x: i64,
        final_width: u64,
        reverse_order: bool,
    }

    fn mutation_cases() -> impl Iterator<Item = MutationCase> {
        const X_VALUES: [i64; 3] = [-20, 0, 280];
        const WIDTH_VALUES: [u64; 3] = [0, 40, 80];

        X_VALUES.into_iter().flat_map(|initial_x| {
            WIDTH_VALUES.into_iter().flat_map(move |initial_width| {
                X_VALUES.into_iter().flat_map(move |final_x| {
                    WIDTH_VALUES.into_iter().flat_map(move |final_width| {
                        [false, true]
                            .into_iter()
                            .map(move |reverse_order| MutationCase {
                                initial_x,
                                initial_width,
                                final_x,
                                final_width,
                                reverse_order,
                            })
                    })
                })
            })
        })
    }

    fn two_element_input(hero_x: i64, hero_width: u64) -> LayoutInput {
        LayoutInput {
            viewport_width: 320,
            elements: vec![
                ElementInput::supported("header", 0, 320),
                ElementInput::supported("hero", hero_x, hero_width),
            ],
        }
    }

    fn mutations_for(case: MutationCase) -> [LayoutMutation; 2] {
        let set_x = LayoutMutation::SetX {
            element: "hero".into(),
            x: case.final_x,
        };
        let set_width = LayoutMutation::SetWidth {
            element: "hero".into(),
            width: case.final_width,
        };
        if case.reverse_order {
            [set_width, set_x]
        } else {
            [set_x, set_width]
        }
    }

    fn assert_mutation_matches_clean(case: MutationCase) {
        let mut incremental_kernel = LayoutKernel::new(LayoutProgram::initial());
        incremental_kernel
            .clean_layout(two_element_input(case.initial_x, case.initial_width))
            .unwrap();

        let incremental = incremental_kernel
            .apply_mutations(mutations_for(case))
            .unwrap();
        let clean = LayoutKernel::new(LayoutProgram::initial())
            .clean_layout(two_element_input(case.final_x, case.final_width))
            .unwrap();

        assert_eq!(
            incremental, clean,
            "initial ({}, {}), final ({}, {}), reverse {}",
            case.initial_x, case.initial_width, case.final_x, case.final_width, case.reverse_order
        );
    }

    #[test]
    fn field_program_rejects_missing_dependency() {
        let result = LayoutProgram::compile(&[FieldSpec {
            field: FieldGroup::Right,
            dependencies: vec![FieldGroup::Width],
        }]);

        assert_eq!(
            result,
            Err(LayoutProgramError::MissingDependency {
                field: FieldGroup::Right,
                dependency: FieldGroup::Width,
            })
        );
    }

    #[test]
    fn field_program_rejects_duplicate_output() {
        let result = LayoutProgram::compile(&[
            FieldSpec {
                field: FieldGroup::X,
                dependencies: vec![],
            },
            FieldSpec {
                field: FieldGroup::X,
                dependencies: vec![],
            },
        ]);

        assert_eq!(
            result,
            Err(LayoutProgramError::DuplicateField(FieldGroup::X))
        );
    }

    #[test]
    fn field_program_rejects_illegal_order() {
        let result = LayoutProgram::compile(&[
            FieldSpec {
                field: FieldGroup::Right,
                dependencies: vec![FieldGroup::X],
            },
            FieldSpec {
                field: FieldGroup::X,
                dependencies: vec![],
            },
        ]);

        assert_eq!(
            result,
            Err(LayoutProgramError::IllegalOrder {
                field: FieldGroup::Right,
                dependency: FieldGroup::X,
            })
        );
    }

    #[test]
    fn field_program_rejects_missing_output() {
        let result = LayoutProgram::compile(&[
            FieldSpec {
                field: FieldGroup::X,
                dependencies: vec![],
            },
            FieldSpec {
                field: FieldGroup::Width,
                dependencies: vec![],
            },
        ]);

        assert_eq!(
            result,
            Err(LayoutProgramError::MissingField(FieldGroup::Right))
        );
    }

    #[test]
    fn incremental_snapshot_equals_the_full_clean_snapshot() {
        let mut incremental_kernel = LayoutKernel::new(LayoutProgram::initial());
        incremental_kernel
            .clean_layout(LayoutInput {
                viewport_width: 320,
                elements: vec![ElementInput::supported("hero", 280, 40)],
            })
            .unwrap();
        let incremental = incremental_kernel
            .apply_mutation(LayoutMutation::SetWidth {
                element: "hero".into(),
                width: 80,
            })
            .unwrap();

        let clean = LayoutKernel::new(LayoutProgram::initial())
            .clean_layout(LayoutInput {
                viewport_width: 320,
                elements: vec![ElementInput::supported("hero", 280, 80)],
            })
            .unwrap();

        assert_eq!(incremental, clean);
    }

    #[test]
    fn batched_fields_run_once_in_dependency_order() {
        let mut incremental_kernel = LayoutKernel::new(LayoutProgram::initial());
        incremental_kernel
            .clean_layout(LayoutInput {
                viewport_width: 320,
                elements: vec![
                    ElementInput::supported("header", 0, 320),
                    ElementInput::supported("hero", 20, 40),
                ],
            })
            .unwrap();

        let incremental = incremental_kernel
            .apply_mutations([
                LayoutMutation::SetWidth {
                    element: "hero".into(),
                    width: 80,
                },
                LayoutMutation::SetX {
                    element: "hero".into(),
                    x: 280,
                },
            ])
            .unwrap();
        let clean = LayoutKernel::new(LayoutProgram::initial())
            .clean_layout(LayoutInput {
                viewport_width: 320,
                elements: vec![
                    ElementInput::supported("header", 0, 320),
                    ElementInput::supported("hero", 280, 80),
                ],
            })
            .unwrap();

        assert_eq!(incremental, clean);
        assert_eq!(
            incremental_kernel.last_recomputed,
            vec![
                FieldAddress {
                    element_index: 1,
                    field: FieldGroup::X,
                },
                FieldAddress {
                    element_index: 1,
                    field: FieldGroup::Width,
                },
                FieldAddress {
                    element_index: 1,
                    field: FieldGroup::Right,
                },
            ]
        );
    }

    #[test]
    fn repeated_dirty_marking_converges_and_unchanged_values_stop_propagation() {
        let input = LayoutInput {
            viewport_width: 320,
            elements: vec![ElementInput::supported("hero", 280, 40)],
        };
        let mut kernel = LayoutKernel::new(LayoutProgram::initial());
        let initial = kernel.clean_layout(input).unwrap();

        let result = kernel
            .apply_mutations([
                LayoutMutation::SetWidth {
                    element: "hero".into(),
                    width: 80,
                },
                LayoutMutation::SetWidth {
                    element: "hero".into(),
                    width: 40,
                },
            ])
            .unwrap();

        assert_eq!(result, initial);
        assert_eq!(
            kernel.last_recomputed,
            vec![FieldAddress {
                element_index: 0,
                field: FieldGroup::Width,
            }]
        );
    }

    #[test]
    fn mutation_matrix_matches_clean_layout() {
        mutation_cases().for_each(assert_mutation_matches_clean);
    }

    #[test]
    fn failed_batch_preserves_the_committed_layout() {
        let input = LayoutInput {
            viewport_width: 320,
            elements: vec![ElementInput::supported("hero", 20, 40)],
        };
        let mut kernel = LayoutKernel::new(LayoutProgram::initial());
        let initial = kernel.clean_layout(input).unwrap();

        let failure = kernel.apply_mutations([
            LayoutMutation::SetX {
                element: "hero".into(),
                x: 280,
            },
            LayoutMutation::SetWidth {
                element: "missing".into(),
                width: 80,
            },
        ]);
        let after_failure = kernel.apply_mutations([]).unwrap();

        assert_eq!(failure, Err(LayoutError::UnknownElement("missing".into())));
        assert_eq!(after_failure, initial);
    }
}
