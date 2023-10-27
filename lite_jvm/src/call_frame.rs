use crate::call_frame::InstructionResult::ContinueMethodExecution;
use crate::call_stack::CallStack;
use crate::java_exception::{InvokeMethodResult, MethodCallError};
use crate::jvm_error::{VmError, VmExecResult};
use crate::loaded_class::{ClassRef, MethodRef};
use crate::reference_value::Value::{
    ArrayRef, Byte, Char, Double, Float, Int, Long, Null, ObjectRef, Short,
};
use crate::reference_value::{
    ArrayElement, ArrayReference, ObjectReference, ReferenceValue, Value,
};
use crate::value_stack::ValueStack;
use crate::virtual_machine::VirtualMachine;
use class_file_reader::cesu8_byte_buffer::ByteBuffer;
use class_file_reader::instruction::{read_one_instruction, Instruction};
use std::ops::{BitAnd, Div};

pub(crate) enum InstructionResult<'a> {
    ReturnFromMethod(Option<Value<'a>>),
    ContinueMethodExecution,
}

pub struct CallFrame<'a> {
    pub(crate) class_ref: ClassRef<'a>,
    pub(crate) method_ref: MethodRef<'a>,
    //复用bytebuffer。包含了pc和code
    pub(crate) byte_buffer: ByteBuffer<'a>,
    // pc: ProgramCounter,
    pub(crate) local_variables: Vec<Value<'a>>,
    pub(crate) stack: ValueStack<'a>,
}

fn is_double_division_returning_nan(a: f64, b: f64) -> bool {
    a.is_nan()
        || b.is_nan()
        || (a.is_infinite() && b.is_infinite())
        || ((a == 0f64 || a == -0f64) && (b == 0f64 || b == -0f64))
}

macro_rules! generate_pop {
    ($name:ident, $variant:ident, $type:ty) => {
        fn $name(&mut self) -> VmExecResult<$type> {
            let value = self.pop()?;
            match value {
                Value::$variant(value) => Ok(value),
                _ => Err(VmError::ValueTypeMissMatch),
            }
        }
    };
}

macro_rules! generate_array_load {
    ($name:ident,$($variant:ident),+) => {
        fn $name(&mut self) -> VmExecResult<()> {
            let index = self.pop_int()? as usize;
            let array = self.pop_array()?;
            let value = array.get_field_by_offset(index)?;
            match value {
                $(Value::$variant(_) => {
                   self.push(value)
                })+
                _=>  Err(VmError::ValueTypeMissMatch)
            }
        }
    };
}

macro_rules! generate_array_store {
    ($name:ident, $($variant:ident),+) => {
        fn $name(&mut self) -> VmExecResult<()> {
            let value = self.pop()?;
            let index = self.pop_int()? as usize;
            let array = self.pop_array()?;
             match value {
                $(Value::$variant(_) => {
                  array.set_field_by_offset(index, &value)
                })+
                _=>  Err(VmError::ValueTypeMissMatch)
            }

        }
    };
}
macro_rules! generate_return {
    ($name:ident, $variant:ident) => {
        fn $name(&mut self) -> Result<InstructionResult<'a>, MethodCallError> {
            let value = self.pop()?;
            match value {
                $variant(..) => Ok(InstructionResult::ReturnFromMethod(Some(value))),
                _ => Err(MethodCallError::from(VmError::ValueTypeMissMatch)),
            }
        }
    };
}

macro_rules! generate_load {
     ($name:ident, $($variant:ident),+) => {
        fn $name(&mut self, index: u8) -> VmExecResult<()> {
            let local = self.local_variables.get(index as usize).ok_or(VmError::ValueTypeMissMatch)?;
            match local {
                $($variant(..) => {
                    self.push(local.clone())
                }),+
                _ => Err(VmError::ValueTypeMissMatch),
            }
        }
    };
}

macro_rules! generate_store {
    ($name:ident, $($variant:ident),+) => {
        fn $name(&mut self, index: u8) -> VmExecResult<()> {
            let object_ref = self.pop_object_or_null()?;
            self.local_variables
                .insert(index as usize, object_ref.clone());
            Ok(())
        }
    };
}

macro_rules! generate_convert {
    ($name:ident, $variant:ident, $target:ident, $type:ty) => {
        fn $name(&mut self) -> VmExecResult<()> {
            let value = self.pop()?;
            if let $variant(v) = value {
                self.push($target(v as $type))
            } else {
                Err(VmError::ExecuteCodeError("convert Error".to_string()))
            }
        }
    };
}

