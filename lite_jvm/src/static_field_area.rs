use crate::jvm_values::{ObjectReference, Value};
use crate::loaded_class::ClassRef;
use indexmap::IndexMap;
use std::collections::HashMap;

/// 静态区。用来存储静态属性和字符串

pub(crate) struct StaticArea<'a> {
    fields: HashMap<ClassRef<'a>, IndexMap<String, Value<'a>>>,
    string_constant_pool: HashMap<&'a str, ObjectReference<'a>>,
}
impl<'a> StaticArea<'a> {
    pub(crate) fn new() -> StaticArea<'a> {
        StaticArea {
            fields: HashMap::new(),
            string_constant_pool: Default::default(),
        }
    }

    pub(crate) fn get_string(&self, str: &str) -> Option<&ObjectReference<'a>> {
        self.string_constant_pool.get(str)
    }

    pub(crate) fn cache_string(&self, str: &str) -> Option<&ObjectReference<'a>> {
        self.string_constant_pool.get(str)
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
