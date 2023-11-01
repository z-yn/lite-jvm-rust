use crate::call_frame::CallFrame;
use crate::jvm_error::{VmError, VmExecResult};
use crate::loaded_class::{ClassRef, MethodRef};
use crate::reference_value::{ObjectReference, Value};
use typed_arena::Arena;

//需要包装一个裸指针，用来保持mutable的引用
#[derive(Debug, Clone)]
pub struct CallFrameRef<'a>(*mut CallFrame<'a>);

impl<'a> AsRef<CallFrame<'a>> for CallFrameRef<'a> {
    fn as_ref(&self) -> &CallFrame<'a> {
        unsafe { self.0.as_ref() }.unwrap()
    }
}

impl<'a> AsMut<CallFrame<'a>> for CallFrameRef<'a> {
    fn as_mut(&mut self) -> &mut CallFrame<'a> {
        unsafe { self.0.as_mut() }.unwrap()
    }
}
pub struct CallStack<'a> {
    frames: Vec<CallFrameRef<'a>>,
    arena: Arena<CallFrame<'a>>,
}

impl<'a> CallStack<'a> {
    pub(crate) fn new() -> CallStack<'a> {
        CallStack {
            frames: Vec::new(),
            arena: Arena::new(),
        }
    }

    pub(crate) fn new_frame(
        &mut self,
        class_ref: ClassRef<'a>,
        method_ref: MethodRef<'a>,
        object: Option<ObjectReference<'a>>,
        args: Vec<Value<'a>>,
    ) -> VmExecResult<CallFrameRef<'a>> {
        if method_ref.is_native() {
            return Err(VmError::NotImplemented);
        };
        let locals: Vec<Value<'a>> = object
            .map(Value::ObjectRef)
            .into_iter()
            .chain(args.into_iter())
            .collect();
        let new_frame = self
            .arena
            .alloc(CallFrame::new(class_ref, method_ref, locals));
        let frame = CallFrameRef(new_frame);
        self.frames.push(frame.clone());
        Ok(frame)
    }

    pub(crate) fn pop_frame(&mut self) {
        if self.frames.len() > 0 {
            self.frames.pop().unwrap();
        }
    }
}
