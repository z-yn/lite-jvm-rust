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
    fn test_allocate_object() {
        use crate::class_finder::{FileSystemClassPath, JarFileClassPath};
        use crate::method_area::MethodArea;
        use crate::object_heap::ObjectHeap;
        use crate::referenced_value::Value;
        let area = MethodArea::new();

        let file_system_path = FileSystemClassPath::new("./resources").unwrap();
        area.add_class_path(Box::new(file_system_path));
        let rt_jar_path = JarFileClassPath::new("./resources/rt.jar").unwrap();

        area.add_class_path(Box::new(rt_jar_path));
        let result = area.load_class("FieldTest").unwrap();

        let mut heap = ObjectHeap::new(1024);
        let allocated_obj = heap.allocate_object(result).unwrap();
        let class_ref = allocated_obj.get_class();
        assert_eq!(class_ref.name, "FieldTest");

        allocated_obj.set_field("a", &Value::Int(2)).unwrap();
        let value = allocated_obj.get_field("a").unwrap();
        assert!(matches!(value, Value::Int(2)));
    }
}
