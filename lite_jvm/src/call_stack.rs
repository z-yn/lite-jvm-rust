use crate::call_frame::CallFrame;
use typed_arena::Arena;

pub struct CallStack<'a> {
    call_frames: Arena<CallFrame<'a>>,
}
