use std::fmt::{Display, Formatter, Result};

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct ProgramCounter(pub u16);

impl Display for ProgramCounter {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}
