use crate::loaded_class::ClassRef;
use crate::reference_value::Value;
use indexmap::IndexMap;
use std::collections::HashMap;

/// 静态区。用来存储静态属性和字符串

pub(crate) struct StaticArea<'a> {
    fields: HashMap<ClassRef<'a>, IndexMap<String, Value<'a>>>,
}
impl<'a> StaticArea<'a> {
    pub(crate) fn new() -> StaticArea<'a> {
        StaticArea {
            fields: HashMap::new(),
        }
    }
    pub(crate) fn get_static_field(&self, class_ref: ClassRef<'a>, field_name: &str) -> Value<'a> {
        self.fields
            .get(class_ref)
            .unwrap()
            .get(field_name)
            .unwrap()
            .clone()
    }

    pub(crate) fn set_static_field(
        &mut self,
        class_ref: ClassRef<'a>,
        field_name: &str,
        value: Value<'a>,
    ) {
        self.fields
            .entry(class_ref)
            .or_insert(IndexMap::new())
            .insert(field_name.to_string(), value);
    }
}
