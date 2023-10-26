use crate::call_frame::InstructionResult::ContinueMethodExecution;
use crate::call_stack::CallStack;
use crate::jvm_exceptions::{Exception, Result};
use crate::loaded_class::{ClassRef, MethodRef};
use crate::reference_value::Value::{
    ArrayRef, Boolean, Byte, Char, Double, Float, Int, Long, Null, ObjectRef, Short,
};
use crate::reference_value::{
    ArrayElement, ArrayReference, ObjectReference, ReferenceValue, Value,
};
use crate::value_stack::ValueStack;
use crate::virtual_machine::VirtualMachine;
use class_file_reader::cesu8_byte_buffer::ByteBuffer;
use class_file_reader::instruction::{read_one_instruction, Instruction};
use thiserror::Error;

pub enum MethodReturnValue<'a> {
    SuccessReturn(Option<Value<'a>>),
    ThrowException(ObjectReference<'a>),
}

pub(crate) enum InstructionResult<'a> {
    ReturnFromMethod(MethodReturnValue<'a>),
    ContinueMethodExecution,
    JumpTo(usize),
}

pub struct CallFrame<'a> {
    class_ref: ClassRef<'a>,
    method_ref: MethodRef<'a>,
    //复用bytebuffer。包含了pc和code
    byte_buffer: ByteBuffer<'a>,
    // pc: ProgramCounter,
    local_variables: Vec<Value<'a>>,
    stack: ValueStack<'a>,
}

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
    ($name:ident,$($variant:ident),+) => {
        fn $name(&mut self) -> Result<()> {
            let index = self.pop_int()? as usize;
            let array = self.pop_array()?;
            let value = array.get_field_by_offset(index)?;
            match value {
                $(Value::$variant(_) => {
                   self.push(value)
                })+
                _=>  Err(Exception::ExecuteCodeError(Box::new(
                    MethodCallFailed::InternalError,
                )))
            }
        }
    };
}

macro_rules! generate_array_store {
    ($name:ident, $($variant:ident),+) => {
        fn $name(&mut self) -> Result<()> {
            let value = self.pop()?;
            let index = self.pop_int()? as usize;
            let array = self.pop_array()?;
             match value {
                $(Value::$variant(_) => {
                  array.set_field_by_offset(index, &value)
                })+
                _=>  Err(Exception::ExecuteCodeError(Box::new(
                    MethodCallFailed::InternalError,
                )))
            }

        }
    };
}
macro_rules! generate_return {
    ($name:ident, $variant:ident) => {
        fn $name(&mut self, index: usize) -> Result<InstructionResult<'a>> {
            let value = self.pop()?;
            match value {
                $variant(..) => Ok(InstructionResult::ReturnFromMethod(
                    MethodReturnValue::SuccessReturn(Some(value)),
                )),
                _ => Err(Exception::ExecuteCodeError(Box::new(
                    MethodCallFailed::InternalError,
                ))),
            }
        }
    };
}

macro_rules! generate_load {
     ($name:ident, $($variant:ident),+) => {
        fn $name(&mut self, index: usize) -> Result<()> {
            let local = self.local_variables.get(index).ok_or(Exception::ExecuteCodeError(Box::new(MethodCallFailed::InternalError)))?;
            match local {
                $($variant(..) => {
                    self.push(local.clone())
                }),+
                _ => Err(Exception::ExecuteCodeError(Box::new(MethodCallFailed::InternalError))),
            }
        }
    };
}

macro_rules! generate_store {
    ($name:ident, $($variant:ident),+) => {
        fn $name(&mut self, index: usize) -> Result<()> {
            let object_ref = self.pop_object_or_null()?;
            self.local_variables.insert(index, object_ref.clone());
            Ok(())
        }
    };
}

