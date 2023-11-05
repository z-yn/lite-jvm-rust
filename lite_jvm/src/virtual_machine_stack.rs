use crate::jvm_error::{VmError, VmExecResult};
use crate::jvm_values::{ObjectReference, Value};
use crate::loaded_class::{ClassRef, MethodRef};
use crate::virtual_machine_stack_frame::VirtualMachineStackFrame;
use typed_arena::Arena;

//需要包装一个裸指针，用来保持mutable的引用
#[derive(Debug, Clone)]
pub struct CallFrameRef<'a>(*mut VirtualMachineStackFrame<'a>);

impl<'a> AsRef<VirtualMachineStackFrame<'a>> for CallFrameRef<'a> {
    fn as_ref(&self) -> &VirtualMachineStackFrame<'a> {
        unsafe { self.0.as_ref() }.unwrap()
    }
}

impl<'a> AsMut<VirtualMachineStackFrame<'a>> for CallFrameRef<'a> {
    fn as_mut(&mut self) -> &mut VirtualMachineStackFrame<'a> {
        unsafe { self.0.as_mut() }.unwrap()
    }
}
pub struct VirtualMachineStack<'a> {
    frames: Vec<CallFrameRef<'a>>,
    arena: Arena<VirtualMachineStackFrame<'a>>,
}

impl<'a> VirtualMachineStack<'a> {
    pub(crate) fn new() -> VirtualMachineStack<'a> {
        VirtualMachineStack {
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
            .alloc(VirtualMachineStackFrame::new(class_ref, method_ref, locals));
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
