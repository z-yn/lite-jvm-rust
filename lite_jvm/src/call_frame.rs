use crate::call_frame::InstructionResult::ContinueMethodExecution;
use crate::call_stack::CallStack;
use crate::jvm_exceptions::{Exception, Result};
use crate::loaded_class::{ClassRef, MethodRef};
use crate::program_counter::ProgramCounter;
use crate::reference_value::{ArrayReference, ReferenceValue, Value};
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

/// Pops a Value of the appropriate type from the stack
macro_rules! generate_pop {
    ($name:ident, $variant:ident, $type:ty) => {
        fn $name(&mut self) -> Result<$type> {
            let value = self.pop()?;
            match value {
                Value::$variant(value) => Ok(value),
                _ => Err(Exception::ExecuteCodeError(Box::new(
                    MethodCallFailed::InternalError,
                ))),
            }
        }
    };
}

macro_rules! generate_array_load {
    ($name:ident, $variant:ident) => {
        fn $name(&mut self) -> Result<()> {
            let index = self.pop_int()? as usize;
            let array = self.pop_array()?;
            let value = array.get_field_by_offset(index)?;
            return if let Value::$variant(_) = value {
                self.push(value)
            } else {
                Err(Exception::ExecuteCodeError(Box::new(
                    MethodCallFailed::InternalError,
                )))
            }
        }
    };
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

    generate_pop!(pop_int, Int, i32);
    generate_pop!(pop_long, Long, i64);
    generate_pop!(pop_float, Float, f32);
    generate_pop!(pop_double, Double, f64);

    fn exec_aaload(&mut self) -> Result<()> {
        let index = self.pop_int()? as usize;
        let array = self.pop_array()?;
        let value = array.get_field_by_offset(index)?;
        return if let Value::ObjectRef(v) = value {
            self.push(value.clone())
        } else {
            Err(Exception::ExecuteCodeError(Box::new(
                MethodCallFailed::InternalError,
            )))
        };
    }
    // generate_array_load!(exec_aaload, ObjectRef);
    // generate_array_load!(exec_caload, Char);
    // generate_array_load!(exec_saload, Short);
    // generate_array_load!(exec_iaload, Int);
    // generate_array_load!(exec_laload, Long);
    // generate_array_load!(exec_faload, Float);
    // generate_array_load!(exec_daload, Double);

    fn pop_array(&mut self) -> Result<ArrayReference<'a>> {
        if let Value::ArrayRef(ref_value) = self.pop()? {
            Ok(ref_value)
        } else {
            Err(Exception::ExecuteCodeError(Box::new(
                MethodCallFailed::InternalError,
            )))
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
            Instruction::Aaload => self.exec_aaload()?,
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

#[derive(Error, Debug, PartialEq)]
pub enum MethodCallFailed {
    #[error("InternalError")]
    InternalError,
    #[error("ExceptionThrown")]
    ExceptionThrown,
}
