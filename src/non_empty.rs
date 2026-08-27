use std::ops::Index;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmpty<T> {
    pub fn one(first: T) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    pub fn from_vec(values: Vec<T>) -> Option<Self> {
        let mut values = values.into_iter();
        let first = values.next()?;
        Some(Self {
            first,
            rest: values.collect(),
        })
    }

    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.first).chain(&self.rest)
    }
}

impl<T> Index<usize> for NonEmpty<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index == 0 {
            &self.first
        } else {
            &self.rest[index - 1]
        }
    }
}
