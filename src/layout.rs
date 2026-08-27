use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum FieldGroup {
    X,
    Width,
    Right,
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
        let mut positions = BTreeMap::new();
        for (position, spec) in fields.iter().enumerate() {
            if positions.insert(spec.field, position).is_some() {
                return Err(LayoutProgramError::DuplicateField(spec.field));
            }
        }

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

        let mut dependents = BTreeMap::<FieldGroup, Vec<FieldGroup>>::new();
        for spec in fields {
            for dependency in &spec.dependencies {
                dependents.entry(*dependency).or_default().push(spec.field);
            }
        }

        Ok(Self {
            order: fields.iter().map(|spec| spec.field).collect(),
            ranks: positions,
            dependents,
        })
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutMutation {
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

#[derive(Debug)]
pub(crate) struct LayoutKernel {
    program: LayoutProgram,
    current: Option<LayoutState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LayoutState {
    input: LayoutInput,
    snapshot: LayoutSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PendingWork {
    rank: usize,
    element_index: usize,
    field: FieldGroup,
}

impl LayoutKernel {
    pub(crate) fn new(program: LayoutProgram) -> Self {
        Self {
            program,
            current: None,
        }
    }

    pub(crate) fn clean_layout(
        &mut self,
        input: LayoutInput,
    ) -> Result<LayoutSnapshot, LayoutError> {
        let mut seen = BTreeSet::new();
        let mut observations = Vec::with_capacity(input.elements.len());
        for element in &input.elements {
            let id = SemanticElementId::parse(element.id.clone())?;
            if !seen.insert(id.clone()) {
                return Err(LayoutError::DuplicateElementId(id.0));
            }
            observations.push(Self::compute_observation(element)?);
        }

        debug_assert_eq!(
            self.program.order,
            [FieldGroup::X, FieldGroup::Width, FieldGroup::Right]
        );
        let snapshot = LayoutSnapshot {
            viewport_width: input.viewport_width,
            observations,
        };
        self.current = Some(LayoutState {
            input,
            snapshot: snapshot.clone(),
        });
        Ok(snapshot)
    }

    pub(crate) fn apply_mutation(
        &mut self,
        mutation: LayoutMutation,
    ) -> Result<LayoutSnapshot, LayoutError> {
        let mut candidate = self.current.clone().ok_or(LayoutError::NoLayout)?;
        let mut pending = BTreeSet::new();
        match mutation {
            LayoutMutation::SetWidth { element, width } => {
                let index = candidate
                    .input
                    .elements
                    .iter()
                    .position(|candidate| candidate.id == element)
                    .ok_or_else(|| LayoutError::UnknownElement(element.clone()))?;
                let ElementLayout::Supported {
                    width: current_width,
                    ..
                } = &mut candidate.input.elements[index].layout
                else {
                    return Err(LayoutError::UnsupportedElement(element));
                };
                if *current_width != width {
                    *current_width = width;
                    pending.insert(PendingWork {
                        rank: self.program.rank(FieldGroup::Width),
                        element_index: index,
                        field: FieldGroup::Width,
                    });
                }
            }
        }

        while let Some(work) = pending.pop_first() {
            if work.field == FieldGroup::Right {
                candidate.snapshot.observations[work.element_index] =
                    Self::compute_observation(&candidate.input.elements[work.element_index])?;
            }
            if let Some(dependents) = self.program.dependents.get(&work.field) {
                pending.extend(dependents.iter().map(|field| PendingWork {
                    rank: self.program.rank(*field),
                    element_index: work.element_index,
                    field: *field,
                }));
            }
        }

        let snapshot = candidate.snapshot.clone();
        self.current = Some(candidate);
        Ok(snapshot)
    }

    fn compute_observation(element: &ElementInput) -> Result<LayoutObservation, LayoutError> {
        let id = SemanticElementId::parse(element.id.clone())?;
        match &element.layout {
            ElementLayout::Supported { x, width } => {
                let signed_width =
                    i64::try_from(*width).map_err(|_| LayoutError::CoordinateOverflow {
                        element: id.0.clone(),
                    })?;
                let right =
                    x.checked_add(signed_width)
                        .ok_or_else(|| LayoutError::CoordinateOverflow {
                            element: id.0.clone(),
                        })?;
                Ok(LayoutObservation::Available(Fragment {
                    element: id,
                    x: *x,
                    width: *width,
                    right,
                }))
            }
            ElementLayout::Unsupported { reason } => Ok(LayoutObservation::Unsupported {
                element: id,
                reason: reason.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ElementInput, FieldGroup, FieldSpec, LayoutInput, LayoutKernel, LayoutMutation,
        LayoutProgram, LayoutProgramError,
    };

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
}
