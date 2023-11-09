use crate::jvm_error::{VmError, VmExecResult};
use crate::jvm_values::Value;
use log::trace;

#[derive(Debug)]
pub struct OperandStack<'a> {
    stack: Vec<Value<'a>>,
}
impl<'a> OperandStack<'a> {
    pub(crate) fn new(max_size: usize) -> OperandStack<'a> {
        OperandStack {
            stack: Vec::with_capacity(max_size),
        }
    }
    pub(crate) fn pop_n(&mut self, n: usize) -> VmExecResult<Vec<Value<'a>>> {
        let mut vec = Vec::with_capacity(n);
        (0..n).for_each(|_| vec.push(Value::Null));
        for i in 1..=n {
            vec[n - i] = self.pop()?
        }
        Ok(vec)
    }
    pub(crate) fn pop(&mut self) -> VmExecResult<Value<'a>> {
        let result = self.stack.pop().ok_or(VmError::PopFromEmptyStack);
        trace!("--- value stack --- {:?}", self.stack);
        result
    }

    pub(crate) fn push(&mut self, value: Value<'a>) -> VmExecResult<()> {
        if self.stack.len() < self.stack.capacity() {
            self.stack.push(value);
            trace!("--- value stack --- {:?}", self.stack);
            Ok(())
        } else {
            Err(VmError::StackOverFlow)
        }
    }

    pub fn dup(&mut self) -> VmExecResult<()> {
        match self.stack.last() {
            None => Err(VmError::PopFromEmptyStack),
            Some(head) => self.push(head.clone()),
        }
    }

    pub fn dup_x1(&mut self) -> VmExecResult<()> {
        let value1 = self.pop()?;
        let value2 = self.pop()?;
        self.push(value1.clone())?;
        self.push(value2)?;
        self.push(value1)
    }

    pub fn dup_x2(&mut self) -> VmExecResult<()> {
        let value1 = self.pop()?;
        let value2 = self.pop()?;
        let value3 = self.pop()?;
        self.push(value1.clone())?;
        self.push(value3)?;
        self.push(value2)?;
        self.push(value1)
    }

    pub fn dup2(&mut self) -> VmExecResult<()> {
        let value1 = self.pop()?;
        let value2 = self.pop()?;
        self.push(value2.clone())?;
        self.push(value1.clone())?;
        self.push(value2)?;
        self.push(value1)
    }

    pub fn dup2_x1(&mut self) -> VmExecResult<()> {
        let value1 = self.pop()?;
        let value2 = self.pop()?;
        let value3 = self.pop()?;
        self.push(value2.clone())?;
        self.push(value1.clone())?;
        self.push(value3)?;
        self.push(value2)?;
        self.push(value1)
    }

    pub fn dup2_x2(&mut self) -> VmExecResult<()> {
        let value1 = self.pop()?;
        let value2 = self.pop()?;
        let value3 = self.pop()?;
        let value4 = self.pop()?;
        self.push(value2.clone())?;
        self.push(value1.clone())?;
        self.push(value4)?;
        self.push(value3)?;
        self.push(value2)?;
        self.push(value1)
    }

    pub fn swap(&mut self) -> VmExecResult<()> {
        let value1 = self.pop()?;
        let value2 = self.pop()?;
        self.push(value1)?;
        self.push(value2)
    }
}
