use crate::jvm_exceptions::{Exception, Result};
use crate::loaded_class::ClassRef;
use crate::runtime_field_info::RuntimeFieldInfo;
use bitfield_struct::bitfield;
use std::marker::PhantomData;
use std::mem::size_of;

///https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-2.html#jvms-2.2
///
/// 用来表示放到内存中的数据
/// Possible primitive types
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum PrimaryType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
}
#[derive(Debug, Default, Clone, PartialEq)]
pub enum Value<'a> {
    #[default]
    Uninitialized,
    Boolean(bool),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Char(u16),
    Float(f32),
    Double(f64),
    ReturnAddress(u16),
    ObjectRef(ReferencedValue<'a>),
    ArrayRef(ReferencedValue<'a>),
    Null,
}

/// 引用对象分配
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ReferencedValue<'a> {
    data: *mut u8,
    _marker: PhantomData<&'a [u8]>,
}

impl<'a> ReferencedValue<'a> {
    pub(crate) fn size_of_object(class: ClassRef<'_>) -> usize {
        let fields_sizes: usize = 8 * class.total_num_of_fields;
        ALLOC_HEADER_SIZE + OBJECT_HEADER_SIZE + fields_sizes
    }

    pub(crate) fn size_of_array(length: usize) -> usize {
        ALLOC_HEADER_SIZE + ARRAY_HEADER_SIZE + length * 8
    }

    unsafe fn write_allocate_header(ptr: *const u8, header: AllocateHeader) -> *mut u8 {
        let next_ptr = ptr as *mut AllocateHeader;
        std::ptr::write(next_ptr, header);
        next_ptr.add(1) as *mut u8
    }

    pub(crate) fn new_object(
        class_ref: ClassRef,
        start_ptr: *const u8,
        size: usize,
    ) -> ReferencedValue<'a> {
        unsafe {
            let next_ptr = Self::write_allocate_header(
                start_ptr,
                AllocateHeader::new()
                    .with_kind(ReferenceValueType::Object)
                    .with_size(size),
            );
            std::ptr::write(next_ptr as *mut ObjectHeader, ObjectHeader { class_ref });
        }
        ReferencedValue {
            data: start_ptr as *mut u8,
            _marker: Default::default(),
        }
    }
    pub(crate) fn get_class(&self) -> ClassRef {
        unsafe {
            let class_ref_ptr = self.data.add(ALLOC_HEADER_SIZE);
            std::ptr::read(class_ref_ptr as *const ObjectHeader).class_ref
        }
    }

    //TODO 校验Value与RuntimeFieldInfo是否一致
    unsafe fn write_value_at_offset(&self, field: &RuntimeFieldInfo, value: &Value<'a>) {
        let offset = field.offset;
        assert!(offset > 0);
        let pointer = self.data.add((offset - 1) * 8);
        match value {
            Value::Byte(v) => std::ptr::write(pointer as *mut i8, *v),
            Value::Short(v) => std::ptr::write(pointer as *mut i16, *v),
            Value::Int(v) => std::ptr::write(pointer as *mut i32, *v),
            Value::Long(v) => std::ptr::write(pointer as *mut i64, *v),
            Value::Char(v) => std::ptr::write(pointer as *mut u16, *v),
            Value::Float(v) => std::ptr::write(pointer as *mut f32, *v),
            Value::Double(v) => std::ptr::write(pointer as *mut f64, *v),
            Value::ReturnAddress(v) => std::ptr::write(pointer as *mut u16, *v),
            Value::ObjectRef(v) => std::ptr::write(pointer as *mut ReferencedValue, *v),
            Value::ArrayRef(v) => std::ptr::write(pointer as *mut ReferencedValue, *v),
            Value::Boolean(v) => std::ptr::write(pointer as *mut bool, *v),
            _ => std::ptr::write(pointer as *mut u64, 0),
        }
    }

    //TODO 校验Value与RuntimeFieldInfo是否一致
    unsafe fn read_value_at_offset(&self, field: &RuntimeFieldInfo) -> Value<'a> {
        let offset = field.offset;
        assert!(offset > 0);
        let pointer = self.data.add((offset - 1) * 8);
        match field.descriptor.as_str() {
            "B" => Value::Byte(std::ptr::read(pointer as *mut i8)),
            "C" => Value::Char(std::ptr::read(pointer as *mut u16)),
            "D" => Value::Double(std::ptr::read(pointer as *mut f64)),
            "F" => Value::Float(std::ptr::read(pointer as *mut f32)),
            "I" => Value::Int(std::ptr::read(pointer as *mut i32)),
            "J" => Value::Long(std::ptr::read(pointer as *mut i64)),
            "S" => Value::Short(std::ptr::read(pointer as *mut i16)),
            "Z" => Value::Boolean(std::ptr::read(pointer as *mut bool)),
            other => {
                if other.starts_with("[") {
                    Value::ArrayRef(std::ptr::read(pointer as *mut ReferencedValue))
                } else {
                    Value::ObjectRef(std::ptr::read(pointer as *mut ReferencedValue))
                }
            }
        }
    }

    //
    pub(crate) fn set_field(&self, name: &str, value: &Value<'a>) -> Result<()> {
        //先查找自身类中的field
        let class = self.get_class();
        if let Some(field) = class.fields.get(name) {
            unsafe {
                self.write_value_at_offset(field, value);
            }
            return Ok(());
        }
        if let Some(super_class) = class.super_class {
            if let Some(field) = super_class.fields.get(name) {
                unsafe {
                    self.write_value_at_offset(field, value);
                }
                return Ok(());
            }
        }
        Err(Exception::FieldNotFoundException(name.to_string()))
    }

    pub(crate) fn get_field(&self, name: &str) -> Result<Value<'a>> {
        //先查找自身类中的field
        let class = self.get_class();
        if let Some(field) = class.fields.get(name) {
            return unsafe { Ok(self.read_value_at_offset(field)) };
        }
        if let Some(super_class) = class.super_class {
            if let Some(field) = super_class.fields.get(name) {
                return unsafe { Ok(self.read_value_at_offset(field)) };
            }
        }
        Err(Exception::FieldNotFoundException(name.to_string()))
    }
}

const fn align_to_8_bytes(required_size: usize) -> usize {
    match required_size % 8 {
        0 => required_size,
        n => required_size + (8 - n),
    }
}

pub(crate) const ALLOC_HEADER_SIZE: usize = align_to_8_bytes(size_of::<AllocateHeader>());
pub(crate) const OBJECT_HEADER_SIZE: usize = align_to_8_bytes(size_of::<ObjectHeader>());
pub(crate) const ARRAY_HEADER_SIZE: usize = align_to_8_bytes(size_of::<ArrayHeader>());

#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AllocateHeader {
    #[bits(1)]
    pub(crate) kind: ReferenceValueType,
    #[bits(32)]
    pub(crate) size: usize,
    #[bits(31)]
    pub(crate) _no_use: i32,
}
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u64)]
pub enum ReferenceValueType {
    Object,
    Array,
}

impl ReferenceValueType {
    // This has to be a const fn
    const fn into_bits(self) -> u64 {
        self as _
    }

    const fn from_bits(value: u64) -> Self {
        match value {
            1 => Self::Array,
            _ => Self::Object,
        }
    }
}

pub enum ArrayElement<'a> {
    PrimaryValue(PrimaryType),
    ClassReference(ClassRef<'a>),
    Array(Value<'a>),
}
struct ArrayHeader<'a> {
    element: ArrayElement<'a>,
    array_size: u32,
}

struct ObjectHeader<'a> {
    class_ref: ClassRef<'a>,
}
