use crate::loaded_class::ClassRef;
use crate::memory_trunk::MemoryChunk;
use crate::referenced_value::ReferencedValue;
use std::marker::PhantomData;

pub struct ObjectHeap<'a> {
    memory: MemoryChunk,
    _marker: PhantomData<&'a ReferencedValue<'a>>,
}
impl<'a> ObjectHeap<'a> {
    pub(crate) fn new(size: usize) -> ObjectHeap<'a> {
        ObjectHeap {
            memory: MemoryChunk::new(size),
            _marker: Default::default(),
        }
    }

    pub fn allocate_object(&mut self, class: ClassRef<'a>) -> Option<ReferencedValue<'a>> {
        let size = ReferencedValue::size_of_object(class);
        self.memory
            .alloc(size)
            .map(|(ptr, size)| ReferencedValue::new_object(class, ptr, size))
    }
}

mod tests {
    #[test]
    fn test_allocate_object() {}
}
