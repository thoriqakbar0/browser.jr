use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectOptionTarget {
    Value(String),
    Label(String),
    Index(usize),
}

impl fmt::Display for SelectOptionTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(value) => write!(formatter, "value {value:?}"),
            Self::Label(label) => write!(formatter, "label {label:?}"),
            Self::Index(index) => write!(formatter, "index {index}"),
        }
    }
}
