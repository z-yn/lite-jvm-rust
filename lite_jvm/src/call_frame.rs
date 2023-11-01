use crate::call_frame::InstructionResult::{ContinueMethodExecution, ReturnFromMethod};
use crate::call_stack::CallStack;
use crate::java_exception::{InvokeMethodResult, MethodCallError};
use crate::jvm_error::VmError::ValueTypeMissMatch;
use crate::jvm_error::{VmError, VmExecResult};
use crate::loaded_class::{ClassRef, MethodRef};
use crate::reference_value::Value::{
    ArrayRef, Boolean, Byte, Char, Double, Float, Int, Long, Null, ObjectRef, ReturnAddress, Short,
    Uninitialized,
};
use crate::reference_value::{
    ArrayElement, ArrayReference, ObjectReference, PrimaryType, ReferenceValue, Value,
};
use crate::runtime_constant_pool::RuntimeConstantPoolEntry;
use crate::value_stack::ValueStack;
use crate::virtual_machine::VirtualMachine;
use class_file_reader::cesu8_byte_buffer::ByteBuffer;
use class_file_reader::instruction::{read_one_instruction, Instruction};
use std::ops::{BitAnd, BitOr, BitXor, Div, Mul, Rem, Shl, Shr, Sub};

#[derive(Debug)]
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

type InvokeResult<'a, T> = Result<T, MethodCallError<'a>>;

fn is_double_division_returning_nan(a: f64, b: f64) -> bool {
    a.is_nan()
        || b.is_nan()
        || (a.is_infinite() && b.is_infinite())
        || ((a == 0f64 || a == -0f64) && (b == 0f64 || b == -0f64))
}

macro_rules! generate_get_local {
    ($name:ident, $variant:ident, $type:ty) => {
        fn $name(&mut self, index: u8) -> InvokeResult<'a, $type> {
            let value = self.get_local_value(index)?;
            match value {
                Value::$variant(value) => Ok(value),
                _ => Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch)),
            }
        }
    };
}

macro_rules! generate_pop {
    ($name:ident, $variant:ident, $type:ty) => {
        fn $name(&mut self) -> InvokeResult<'a, $type> {
            let value = self.pop()?;
            match value {
                Value::$variant(value) => Ok(value),
                _ => Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch)),
            }
        }
    };
}

macro_rules! generate_array_load {
    ($name:ident,$($variant:ident),+) => {
        fn $name(&mut self) -> InvokeResult<'a,()> {
            let index = self.pop_int()? as usize;
            let array = self.pop_array()?;
            let value = array.get_field_by_offset(index)?;
            match value {
                $(Value::$variant(_) => {
                   self.push(value)
                })+
                _=>  Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch))
            }
        }
    };
}

macro_rules! generate_array_store {
    ($name:ident, $($variant:ident),+) => {
        fn $name(&mut self) -> InvokeResult<'a,()> {
            let value = self.pop()?;
            let index = self.pop_int()? as usize;
            let array = self.pop_array()?;
             match value {
                $(Value::$variant(_) => {
                  array.set_field_by_offset(index, &value).map_err(MethodCallError::from)
                })+
                _=>  Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch))
            }

        }
    };
}
macro_rules! generate_return {
    ($name:ident, $variant:ident) => {
        fn $name(&mut self) -> InvokeResult<'a, InstructionResult<'a>> {
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
        fn $name(&mut self, index: u8) -> InvokeResult<'a,()> {
            let local = self.local_variables.get(index as usize).ok_or(MethodCallError::InternalError(VmError::ValueTypeMissMatch))?;
            match local {
                $($variant(..) => {
                    self.push(local.clone())
                }),+
                _ => Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch)),
            }
        }
    };
}

macro_rules! generate_store {
    ($name:ident, $variant:ident) => {
        fn $name(&mut self, index: u8) -> InvokeResult<'a, ()> {
            let value = self.pop()?;
            match value {
                $variant(..) => {
                    self.local_variables[index as usize] = value;
                    Ok(())
                }
                _ => Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch)),
            }
        }
    };
}

macro_rules! generate_convert {
    ($name:ident, $variant:ident, $target:ident, $type:ty) => {
        fn $name(&mut self) -> InvokeResult<'a, ()> {
            let value = self.pop()?;
            if let $variant(v) = value {
                self.push($target(v as $type))
            } else {
                Err(MethodCallError::InternalError(VmError::ExecuteCodeError(
                    "convert Error".to_string(),
                )))
            }
        }
    };
}

macro_rules! generate_if_cmp {
    ($name:ident,$variant:ident,$type:ty) => {
        fn $name<T>(&mut self, branch: u16, evaluator: T) -> InvokeResult<'a, ()>
        where
            T: FnOnce($type, $type) -> bool,
        {
            let val2 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch));
            };
            let val1 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch));
            };
            let result = evaluator(val1, val2);
            if result {
                self.goto(branch as usize)
            }
            Ok(())
        }
    };
}

