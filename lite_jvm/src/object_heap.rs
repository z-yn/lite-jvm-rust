use crate::loaded_class::ClassRef;
use crate::memory_trunk::MemoryChunk;
use crate::reference_value::{
    size_of_array, size_of_object, ArrayElement, ArrayReference, ObjectReference,
};
use std::marker::PhantomData;

pub struct ObjectHeap<'a> {
    memory: MemoryChunk,
    _marker: PhantomData<&'a ObjectReference<'a>>,
}
impl<'a> ObjectHeap<'a> {
    pub(crate) fn new(size: usize) -> ObjectHeap<'a> {
        ObjectHeap {
            memory: MemoryChunk::new(size),
            _marker: Default::default(),
        }
    }

    pub fn allocate_object(&mut self, class: ClassRef<'a>) -> Option<ObjectReference<'a>> {
        let size = size_of_object(class);
        self.memory
            .alloc(size)
            .map(|(ptr, size)| ObjectReference::new_object(class, ptr, size))
    }

    pub fn allocate_array(
        &mut self,
        array_element: ArrayElement,
        length: usize,
    ) -> Option<ObjectReference<'a>> {
        let size = size_of_array(length);
        self.memory
            .alloc(size)
            .map(|(ptr, size)| ArrayReference::new_array(array_element, length, ptr, size))
    }
}

mod tests {

    #[test]
    fn test_allocate_object() {
        use crate::class_finder::{FileSystemClassPath, JarFileClassPath};
        use crate::method_area::MethodArea;
        use crate::object_heap::ObjectHeap;
        use crate::reference_value::ReferenceValue;
        use crate::reference_value::Value;
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

        allocated_obj
            .set_field_by_name("a", &Value::Int(2))
            .unwrap();
        let value = allocated_obj.get_field_by_name("a").unwrap();
        assert!(matches!(value, Value::Int(2)));
    }
}
