use crate::jvm_values::{ObjectReference, Value};
use crate::loaded_class::ClassRef;
use crate::object_heap::ObjectHeap;
use indexmap::IndexMap;
use std::collections::HashMap;

/// 静态区。用来存储静态属性和字符串

pub(crate) struct StaticArea<'a> {
    fields: HashMap<ClassRef<'a>, IndexMap<String, Value<'a>>>,
    static_object_heap: ObjectHeap<'a>,
    pub(crate) string_constant_pool: HashMap<String, ObjectReference<'a>>,
    pub(crate) class_constant_pool: HashMap<String, ObjectReference<'a>>,
}
impl<'a> StaticArea<'a> {
    pub(crate) fn new(static_heap_size: usize) -> StaticArea<'a> {
        StaticArea {
            fields: HashMap::new(),
            static_object_heap: ObjectHeap::new(static_heap_size),
            string_constant_pool: Default::default(),
            class_constant_pool: Default::default(),
        }
    }

    pub fn new_object(&mut self, class_ref: ClassRef) -> ObjectReference<'a> {
        self.static_object_heap.allocate_object(class_ref).unwrap()
    }

    pub(crate) fn get_static_field(
        &self,
        class_ref: ClassRef<'a>,
        field_name: &str,
    ) -> Option<&Value<'a>> {
        let map = self.fields.get(class_ref)?;
        map.get(field_name)
    }

    pub(crate) fn set_static_field(
        &mut self,
        class_ref: ClassRef<'a>,
        field_name: &str,
        value: Value<'a>,
    ) {
        self.fields
            .entry(class_ref)
            .or_default()
            .insert(field_name.to_string(), value);
    }
}
