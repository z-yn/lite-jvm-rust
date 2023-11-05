use crate::jvm_error::VmError;
use crate::jvm_values::{ObjectReference, Value};

#[derive(Debug)]
pub enum MethodCallError<'a> {
    InternalError(VmError),
    ExceptionThrown(ObjectReference<'a>),
}

impl<'a> From<VmError> for MethodCallError<'a> {
    fn from(value: VmError) -> Self {
        Self::InternalError(value)
    }
}

pub type InvokeMethodResult<'a> = Result<Option<Value<'a>>, MethodCallError<'a>>;
