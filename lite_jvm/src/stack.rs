use crate::jvm_error::{VmError, VmExecResult};
use crate::jvm_values::{ObjectReference, Value};
use crate::loaded_class::{ClassRef, MethodRef};
use crate::stack_frame::StackFrame;
use typed_arena::Arena;

//需要包装一个裸指针，用来保持mutable的引用
#[derive(Debug, Clone)]
pub struct StackFrameRef<'a>(*mut StackFrame<'a>);

impl<'a> AsRef<StackFrame<'a>> for StackFrameRef<'a> {
    fn as_ref(&self) -> &StackFrame<'a> {
        unsafe { self.0.as_ref() }.unwrap()
    }
}

impl<'a> AsMut<StackFrame<'a>> for StackFrameRef<'a> {
    fn as_mut(&mut self) -> &mut StackFrame<'a> {
        unsafe { self.0.as_mut() }.unwrap()
    }
}
pub struct CallStack<'a> {
    frames: Vec<StackFrameRef<'a>>,
    arena: Arena<StackFrame<'a>>,
}

impl<'a> CallStack<'a> {
    pub(crate) fn new() -> CallStack<'a> {
        CallStack {
            frames: Vec::new(),
            arena: Arena::new(),
        }
    }
    pub(crate) fn depth(&self) -> usize {
        self.frames.len()
    }
    pub(crate) fn new_frame(
        &mut self,
        class_ref: ClassRef<'a>,
        method_ref: MethodRef<'a>,
        object: Option<ObjectReference<'a>>,
        args: Vec<Value<'a>>,
    ) -> VmExecResult<StackFrameRef<'a>> {
        if method_ref.1.is_native() {
            return Err(VmError::NotImplemented);
        };
        let locals: Vec<Value<'a>> = object
            .map(Value::ObjectRef)
            .into_iter()
            .chain(args)
            .collect();
        let new_frame = self
            .arena
            .alloc(StackFrame::new(class_ref, method_ref, locals));
        let frame = StackFrameRef(new_frame);
        self.frames.push(frame.clone());
        Ok(frame)
    }

    pub(crate) fn pop_frame(&mut self) {
        if !self.frames.is_empty() {
            self.frames.pop().unwrap();
        }
    }
}
