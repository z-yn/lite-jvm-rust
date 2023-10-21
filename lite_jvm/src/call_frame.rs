use crate::loaded_class::{ClassRef, MethodRef};
use crate::program_counter::ProgramCounter;
use crate::referenced_value::Value;

#[derive(Debug)]
pub struct ValueStack<'a> {
    stack: Vec<Value<'a>>,
}
pub struct CallFrame<'a> {
    class_ref: ClassRef<'a>,
    method_ref: MethodRef<'a>,
    pc: ProgramCounter,
    local_variables: Vec<Value<'a>>,
    stack: ValueStack<'a>,
    code: &'a Vec<u8>,
}
