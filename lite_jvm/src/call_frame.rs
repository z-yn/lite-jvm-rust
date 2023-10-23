use crate::call_frame::InstructionResult::ContinueMethodExecution;
use crate::call_stack::CallStack;
use crate::jvm_exceptions::{Exception, Result};
use crate::loaded_class::{ClassRef, MethodRef};
use crate::program_counter::ProgramCounter;
use crate::referenced_value::Value;
use crate::virtual_machine::VirtualMachine;
use class_file_reader::instruction::{read_one_instruction, Instruction};
use thiserror::Error;

pub enum MethodReturnValue<'a> {
    SuccessReturn(Option<Value<'a>>),
    ThrowException(String),
}

pub(crate) enum InstructionResult<'a> {
    ReturnFromMethod(MethodReturnValue<'a>),
    ContinueMethodExecution,
}

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
pub struct CallFrame<'a> {
    class_ref: ClassRef<'a>,
    method_ref: MethodRef<'a>,
    //复用bytebuffer。包含了pc和code
    code: &'a [u8],
    pc: ProgramCounter,
    local_variables: Vec<Value<'a>>,
    stack: ValueStack<'a>,
}

impl<'a> CallFrame<'a> {
    pub fn new(class_ref: ClassRef<'a>, method_ref: MethodRef<'a>) -> CallFrame<'a> {
        let code_attr = method_ref.code.as_ref().expect("Should Has Code");
        CallFrame {
            class_ref,
            method_ref,
            code: &code_attr.code,
            pc: ProgramCounter(0),
            local_variables: vec![],
            stack: ValueStack::new(code_attr.max_stack as usize),
        }
    }

    fn pop(&mut self) -> Result<Value<'a>> {
        self.stack.pop()
    }

    fn push(&mut self, value: Value<'a>) -> Result<()> {
        self.stack.push(value)
    }

    fn execute_instruction(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
        instruction: Instruction,
    ) -> Result<InstructionResult<'a>> {
        match instruction {
            Instruction::Aaload => {}
            Instruction::Aastore => {}
        }

        Ok(ContinueMethodExecution)
    }
    pub fn execute(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
    ) -> Result<MethodReturnValue> {
        loop {
            let instruction = read_one_instruction(self.code).unwrap();
            let result = self.execute_instruction(vm, call_stack, instruction);
            if let Ok(InstructionResult::ReturnFromMethod(return_value)) = result {
                return Ok(return_value);
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MethodCallFailed {
    InternalError,
    ExceptionThrown,
}
