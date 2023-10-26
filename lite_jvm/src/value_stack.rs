use crate::jvm_exceptions::{Exception, Result};
use crate::reference_value::Value;
use thiserror::Error;

#[derive(Debug)]
pub struct ValueStack<'a> {
    stack: Vec<Value<'a>>,
}
/// Errors returned from various stack operations
#[derive(Error, Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ValueStackFault {
    #[error("stack overflow ")]
    StackOverFlow,
    #[error("cannot pop from an empty stack")]
    EmptyStack,
}
impl<'a> ValueStack<'a> {
    pub(crate) fn new(max_size: usize) -> ValueStack<'a> {
        ValueStack {
            stack: Vec::with_capacity(max_size),
        }
    }

    pub(crate) fn pop(&mut self) -> Result<Value<'a>> {
        self.stack.pop().ok_or(Exception::ExecuteCodeError(Box::new(
            ValueStackFault::EmptyStack,
        )))
    }

    pub(crate) fn push(&mut self, value: Value<'a>) -> Result<()> {
        if self.stack.len() < self.stack.capacity() {
            self.stack.push(value);
            Ok(())
        } else {
            Err(Exception::ExecuteCodeError(Box::new(
                ValueStackFault::StackOverFlow,
            )))
        }
    }
}