macro_rules! generate_math {
    ($name:ident, $variant:ident, $type:ty) => {
        fn $name<T>(&mut self, evaluator: T) -> VmExecResult<()>
        where
            T: FnOnce($type, $type) -> $type,
        {
            let val2 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(VmError::ValueTypeMissMatch);
            };
            let val1 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(VmError::ValueTypeMissMatch);
            };
            let result = evaluator(val1, val2);
            self.push($variant(result))
        }
    };
}

macro_rules! generate_cmp {
    ($name:ident, $variant:ident,$type:ty) => {
        fn $name(&mut self, greater_result: i32) -> VmExecResult<()> {
            let val2 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(VmError::ValueTypeMissMatch);
            };
            let val1 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(VmError::ValueTypeMissMatch);
            };
            let result = val1 - val2;
            let value = if result > 0 as $type {
                greater_result
            } else if result < 0 as $type {
                0 - greater_result
            } else {
                0
            };
            self.push(Int(value))
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
    generate_load!(exec_dload, Double);
    generate_load!(exec_fload, Float);

    generate_store!(exec_astore, ObjectRef);
    generate_store!(exec_dstore, Double);
    generate_store!(exec_fstore, Float);

    generate_convert!(exec_d2f, Double, Float, f32);
    generate_convert!(exec_d2l, Double, Long, i64);
    generate_convert!(exec_d2i, Double, Int, i32);
    generate_convert!(exec_f2d, Float, Double, f64);
    generate_convert!(exec_f2i, Float, Int, i32);
    generate_convert!(exec_f2l, Float, Long, i64);

    generate_convert!(exec_i2b, Int, Byte, i8);
    generate_convert!(exec_i2c, Int, Char, u16);
    generate_convert!(exec_i2d, Int, Double, f64);
    generate_convert!(exec_i2f, Int, Float, f32);
    generate_convert!(exec_i2l, Int, Long, i64);
    generate_convert!(exec_i2s, Int, Short, i16);

    generate_math!(exec_double_math, Double, f64);
    generate_math!(exec_float_math, Float, f32);
    generate_math!(exec_int_math, Int, i32);
    generate_math!(exec_long_math, Long, i64);

    generate_cmp!(exec_dcmp, Double, f64);
    generate_cmp!(exec_fcmp, Float, f32);

    generate_return!(exec_dreturn, Double);
    generate_return!(exec_freturn, Float);
    generate_return!(exec_lreturn, Long);

    fn pop_array(&mut self) -> VmExecResult<ArrayReference<'a>> {
        if let ArrayRef(v) = self.pop()? {
            Ok(v)
        } else {
            Err(VmError::ExecuteCodeError("ShouldBeArray".to_string()))
        }
    }

    fn pop_array_or_null(&mut self) -> VmExecResult<Value<'a>> {
        let value = self.pop()?;
        if let ArrayRef(_) | Null = value {
            Ok(value)
        } else {
            Err(VmError::ExecuteCodeError("ShouldBeArrayOrNull".to_string()))
        }
    }
    fn pop_object_or_null(&mut self) -> VmExecResult<Value<'a>> {
        let value = self.pop()?;
        if let ObjectRef(_) | Null = value {
            Ok(value)
        } else {
            Err(VmError::ExecuteCodeError(
                "ShouldBeObjectOrNull".to_string(),
            ))
        }
    }

    fn pop_object(&mut self) -> VmExecResult<ObjectReference<'a>> {
        if let ObjectRef(v) = self.pop()? {
            Ok(v)
        } else {
            Err(VmError::ExecuteCodeError("ShouldBeObject".to_string()))
        }
    }

    fn pop(&mut self) -> VmExecResult<Value<'a>> {
        self.stack.pop()
    }

    fn push(&mut self, value: Value<'a>) -> VmExecResult<()> {
        self.stack.push(value)
    }

    fn get_class_name_in_constant_pool(&self, index: u16) -> VmExecResult<&str> {
        self.class_ref.constant_pool.get_class_name(index)
    }

    fn get_field_in_constant_pool(&self, index: u16) -> VmExecResult<(&str, &str, &str)> {
        self.class_ref.constant_pool.get_field_name(index)
    }

    fn exec_areturn(&mut self) -> Result<InstructionResult<'a>, MethodCallError> {
        let value = self.pop()?;
        match value {
            ObjectRef(_) | ArrayRef(_) | Null => {
                Ok(InstructionResult::ReturnFromMethod(Some(value)))
            }
            _ => Err(MethodCallError::from(VmError::ExecuteCodeError(
                "Should be a reference or null".to_string(),
            ))),
        }
    }

    fn exec_anewarray(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        constant_index: u16,
    ) -> VmExecResult<()> {
        let length = self.pop_int()? as usize;
        let class_name = self.get_class_name_in_constant_pool(constant_index)?;
        let class = vm.lookup_class(class_name)?;
        let array = vm.new_array(ArrayElement::ClassReference(class), length);
        self.push(Value::ArrayRef(array))
    }

    fn exec_arraylength(&mut self) -> VmExecResult<()> {
        let array = self.pop_array()?;
        let length = array.get_data_length();
        self.push(Value::Int(length as i32))
    }

    fn exec_athrow(&mut self) -> Result<InstructionResult<'a>, MethodCallError> {
        let value = self.pop_object()?;
        assert!(value.get_class().is_subclass_of("java/lang/Throwable"));
        Err(MethodCallError::ExceptionThrown(value))
    }

    //需要支持数组
    fn check_instance_of(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        constant_pool_index: u16,
        value: &Value<'a>,
    ) -> VmExecResult<bool> {
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
    ) -> Result<InstructionResult<'a>, MethodCallError> {
        match instruction {
            Instruction::Aaload => self.exec_aaload()?,
            Instruction::Aastore => self.exec_aastore()?,
            Instruction::Aconst_null => self.stack.push(Value::Null)?,
            Instruction::Aload(local_index) => self.exec_aload(local_index)?,
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
            Instruction::Astore(local_index) => self.exec_astore(local_index)?,
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
            Instruction::Checkcast(constant_pool_index) => {
                let value = self.pop()?;
                let is_instance_of = self.check_instance_of(vm, constant_pool_index, &value)?;
                if is_instance_of {
                    self.push(value)?
                } else {
                    return Err(MethodCallError::from(VmError::ValueTypeMissMatch));
                }
            }
            Instruction::D2f => self.exec_d2f()?,
            Instruction::D2i => self.exec_d2i()?,
            Instruction::D2l => self.exec_d2l()?,
            Instruction::Dadd => self.exec_double_math(|v1, v2| v1 + v2)?,
            Instruction::Daload => self.exec_daload()?,
            Instruction::Dastore => self.exec_dastore()?,
            Instruction::Dcmpg => self.exec_dcmp(-1)?,
            Instruction::Dcmpl => self.exec_dcmp(1)?,
            Instruction::Dconst_0 => self.push(Double(0f64))?,
            Instruction::Dconst_1 => self.push(Double(1f64))?,
            Instruction::Ddiv => self.exec_double_math(|v1, v2| {
                if is_double_division_returning_nan(v1, v2) {
                    f64::NAN
                } else {
                    v1 / v2
                }
            })?,
            Instruction::Dload(local_index) => self.exec_dload(local_index)?,
            Instruction::Dload_0 => self.exec_dload(0)?,
            Instruction::Dload_1 => self.exec_dload(1)?,
            Instruction::Dload_2 => self.exec_dload(2)?,
            Instruction::Dload_3 => self.exec_dload(3)?,
            Instruction::Dmul => self.exec_double_math(|v1, v2| v1 * v2)?,
            Instruction::Dneg => {
                let value = self.pop_double()?;
                self.push(Double(-value))?;
            }
            Instruction::Drem => self.exec_double_math(|v1, v2| {
                if is_double_division_returning_nan(v1, v2) {
                    f64::NAN
                } else {
                    v1 % v2
                }
            })?,
            Instruction::Dreturn => return self.exec_dreturn(),
            Instruction::Dstore(local_index) => self.exec_dstore(local_index)?,
            Instruction::Dstore_0 => self.exec_dstore(0)?,
            Instruction::Dstore_1 => self.exec_dstore(1)?,
            Instruction::Dstore_2 => self.exec_dstore(2)?,
            Instruction::Dstore_3 => self.exec_dstore(3)?,
            Instruction::Dsub => self.exec_double_math(|v1, v2| v1 - v2)?,
            Instruction::Dup => self.stack.dup()?,
            Instruction::Dup_x1 => self.stack.dup_x1()?,
            Instruction::Dup_x2 => self.stack.dup_x2()?,
            Instruction::Dup2 => self.stack.dup2()?,
            Instruction::Dup2_x1 => self.stack.dup2_x1()?,
            Instruction::Dup2_x2 => self.stack.dup2_x2()?,
            Instruction::F2d => self.exec_f2d()?,
            Instruction::F2i => self.exec_f2i()?,
            Instruction::F2l => self.exec_f2l()?,
            Instruction::Fadd => self.exec_float_math(|v1, v2| v1 + v2)?,
            Instruction::Faload => self.exec_faload()?,
            Instruction::Fastore => self.exec_fastore()?,
            Instruction::Fcmpl => self.exec_fcmp(1)?,
            Instruction::Fcmpg => self.exec_fcmp(-1)?,
            Instruction::Fconst_0 => self.push(Float(0f32))?,
            Instruction::Fconst_1 => self.push(Float(1f32))?,
            Instruction::Fconst_2 => self.push(Float(2f32))?,
            Instruction::Fdiv => self.exec_float_math(|v1, v2| {
                if is_double_division_returning_nan(v1 as f64, v2 as f64) {
                    f32::NAN
                } else {
                    v1 / v2
                }
            })?,
            Instruction::Fload(local_index) => self.exec_fload(local_index)?,
            Instruction::Fload_0 => self.exec_fload(0)?,
            Instruction::Fload_1 => self.exec_fload(1)?,
            Instruction::Fload_2 => self.exec_fload(2)?,
            Instruction::Fload_3 => self.exec_fload(3)?,
            Instruction::Fmul => self.exec_float_math(|v1, v2| v1 * v2)?,
            Instruction::Fneg => {
                let v = self.pop_float()?;
                self.push(Float(-v))?;
            }
            Instruction::Frem => self.exec_float_math(|v1, v2| {
                if is_double_division_returning_nan(v1 as f64, v2 as f64) {
                    f32::NAN
                } else {
                    v1 % v2
                }
            })?,
            Instruction::Freturn => return self.exec_freturn(),
            Instruction::Fstore(local_index) => self.exec_fstore(local_index)?,
            Instruction::Fstore_0 => self.exec_fstore(0)?,
            Instruction::Fstore_1 => self.exec_fstore(1)?,
            Instruction::Fstore_2 => self.exec_fstore(2)?,
            Instruction::Fstore_3 => self.exec_fstore(3)?,
            Instruction::Fsub => self.exec_float_math(|v1, v2| v1 - v2)?,
            Instruction::Getfield(const_pool_index) => self.exec_get_field(const_pool_index)?,
            Instruction::Getstatic(const_pool_index) => {
                self.exec_get_static(vm, const_pool_index)?
            }
            Instruction::Goto(code_position) => self.goto(code_position as usize),
            Instruction::Goto_w(code_position) => self.goto(code_position as usize),
            Instruction::I2b => self.exec_i2b()?,
            Instruction::I2c => self.exec_i2c()?,
            Instruction::I2d => self.exec_i2d()?,
            Instruction::I2f => self.exec_i2f()?,
            Instruction::I2l => self.exec_i2l()?,
            Instruction::I2s => self.exec_i2s()?,
            Instruction::Iadd => self.exec_int_math(|i1, i2| i1 + i2)?,
            Instruction::Iaload => self.exec_iaload()?,
            Instruction::Iand => self.exec_int_math(|i1, i2| i1.bitand(i2))?,
            Instruction::Iastore => self.exec_iastore()?,
            Instruction::Iconst_m1 => self.push(Int(-1))?,
            Instruction::Iconst_0 => self.push(Int(0))?,
            Instruction::Iconst_1 => self.push(Int(1))?,
            Instruction::Iconst_2 => self.push(Int(2))?,
            Instruction::Iconst_3 => self.push(Int(3))?,
            Instruction::Iconst_4 => self.push(Int(4))?,
            Instruction::Iconst_5 => self.push(Int(5))?,
            //TODO 除以0异常，
            Instruction::Idiv => self.exec_int_math(|i1, i2| i1.div(i2))?,
        }

        Ok(ContinueMethodExecution)
    }

    fn goto(&mut self, new_pc: usize) {
        self.byte_buffer.jump_to(new_pc);
    }

    fn exec_get_field(&mut self, field_index: u16) -> VmExecResult<()> {
        let object = self.pop()?;
        if let ObjectRef(object_ref) = object {
            let (class_name, field_name, _descriptor) =
                self.get_field_in_constant_pool(field_index)?;
            let class_ref = object_ref.get_class();
            //TODO 校验描述符类型
            assert_eq!(class_ref.name, class_name);
            let field_value = object_ref.get_field_by_name(field_name)?;
            return self.push(field_value);
        }
        Err(VmError::MethodNotFoundException("".to_string()))
    }

    fn exec_get_static(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        field_index: u16,
    ) -> VmExecResult<()> {
        let (class_name, field_name, _descriptor) = self.get_field_in_constant_pool(field_index)?;
        let value = vm.get_static_field(class_name, field_name)?;
        self.push(value)
    }

    pub fn execute(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
    ) -> InvokeMethodResult<'a> {
        loop {
            let instruction = read_one_instruction(&mut self.byte_buffer).unwrap();
            let result = self.execute_instruction(vm, call_stack, instruction);
            //TODO 处理异常情况
            if let Ok(InstructionResult::ReturnFromMethod(return_value)) = result {
                return Ok(return_value);
            }
        }
    }
}

mod tests {

    #[test]
    fn test_instruction() {}
}