macro_rules! generate_math {
    ($name:ident, $variant:ident, $type:ty) => {
        fn $name<T>(&mut self, evaluator: T) -> InvokeResult<'a, ()>
        where
            T: FnOnce($type, $type) -> InvokeResult<'a, $type>,
        {
            let val2 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch));
            };
            let val1 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch));
            };
            let result = evaluator(val1, val2)?;
            self.push($variant(result))
        }
    };
}

macro_rules! generate_cmp {
    ($name:ident, $variant:ident,$type:ty) => {
        fn $name(&mut self, greater_result: i32) -> InvokeResult<'a, ()> {
            let val2 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch));
            };
            let val1 = if let $variant(v) = self.pop()? {
                v
            } else {
                return Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch));
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
    pub fn new(
        class_ref: ClassRef<'a>,
        method_ref: MethodRef<'a>,
        mut local_variables: Vec<Value<'a>>,
    ) -> CallFrame<'a> {
        let code_attr = method_ref.code.as_ref().expect("Should Has Code");
        let n = code_attr.max_locals as usize;
        (0..n).for_each(|_| local_variables.push(Uninitialized));
        CallFrame {
            class_ref,
            method_ref,
            byte_buffer: ByteBuffer::new(&code_attr.code),
            // pc: ProgramCounter(0),
            local_variables,
            stack: ValueStack::new(code_attr.max_stack as usize),
        }
    }

    generate_pop!(pop_int, Int, i32);
    generate_pop!(pop_long, Long, i64);
    generate_pop!(pop_float, Float, f32);
    generate_pop!(pop_double, Double, f64);
    generate_get_local!(get_local_int, Int, i32);

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

    fn exec_aload(&mut self, index: u8) -> InvokeResult<'a, ()> {
        let local = self
            .local_variables
            .get(index as usize)
            .ok_or(MethodCallError::InternalError(ValueTypeMissMatch))?;
        match local {
            ObjectRef(_) | Null => self.push(local.clone()),
            _ => Err(MethodCallError::InternalError(ValueTypeMissMatch)),
        }
    }

    generate_load!(exec_dload, Double);
    generate_load!(exec_fload, Float);
    generate_load!(exec_iload, Int);
    generate_load!(exec_lload, Long);

    fn exec_astore(&mut self, index: u8) -> InvokeResult<'a, ()> {
        let object_ref = self.pop_object_or_null()?;
        self.local_variables
            .insert(index as usize, object_ref.clone());
        Ok(())
    }

    generate_store!(exec_dstore, Double);
    generate_store!(exec_fstore, Float);
    generate_store!(exec_istore, Int);
    generate_store!(exec_lstore, Long);

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

    generate_convert!(exec_l2d, Long, Double, f64);
    generate_convert!(exec_l2f, Long, Float, f32);
    generate_convert!(exec_l2i, Long, Int, i32);

    generate_math!(exec_double_math, Double, f64);
    generate_math!(exec_float_math, Float, f32);
    generate_math!(exec_int_math, Int, i32);
    generate_math!(exec_long_math, Long, i64);

    generate_cmp!(exec_dcmp, Double, f64);
    generate_cmp!(exec_fcmp, Float, f32);
    generate_cmp!(exec_lcmp, Long, i64);

    generate_return!(exec_dreturn, Double);
    generate_return!(exec_freturn, Float);
    generate_return!(exec_lreturn, Long);
    generate_return!(exec_ireturn, Int);

    generate_if_cmp!(exec_if_acmp, ObjectRef, ObjectReference);
    generate_if_cmp!(exec_if_icmp, Int, i32);

    fn exec_long_shift<T>(&mut self, evaluator: T) -> Result<(), MethodCallError<'a>>
    where
        T: FnOnce(i64, i32) -> Result<i64, VmError>,
    {
        let val2 = self.pop_int()?;
        let val1 = self.pop_long()?;
        let result = evaluator(val1, val2)?;
        self.push(Long(result))
    }

    fn get_local_value(&self, index: u8) -> InvokeResult<'a, Value<'a>> {
        let index = index as usize;
        if index >= self.local_variables.len() {
            return Err(MethodCallError::InternalError(VmError::IndexOutOfBounds));
        }
        Ok(self.local_variables.get(index).unwrap().clone())
    }

    fn set_local_value(&mut self, index: u8, value: Value<'a>) -> InvokeResult<'a, ()> {
        let index = index as usize;
        if index >= self.local_variables.len() {
            return Err(MethodCallError::InternalError(VmError::IndexOutOfBounds));
        }
        self.local_variables.insert(index, value);
        Ok(())
    }

    fn pop_array(&mut self) -> InvokeResult<'a, ArrayReference<'a>> {
        if let ArrayRef(v) = self.pop()? {
            Ok(v)
        } else {
            Err(MethodCallError::InternalError(VmError::ExecuteCodeError(
                "ShouldBeArray".to_string(),
            )))
        }
    }

    fn pop_array_or_null(&mut self) -> InvokeResult<'a, Value<'a>> {
        let value = self.pop()?;
        if let ArrayRef(_) | Null = value {
            Ok(value)
        } else {
            Err(MethodCallError::InternalError(VmError::ExecuteCodeError(
                "ShouldBeArrayOrNull".to_string(),
            )))
        }
    }
    fn pop_object_or_null(&mut self) -> InvokeResult<'a, Value<'a>> {
        let value = self.pop()?;
        if let ObjectRef(_) | Null = value {
            Ok(value)
        } else {
            Err(MethodCallError::InternalError(VmError::ExecuteCodeError(
                "ShouldBeObjectOrNull".to_string(),
            )))
        }
    }

    fn pop_reference_or_null(&mut self) -> InvokeResult<'a, Value<'a>> {
        let value = self.pop()?;
        if let ObjectRef(_) | ArrayRef(_) | Null = value {
            Ok(value)
        } else {
            Err(MethodCallError::InternalError(VmError::ExecuteCodeError(
                "ShouldBeObjectOrNull".to_string(),
            )))
        }
    }
    fn pop_n(&mut self, n: usize) -> InvokeResult<'a, Vec<Value<'a>>> {
        self.stack
            .pop_n(n)
            .map_err(|e| MethodCallError::InternalError(e))
    }
    fn pop_object(&mut self) -> InvokeResult<'a, ObjectReference<'a>> {
        if let ObjectRef(v) = self.pop()? {
            Ok(v)
        } else {
            Err(MethodCallError::InternalError(VmError::ExecuteCodeError(
                "ShouldBeObject".to_string(),
            )))
        }
    }

    fn pop(&mut self) -> InvokeResult<'a, Value<'a>> {
        self.stack.pop().map_err(MethodCallError::from)
    }

    fn exec_pop2(&mut self) -> InvokeResult<'a, ()> {
        let value_1 = self.stack.pop()?;
        match value_1 {
            Long(_) | Double(_) => Ok(()),
            Boolean(_) | Byte(_) | Int(_) | Short(_) | Float(_) | ReturnAddress(_) => {
                if let Boolean(_) | Byte(_) | Int(_) | Short(_) | Float(_) | ReturnAddress(_) =
                    self.pop()?
                {
                    Ok(())
                } else {
                    Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch))
                }
            }
            _ => Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch)),
        }
    }

    fn push(&mut self, value: Value<'a>) -> InvokeResult<'a, ()> {
        self.stack.push(value).map_err(MethodCallError::from)
    }

    fn get_class_name_in_constant_pool(&self, index: u16) -> InvokeResult<'a, &str> {
        self.class_ref
            .constant_pool
            .get_class_name(index)
            .map_err(MethodCallError::from)
    }

    fn get_field_in_constant_pool(&self, index: u16) -> InvokeResult<'a, (&str, &str, &str)> {
        self.class_ref
            .constant_pool
            .get_field_name(index)
            .map_err(MethodCallError::from)
    }

    fn exec_areturn(&mut self) -> InvokeResult<'a, InstructionResult<'a>> {
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
        call_stack: &mut CallStack<'a>,
        constant_index: u16,
    ) -> InvokeResult<'a, ()> {
        let length = self.pop_int()? as usize;
        let class_name = self.get_class_name_in_constant_pool(constant_index)?;
        let class = vm.lookup_class(call_stack, class_name)?;
        let array = vm.new_array(ArrayElement::ClassReference(class), length);
        self.push(ArrayRef(array))
    }

    fn exec_arraylength(&mut self) -> InvokeResult<'a, ()> {
        let array = self.pop_array()?;
        let length = array.get_data_length();
        self.push(Value::Int(length as i32))
    }

    fn exec_athrow(&mut self) -> InvokeResult<'a, InstructionResult<'a>> {
        let value = self.pop_object()?;
        assert!(value.get_class().is_subclass_of("java/lang/Throwable"));
        Err(MethodCallError::ExceptionThrown(value))
    }

    //需要支持数组
    fn check_instance_of(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
        constant_pool_index: u16,
        value: &Value<'a>,
    ) -> InvokeResult<'a, bool> {
        let class_name = self.get_class_name_in_constant_pool(constant_pool_index)?;
        //TODO 数组类，目前先解析了一级数组。后续需要使用递归处理内部类型
        let (is_array, target_class_ref, array_class) =
            if class_name.starts_with("[L") && class_name.ends_with(';') {
                (
                    true,
                    None,
                    Some(ArrayElement::ClassReference(vm.lookup_class(
                        call_stack,
                        &class_name[2..class_name.len() - 1],
                    )?)),
                )
            } else {
                (false, Some(vm.lookup_class(call_stack, class_name)?), None)
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
    ) -> InvokeResult<'a, InstructionResult<'a>> {
        println!("exec {:?}", instruction);
        match instruction {
            Instruction::Aaload => self.exec_aaload()?,
            Instruction::Aastore => self.exec_aastore()?,
            Instruction::Aconst_null => self.stack.push(Null)?,
            Instruction::Aload(local_index) => self.exec_aload(local_index)?,
            Instruction::Aload_0 => self.exec_aload(0)?,
            Instruction::Aload_1 => self.exec_aload(1)?,
            Instruction::Aload_2 => self.exec_aload(2)?,
            Instruction::Aload_3 => self.exec_aload(3)?,
            Instruction::Anewarray(constant_pool_offset) => {
                self.exec_anewarray(vm, call_stack, constant_pool_offset)?
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
                let is_instance_of =
                    self.check_instance_of(vm, call_stack, constant_pool_index, &value)?;
                if is_instance_of {
                    self.push(value)?
                } else {
                    return Err(MethodCallError::from(ValueTypeMissMatch));
                }
            }
            Instruction::D2f => self.exec_d2f()?,
            Instruction::D2i => self.exec_d2i()?,
            Instruction::D2l => self.exec_d2l()?,
            Instruction::Dadd => self.exec_double_math(|v1, v2| Ok(v1 + v2))?,
            Instruction::Daload => self.exec_daload()?,
            Instruction::Dastore => self.exec_dastore()?,
            Instruction::Dcmpg => self.exec_dcmp(-1)?,
            Instruction::Dcmpl => self.exec_dcmp(1)?,
            Instruction::Dconst_0 => self.push(Double(0f64))?,
            Instruction::Dconst_1 => self.push(Double(1f64))?,
            Instruction::Ddiv => self.exec_double_math(|v1, v2| {
                Ok({
                    if is_double_division_returning_nan(v1, v2) {
                        f64::NAN
                    } else {
                        v1 / v2
                    }
                })
            })?,
            Instruction::Dload(local_index) => self.exec_dload(local_index)?,
            Instruction::Dload_0 => self.exec_dload(0)?,
            Instruction::Dload_1 => self.exec_dload(1)?,
            Instruction::Dload_2 => self.exec_dload(2)?,
            Instruction::Dload_3 => self.exec_dload(3)?,
            Instruction::Dmul => self.exec_double_math(|v1, v2| Ok(v1 * v2))?,
            Instruction::Dneg => {
                let value = self.pop_double()?;
                self.push(Double(-value))?;
            }
            Instruction::Drem => self.exec_double_math(|v1, v2| {
                Ok(if is_double_division_returning_nan(v1, v2) {
                    f64::NAN
                } else {
                    v1 % v2
                })
            })?,
            Instruction::Dreturn => return self.exec_dreturn(),
            Instruction::Dstore(local_index) => self.exec_dstore(local_index)?,
            Instruction::Dstore_0 => self.exec_dstore(0)?,
            Instruction::Dstore_1 => self.exec_dstore(1)?,
            Instruction::Dstore_2 => self.exec_dstore(2)?,
            Instruction::Dstore_3 => self.exec_dstore(3)?,
            Instruction::Dsub => self.exec_double_math(|v1, v2| Ok(v1 - v2))?,
            Instruction::Dup => self.stack.dup()?,
            Instruction::Dup_x1 => self.stack.dup_x1()?,
            Instruction::Dup_x2 => self.stack.dup_x2()?,
            Instruction::Dup2 => self.stack.dup2()?,
            Instruction::Dup2_x1 => self.stack.dup2_x1()?,
            Instruction::Dup2_x2 => self.stack.dup2_x2()?,
            Instruction::F2d => self.exec_f2d()?,
            Instruction::F2i => self.exec_f2i()?,
            Instruction::F2l => self.exec_f2l()?,
            Instruction::Fadd => self.exec_float_math(|v1, v2| Ok(v1 + v2))?,
            Instruction::Faload => self.exec_faload()?,
            Instruction::Fastore => self.exec_fastore()?,
            Instruction::Fcmpl => self.exec_fcmp(1)?,
            Instruction::Fcmpg => self.exec_fcmp(-1)?,
            Instruction::Fconst_0 => self.push(Float(0f32))?,
            Instruction::Fconst_1 => self.push(Float(1f32))?,
            Instruction::Fconst_2 => self.push(Float(2f32))?,
            Instruction::Fdiv => self.exec_float_math(|v1, v2| {
                Ok({
                    if is_double_division_returning_nan(v1 as f64, v2 as f64) {
                        f32::NAN
                    } else {
                        v1 / v2
                    }
                })
            })?,
            Instruction::Fload(local_index) => self.exec_fload(local_index)?,
            Instruction::Fload_0 => self.exec_fload(0)?,
            Instruction::Fload_1 => self.exec_fload(1)?,
            Instruction::Fload_2 => self.exec_fload(2)?,
            Instruction::Fload_3 => self.exec_fload(3)?,
            Instruction::Fmul => self.exec_float_math(|v1, v2| Ok(v1 * v2))?,
            Instruction::Fneg => {
                let v = self.pop_float()?;
                self.push(Float(-v))?;
            }
            Instruction::Frem => self.exec_float_math(|v1, v2| {
                Ok(if is_double_division_returning_nan(v1 as f64, v2 as f64) {
                    f32::NAN
                } else {
                    v1 % v2
                })
            })?,
            Instruction::Freturn => return self.exec_freturn(),
            Instruction::Fstore(local_index) => self.exec_fstore(local_index)?,
            Instruction::Fstore_0 => self.exec_fstore(0)?,
            Instruction::Fstore_1 => self.exec_fstore(1)?,
            Instruction::Fstore_2 => self.exec_fstore(2)?,
            Instruction::Fstore_3 => self.exec_fstore(3)?,
            Instruction::Fsub => self.exec_float_math(|v1, v2| Ok(v1 - v2))?,
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
            Instruction::Iadd => self.exec_int_math(|i1, i2| Ok(i1 + i2))?,
            Instruction::Iaload => self.exec_iaload()?,
            Instruction::Iand => self.exec_int_math(|i1, i2| Ok(i1 & i2))?,
            Instruction::Iastore => self.exec_iastore()?,
            Instruction::Iconst_m1 => self.push(Int(-1))?,
            Instruction::Iconst_0 => self.push(Int(0))?,
            Instruction::Iconst_1 => self.push(Int(1))?,
            Instruction::Iconst_2 => self.push(Int(2))?,
            Instruction::Iconst_3 => self.push(Int(3))?,
            Instruction::Iconst_4 => self.push(Int(4))?,
            Instruction::Iconst_5 => self.push(Int(5))?,
            //TODO 除以0异常，
            Instruction::Idiv => self.exec_int_math(|i1, i2| match i2 {
                0 => Err(MethodCallError::InternalError(VmError::ArithmeticException)),
                _ => Ok(i1 / i2),
            })?,
            Instruction::If_acmpeq(branch) => self.exec_if_acmp(branch, |a1, a2| a1 == a2)?,
            Instruction::If_acmpne(branch) => self.exec_if_acmp(branch, |a1, a2| a1 != a2)?,
            Instruction::If_icmpeq(branch) => self.exec_if_icmp(branch, |i1, i2| i1 == i2)?,
            Instruction::If_icmpne(branch) => self.exec_if_icmp(branch, |i1, i2| i1 != i2)?,
            Instruction::If_icmplt(branch) => self.exec_if_icmp(branch, |i1, i2| i1 < i2)?,
            Instruction::If_icmpge(branch) => self.exec_if_icmp(branch, |i1, i2| i1 >= i2)?,
            Instruction::If_icmpgt(branch) => self.exec_if_icmp(branch, |i1, i2| i1 > i2)?,
            Instruction::If_icmple(branch) => self.exec_if_icmp(branch, |i1, i2| i1 <= i2)?,
            Instruction::Ifeq(branch) => self.exec_if(branch, |i1| i1 == 0)?,
            Instruction::Ifne(branch) => self.exec_if(branch, |i1| i1 != 0)?,
            Instruction::Iflt(branch) => self.exec_if(branch, |i1| i1 < 0)?,
            Instruction::Ifge(branch) => self.exec_if(branch, |i1| i1 >= 0)?,
            Instruction::Ifgt(branch) => self.exec_if(branch, |i1| i1 > 0)?,
            Instruction::Ifle(branch) => self.exec_if(branch, |i1| i1 <= 0)?,
            Instruction::Ifnonnull(branch) => {
                let v = self.pop_reference_or_null()?;
                if let Null = v {
                } else {
                    self.goto(branch as usize);
                }
            }
            Instruction::Ifnull(branch) => {
                let v = self.pop_reference_or_null()?;
                if let Null = v {
                    self.goto(branch as usize);
                }
            }
            Instruction::Iinc(index, to_add) => {
                let local = self.get_local_int(index)?;
                self.set_local_value(index, Int(local + to_add as i32))?;
            }
            Instruction::Iload(n) => self.exec_iload(n)?,
            Instruction::Iload_0 => self.exec_iload(0)?,
            Instruction::Iload_1 => self.exec_iload(1)?,
            Instruction::Iload_2 => self.exec_iload(2)?,
            Instruction::Iload_3 => self.exec_iload(3)?,
            Instruction::Imul => self.exec_int_math(|i1, i2| Ok(i1 * i2))?,
            Instruction::Ineg => {
                let value = self.pop_int()?;
                self.push(Int(-value))?;
            }
            Instruction::Instanceof(cp_index) => {
                let value = self.pop()?;
                let result = self.check_instance_of(vm, call_stack, cp_index, &value)?;
                if result {
                    self.push(Int(1))?;
                } else {
                    self.push(Int(0))?;
                }
            }
            Instruction::Invokedynamic(_) => {
                todo!()
            }
            Instruction::Invokeinterface(_, _) => {
                todo!()
            }
            Instruction::Invokespecial(offset) => {
                self.exec_invoke_special(vm, call_stack, offset)?
            }
            Instruction::Invokestatic(offset) => self.exec_invoke_static(vm, call_stack, offset)?,
            Instruction::Invokevirtual(offset) => {
                todo!()
            }
            Instruction::Ior => self.exec_int_math(|i1, i2| Ok(i1.bitor(i2)))?,
            Instruction::Irem => self.exec_int_math(|i1, i2| match i2 {
                0 => Err(MethodCallError::InternalError(VmError::ArithmeticException)),
                _ => Ok(i1.rem(i2)),
            })?,
            Instruction::Ireturn => {
                return self.exec_ireturn();
            }
            Instruction::Ishl => self.exec_int_math(|i1, i2| Ok(i1 << (i2 & 0x1f)))?,
            Instruction::Ishr => self.exec_int_math(|i1, i2| Ok(i1 >> (i2 & 0x1f)))?,
            Instruction::Istore(local_index) => self.exec_istore(local_index)?,
            Instruction::Istore_0 => self.exec_istore(0)?,
            Instruction::Istore_1 => self.exec_istore(1)?,
            Instruction::Istore_2 => self.exec_istore(2)?,
            Instruction::Istore_3 => self.exec_istore(3)?,
            Instruction::Isub => self.exec_int_math(|i1, i2| Ok(i1 - i2))?,
            Instruction::Iushr => self.exec_int_math(|i1, i2| {
                Ok({
                    if i1 > 0 {
                        i1 >> (i2 & 0x1f)
                    } else {
                        ((i1 as u32) >> (i2 & 0x1f)) as i32
                    }
                })
            })?,
            Instruction::Ixor => self.exec_int_math(|i1, i2| Ok(i1.bitxor(i2)))?,
            Instruction::Jsr(address) => self.push(ReturnAddress(address as u32))?,
            Instruction::Jsr_w(address) => self.push(ReturnAddress(address))?,
            Instruction::L2d => self.exec_l2d()?,
            Instruction::L2f => self.exec_l2f()?,
            Instruction::L2i => self.exec_l2i()?,
            Instruction::Ladd => self.exec_long_math(|l1, l2| Ok(l1 + l2))?,
            Instruction::Laload => self.exec_laload()?,
            Instruction::Land => self.exec_long_math(|l1, l2| Ok(l1.bitand(l2)))?,
            Instruction::Lastore => self.exec_lastore()?,
            Instruction::Lcmp => self.exec_lcmp(1)?,
            Instruction::Lconst_0 => self.push(Long(0))?,
            Instruction::Lconst_1 => self.push(Long(1))?,
            Instruction::Ldc(constant_pool_index) => {
                self.exec_ldc(vm, call_stack, constant_pool_index as u16)?
            }
            Instruction::Ldc_w(constant_pool_index) => {
                self.exec_ldc(vm, call_stack, constant_pool_index)?
            }
            Instruction::Ldc2_w(constant_pool_index) => self.exec_ldc2(constant_pool_index)?,
            Instruction::Ldiv => self.exec_long_math(|l1, l2| match l2 {
                0 => Err(MethodCallError::InternalError(VmError::ArithmeticException)),
                _ => Ok(l1.div(l2)),
            })?,
            Instruction::Lload(n) => self.exec_lload(n)?,
            Instruction::Lload_0 => self.exec_lload(0)?,
            Instruction::Lload_1 => self.exec_lload(1)?,
            Instruction::Lload_2 => self.exec_lload(2)?,
            Instruction::Lload_3 => self.exec_lload(3)?,
            Instruction::Lmut => self.exec_long_math(|l1, l2| Ok(l1.mul(l2)))?,
            Instruction::Lneg => {
                let value = self.pop_long()?;
                self.push(Long(-value))?
            }
            Instruction::Lookupswitch => {}
            Instruction::Lor => self.exec_long_math(|l1, l2| Ok(l1.bitxor(l2)))?,
            Instruction::Lrem => self.exec_long_math(|l1, l2| match l2 {
                0 => Err(MethodCallError::InternalError(VmError::ArithmeticException)),
                _ => Ok(l1.rem(l2)),
            })?,
            Instruction::Lreturn => return self.exec_lreturn(),
            Instruction::Lshl => self.exec_long_shift(|l1, l2| Ok(l1.shl(l2)))?,
            Instruction::Lshr => self.exec_long_shift(|l1, l2| Ok(l1.shr(l2)))?,
            Instruction::Lstore(n) => self.exec_lstore(n)?,
            Instruction::Lstore_0 => self.exec_lstore(0)?,
            Instruction::Lstore_1 => self.exec_lstore(1)?,
            Instruction::Lstore_2 => self.exec_lstore(2)?,
            Instruction::Lstore_3 => self.exec_lstore(3)?,
            Instruction::Lsub => self.exec_long_math(|l1, l2| Ok(l1.sub(l2)))?,
            Instruction::Lushr => self.exec_long_shift(|l1, l2| {
                Ok({
                    if l1 > 0 {
                        l1 >> (l2 & 0x1f)
                    } else {
                        ((l1 as u64) >> (l2 & 0x1f)) as i64
                    }
                })
            })?,
            Instruction::Lxor => self.exec_long_math(|l1, l2| Ok(l1.bitxor(l2)))?,
            Instruction::Monitorenter => {}
            Instruction::Monitorexit => {}
            Instruction::Multianewarray(_, _) => {}
            Instruction::New(constant_pool_index) => {
                self.exec_new_object(vm, call_stack, constant_pool_index)?
            }
            Instruction::NewArray(a_type) => self.exec_new_array(vm, a_type)?,
            Instruction::Nop => {}
            Instruction::Pop => {
                self.pop()?;
            }
            Instruction::Pop2 => self.exec_pop2()?,
            Instruction::Putfield(constant_pool_index) => {
                self.exec_put_field(constant_pool_index)?
            }
            Instruction::Putstatic(constant_pool_index) => {
                self.exec_put_static(vm, constant_pool_index)?
            }
            Instruction::Ret(local_var_index) => {
                if let ReturnAddress(address) = self.get_local_value(local_var_index)? {
                    self.goto(address as usize);
                } else {
                    return Err(MethodCallError::InternalError(ValueTypeMissMatch));
                }
            }
            Instruction::Return => return Ok(ReturnFromMethod(None)),
            Instruction::Saload => self.exec_saload()?,
            Instruction::Sastore => self.exec_sastore()?,
            Instruction::Sipush(value) => self.push(Short(value))?,
            Instruction::Swap => self.stack.swap()?,
            Instruction::Tableswitch => {}
            Instruction::Wide => {}
        }
        Ok(ContinueMethodExecution)
    }

    fn exec_new_object(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
        pool_index: u16,
    ) -> InvokeResult<'a, ()> {
        let class_name = self.get_class_name_in_constant_pool(pool_index)?;
        let class_ref = vm.lookup_class(call_stack, class_name)?;
        let object_reference = vm.new_object(class_ref);
        self.push(ObjectRef(object_reference))
    }

    fn exec_new_array(&mut self, vm: &mut VirtualMachine<'a>, a_type: u8) -> InvokeResult<'a, ()> {
        let count = self.pop_int()?;
        let primary_type = match a_type {
            4 => PrimaryType::Boolean,
            5 => PrimaryType::Char,
            6 => PrimaryType::Float,
            7 => PrimaryType::Double,
            8 => PrimaryType::Byte,
            9 => PrimaryType::Short,
            10 => PrimaryType::Int,
            11 => PrimaryType::Long,
            _ => return Err(MethodCallError::InternalError(ValueTypeMissMatch)),
        };
        let array_ref = vm.new_array(ArrayElement::PrimaryValue(primary_type), count as usize);
        self.push(ArrayRef(array_ref))
    }

    fn get_constant_pool(&self, offset: u16) -> VmExecResult<&'a RuntimeConstantPoolEntry> {
        self.class_ref.constant_pool.get(offset)
    }

    fn exec_ldc(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
        index: u16,
    ) -> InvokeResult<'a, ()> {
        let value = self.get_constant_pool(index)?;
        match value {
            RuntimeConstantPoolEntry::Integer(i) => self.push(Int(*i)),
            RuntimeConstantPoolEntry::Float(f) => self.push(Float(*f)),

            RuntimeConstantPoolEntry::ClassReference(class_name) => self.push(ObjectRef(
                vm.new_java_lang_class_object(call_stack, class_name)
                    .unwrap(),
            )),
            RuntimeConstantPoolEntry::StringReference(str) => self.push(ObjectRef(
                vm.new_java_lang_string_object(call_stack, str).unwrap(),
            )),

            RuntimeConstantPoolEntry::MethodReference(_, _, _) => {
                todo!("新建一个java.lang.invoke.MethodType")
            }
            RuntimeConstantPoolEntry::MethodHandler(_, _, _, _) => {
                todo!("新建一个java.lang.invoke.MethodHandle")
            }
            _ => Err(MethodCallError::InternalError(ValueTypeMissMatch)),
        }
    }

    fn exec_ldc2(&mut self, index: u16) -> InvokeResult<'a, ()> {
        let value = self.get_constant_pool(index)?;
        match value {
            RuntimeConstantPoolEntry::Long(i) => self.push(Long(*i)),
            RuntimeConstantPoolEntry::Double(f) => self.push(Double(*f)),
            _ => Err(MethodCallError::InternalError(VmError::ValueTypeMissMatch)),
        }
    }

    fn exec_if<T>(&mut self, branch: u16, evaluator: T) -> InvokeResult<'a, ()>
    where
        T: FnOnce(i32) -> bool,
    {
        let value = self.pop_int()?;
        if evaluator(value) {
            self.goto(branch as usize);
        }
        Ok(())
    }

    fn goto(&mut self, new_pc: usize) {
        self.byte_buffer.jump_to(new_pc);
    }

    fn exec_get_field(&mut self, field_index: u16) -> InvokeResult<'a, ()> {
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
        Err(MethodCallError::InternalError(ValueTypeMissMatch))
    }

    fn exec_put_field(&mut self, field_index: u16) -> InvokeResult<'a, ()> {
        let value = self.pop()?;
        let object = self.pop()?;
        if let ObjectRef(object_ref) = object {
            let (class_name, field_name, _descriptor) =
                self.get_field_in_constant_pool(field_index)?;
            let class_ref = object_ref.get_class();
            //TODO 校验描述符类型
            assert_eq!(class_ref.name, class_name);
            //TODO 校验值类型
            object_ref.set_field_by_name(field_name, &value)?
        }
        Err(MethodCallError::InternalError(ValueTypeMissMatch))
    }

    fn exec_get_static(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        field_index: u16,
    ) -> InvokeResult<'a, ()> {
        let (class_name, field_name, _descriptor) = self.get_field_in_constant_pool(field_index)?;
        let value = vm.get_static_field_by_class_name(class_name, field_name)?;
        self.push(value.unwrap().clone())
    }

    fn exec_put_static(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        field_index: u16,
    ) -> InvokeResult<'a, ()> {
        let static_value = self.pop()?;
        let (class_name, field_name, _descriptor) = self.get_field_in_constant_pool(field_index)?;
        vm.set_static_field_by_class_name(class_name, field_name, static_value)
    }

    fn exec_invoke_special(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
        offset: u16,
    ) -> InvokeResult<'a, ()> {
        if let RuntimeConstantPoolEntry::MethodReference(class_name, method_name, descriptor) =
            self.get_constant_pool(offset)?
        {
            let class_ref = vm.lookup_class(call_stack, class_name)?;
            let method_ref = class_ref.get_method(method_name, descriptor)?;
            let method_args = &method_ref.descriptor_args_ret.args;
            //TODO validate method_args and poped args type
            let args = self.pop_n(method_args.len())?;
            let object_ref = self.pop_object()?;
            //必须是子类调用父类的方法，自身的私有方法，以及实例初始化化方法
            assert!(object_ref.is_instance_of(class_ref));

            vm.invoke_method(call_stack, class_ref, method_ref, Some(object_ref), args)?;
            Ok(())
        } else {
            Err(MethodCallError::InternalError(ValueTypeMissMatch))
        }
    }
    fn exec_invoke_virtual(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
        offset: u16,
    ) -> InvokeResult<'a, ()> {
        if let RuntimeConstantPoolEntry::MethodReference(class_name, method_name, descriptor)
        | RuntimeConstantPoolEntry::InterfaceMethodReference(
            class_name,
            method_name,
            descriptor,
        ) = self.get_constant_pool(offset)?
        {
            let mut class_ref = vm.lookup_class(call_stack, class_name)?;
            let mut method_ref = class_ref.get_method(method_name, descriptor)?;
            let method_args = &method_ref.descriptor_args_ret.args;
            //TODO validate method_args and poped args type
            let args = self.stack.pop_n(method_args.len())?;
            let object_ref = self.pop_object()?;
            //多态方法，方法要从当前对象去查方法实例
            assert!(object_ref.is_instance_of(class_ref));
            class_ref = object_ref.get_class();
            method_ref = class_ref.get_method_by_checking_super(method_name, descriptor)?;
            vm.invoke_method(call_stack, class_ref, method_ref, Some(object_ref), args)?;
            Ok(())
        } else {
            Err(MethodCallError::InternalError(ValueTypeMissMatch))
        }
    }

    fn exec_invoke_static(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
        offset: u16,
    ) -> InvokeResult<'a, ()> {
        if let RuntimeConstantPoolEntry::MethodReference(class_name, method_name, descriptor)
        | RuntimeConstantPoolEntry::InterfaceMethodReference(
            class_name,
            method_name,
            descriptor,
        ) = self.get_constant_pool(offset)?
        {
            let class_ref = if &self.class_ref.name != class_name {
                vm.lookup_class(call_stack, class_name)?
            } else {
                self.class_ref
            };
            let method_ref = class_ref.get_method(method_name, descriptor)?;
            assert!(method_ref.is_static());
            let method_args = &method_ref.descriptor_args_ret.args;
            //TODO validate method_args and poped args type
            let args = self.stack.pop_n(method_args.len())?;
            vm.invoke_method(call_stack, class_ref, method_ref, None, args)?;
            Ok(())
        } else {
            Err(MethodCallError::InternalError(ValueTypeMissMatch))
        }
    }

    pub fn execute(
        &mut self,
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
    ) -> InvokeMethodResult<'a> {
        println!(
            "invoke method=> {}:{}->{}",
            self.class_ref.name, self.method_ref.name, self.method_ref.descriptor
        );
        loop {
            let instruction = read_one_instruction(&mut self.byte_buffer).unwrap();
            let result = self.execute_instruction(vm, call_stack, instruction);
            match result {
                Ok(ReturnFromMethod(return_value)) => {
                    return Ok(return_value);
                }
                Err(e) => {
                    return Err(e);
                }
                _ => {}
            }
        }
    }
}