impl<'a> CallFrame<'a> {
    pub fn new(class_ref: ClassRef<'a>, method_ref: MethodRef<'a>) -> CallFrame<'a> {
        let code_attr = method_ref.code.as_ref().expect("Should Has Code");
        CallFrame {
            class_ref,
            method_ref,
            byte_buffer: ByteBuffer::new(&code_attr.code),
            // pc: ProgramCounter(0),
            local_variables: vec![],
            stack: ValueStack::new(code_attr.max_stack as usize),
        }
    }

    generate_pop!(pop_int, Int, i32);
    generate_pop!(pop_long, Long, i64);
    generate_pop!(pop_float, Float, f32);
    generate_pop!(pop_double, Double, f64);

    generate_array_load!(exec_aaload, ObjectRef);
    generate_array_load!(exec_caload, Char);
    generate_array_load!(exec_saload, Short);
    generate_array_load!(exec_iaload, Int);
    generate_array_load!(exec_laload, Long);
    generate_array_load!(exec_faload, Float);
    generate_array_load!(exec_daload, Double);
    generate_array_load!(exec_baload, Boolean, Byte);

    generate_array_store!(exec_aastore, ObjectRef);
    generate_array_store!(exec_castore, Char);
    generate_array_store!(exec_sastore, Short);
    generate_array_store!(exec_iastore, Int);
    generate_array_store!(exec_lastore, Long);
    generate_array_store!(exec_fastore, Float);
    generate_array_store!(exec_dastore, Double);
    generate_array_store!(exec_bastore, Boolean, Byte);

    generate_load!(exec_aload, ObjectRef);

    generate_store!(exec_astore, ObjectRef);

    fn pop_array(&mut self) -> Result<ArrayReference<'a>> {
        if let ArrayRef(v) = self.pop()? {
            Ok(v)
        } else {
            Err(Exception::ExecuteCodeError(Box::new(
                MethodCallFailed::InternalError,
            )))
        }
    }

    fn pop_array_or_null(&mut self) -> Result<Value<'a>> {
        let value = self.pop()?;
        if let ArrayRef(_) | Null = value {
            Ok(value)
        } else {
            Err(Exception::ExecuteCodeError(Box::new(
                MethodCallFailed::InternalError,
            )))
        }
    }
    fn pop_object_or_null(&mut self) -> Result<Value<'a>> {
        let value = self.pop()?;
        if let ObjectRef(_) | Null = value {
            Ok(value)
        } else {
            Err(Exception::ExecuteCodeError(Box::new(
                MethodCallFailed::InternalError,
            )))
        }
    }

    fn pop_object(&mut self) -> Result<ObjectReference<'a>> {
        if let ObjectRef(v) = self.pop()? {
            Ok(v)
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

    fn get_class_name_in_constant_pool(&self, index: u16) -> Result<&str> {
        self.class_ref.constant_pool.get_class_name(index)
    }

    fn exec_areturn(&mut self) -> Result<InstructionResult<'a>> {
        let value = self.pop()?;
        match value {
            ObjectRef(_) | ArrayRef(_) | Null => Ok(InstructionResult::ReturnFromMethod(
                MethodReturnValue::SuccessReturn(Some(value)),
            )),
            _ => Err(Exception::ExecuteCodeError(Box::new(
                MethodCallFailed::InternalError,
            ))),
        }
    }

    fn exec_anewarray(&mut self, vm: &mut VirtualMachine<'a>, constant_index: u16) -> Result<()> {
        let length = self.pop_int()? as usize;
        let class_name = self.get_class_name_in_constant_pool(constant_index)?;
        let class = vm.lookup_class(class_name)?;
        let array = vm.new_array(ArrayElement::ClassReference(class), length);
        self.push(Value::ArrayRef(array))
    }

    fn exec_arraylength(&mut self) -> Result<()> {
        let array = self.pop_array()?;
        let length = array.get_data_length();
        self.push(Value::Int(length as i32))
    }

    fn exec_athrow(&mut self) -> Result<InstructionResult<'a>> {
        let value = self.pop_object()?;
        assert!(value.get_class().is_subclass_of("java/lang/Throwable"));
        Ok(InstructionResult::ReturnFromMethod(
            MethodReturnValue::ThrowException(value),
        ))
    }

    //需要支持数组
    fn check_instance_of(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        constant_pool_index: u16,
        value: Value<'a>,
    ) -> Result<bool> {
        let class_name = self.get_class_name_in_constant_pool(constant_pool_index)?;
        //TODO 数组类，目前先解析了一级数组。后续需要使用递归处理内部类型
        let (is_array, target_class_ref, array_class) =
            if class_name.starts_with("[L") && class_name.ends_with(';') {
                (
                    true,
                    None,
                    Some(ArrayElement::ClassReference(
                        vm.lookup_class(&class_name[2..class_name.len() - 1])?,
                    )),
                )
            } else {
                (false, Some(vm.lookup_class(class_name)?), None)
            };
        let result = match value {
            Null => false,
            ObjectRef(class_ref) => {
                if is_array {
                    false
                } else {
                    class_ref.is_instance_of(target_class_ref.unwrap())
                }
            }
            ArrayRef(array_ref) => {
                if is_array {
                    array_ref.is_instance_of(&array_class.unwrap())
                } else {
                    false
                }
            }
            _ => false,
        };
        Ok(result)
    }

    fn execute_instruction(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
        instruction: Instruction,
    ) -> Result<InstructionResult<'a>> {
        match instruction {
            Instruction::Aaload => self.exec_aaload()?,
            Instruction::Aastore => self.exec_aastore()?,
            Instruction::Aconst_null => self.stack.push(Value::Null)?,
            Instruction::Aload(local_index) => self.exec_aload(local_index as usize)?,
            Instruction::Aload_0 => self.exec_aload(0)?,
            Instruction::Aload_1 => self.exec_aload(1)?,
            Instruction::Aload_2 => self.exec_aload(2)?,
            Instruction::Aload_3 => self.exec_aload(3)?,
            Instruction::Anewarray(constant_pool_offset) => {
                self.exec_anewarray(vm, constant_pool_offset)?
            }
            Instruction::Areturn => {
                return self.exec_areturn();
            }
            Instruction::Arraylength => self.exec_arraylength()?,
            Instruction::Astore(local_index) => self.exec_astore(local_index as usize)?,
            Instruction::Astore_0 => self.exec_astore(0)?,
            Instruction::Astore_1 => self.exec_astore(1)?,
            Instruction::Astore_2 => self.exec_astore(2)?,
            Instruction::Astore_3 => self.exec_astore(3)?,
            Instruction::Athrow => {
                return self.exec_athrow();
            }
            Instruction::Baload => self.exec_baload()?,
            Instruction::Bastore => self.exec_bastore()?,
            Instruction::Bipush(byte_value) => self.push(Int(byte_value as i32))?,
            Instruction::Caload => self.exec_caload()?,
            Instruction::Castore => self.exec_castore()?,
            Instruction::Checkcast(constant_pool_index) => {}
        }

        Ok(ContinueMethodExecution)
    }

    pub fn execute(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
    ) -> Result<MethodReturnValue> {
        loop {
            let instruction = read_one_instruction(&mut self.byte_buffer).unwrap();
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

mod tests {

    #[test]
    fn test_instruction() {}
}
