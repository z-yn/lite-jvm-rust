use crate::call_frame::CallFrame;
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
    fn new() -> CallStack<'a> {
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
    ) -> CallFrameRef<'a> {
        todo!()
    }

    pub(crate) fn pop_frame(&mut self) {
        if self.frames.len() > 0 {
            self.frames.pop().unwrap();
        }
    }
}
