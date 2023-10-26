use crate::jvm_exceptions::{Exception, Result};
use crate::loaded_class::{ClassRef, FieldRef};

use bitfield_struct::bitfield;
use std::marker::PhantomData;
use std::mem::size_of;
use thiserror::Error;

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
    ObjectRef(ObjectReference<'a>),
    ArrayRef(ArrayReference<'a>),
    Null,
}

pub trait ReferenceValue {
    fn get_data_length(&self) -> usize;
    fn data_offset(&self) -> usize;
    fn get_header(&self) -> AllocateHeader;
    fn set_field_by_name(&self, name: &str, value: &Value<'_>) -> Result<()>;
    fn set_field_by_offset(&self, offset: usize, value: &Value<'_>) -> Result<()>;
    fn get_field_by_name(&self, name: &str) -> Result<Value<'static>>;
    fn get_field_by_offset(&self, offset: usize) -> Result<Value<'static>>;
}

//数组引用分配
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ArrayReference<'a> {
    data: *mut u8,
    _marker: PhantomData<&'a [u8]>,
}

/// 对象引用分配
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ObjectReference<'a> {
    data: *mut u8,
    _marker: PhantomData<&'a [u8]>,
}
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

unsafe fn read_allocate_header(ptr: *const u8) -> AllocateHeader {
    unsafe { std::ptr::read(ptr as *const AllocateHeader) }
}

const fn align_to_8_bytes(required_size: usize) -> usize {
    match required_size % 8 {
        0 => required_size,
        n => required_size + (8 - n),
    }
}

#[derive(Error, Eq, PartialEq, Debug)]
pub enum AllocateError {
    #[error("value type error")]
    ValueTypeError,

    #[error("index out of bounds")]
    IndexOutOfBounds,
}

pub(crate) const ALLOC_HEADER_SIZE: usize = align_to_8_bytes(size_of::<AllocateHeader>());
pub(crate) const OBJECT_HEADER_SIZE: usize = align_to_8_bytes(size_of::<ObjectHeader>());
pub(crate) const ARRAY_HEADER_SIZE: usize = align_to_8_bytes(size_of::<ArrayHeader>());

macro_rules! read_value_at {
    ($name:ident,$variant:ident, $type:ty) => {
        pub(crate) unsafe fn $name(&self, index: usize) -> Result<Value<'static>> {
            let total_fields = self.get_data_length();
            if index >= total_fields {
                return Err(Exception::ExecuteCodeError(Box::new(
                    AllocateError::IndexOutOfBounds,
                )));
            }
            let offset = self.data_offset() + 8 * index;
            let pointer = self.data.add(offset);
            Ok(Value::$variant(std::ptr::read(pointer as *mut $type)))
        }
    };
}

macro_rules! read_nullable_value_at {
    ($name:ident,$variant:ident, $type:ty) => {
        pub(crate) unsafe fn $name(&self, index: usize) -> Result<Value<'static>> {
            let total_fields = self.get_data_length();
            if index >= total_fields {
                return Err(Exception::ExecuteCodeError(Box::new(
                    AllocateError::IndexOutOfBounds,
                )));
            }
            let offset = self.data_offset() + 8 * index;
            let pointer = self.data.add(offset);
            let data = std::ptr::read(pointer as *mut u64);
            if data == 0 {
                Ok(Value::Null)
            } else {
                Ok(Value::$variant(std::ptr::read(pointer as *mut $type)))
            }
        }
    };
}

macro_rules! write_value_at {
    ($name:ident,$variant:ident, $type:ty) => {
        pub(crate) unsafe fn $name(&self, index: usize, value: &Value<'a>) -> Result<()> {
            let total_fields = self.get_data_length();
            if index >= total_fields {
                return Err(Exception::ExecuteCodeError(Box::new(
                    AllocateError::IndexOutOfBounds,
                )));
            }
            let offset = self.data_offset() + 8 * index;
            let pointer = self.data.add(offset);
            if let Value::$variant(v) = value {
                std::ptr::write(pointer as *mut $type, *v);
                Ok(())
            } else {
                Err(Exception::ExecuteCodeError(Box::new(
                    AllocateError::ValueTypeError,
                )))
            }
        }
    };
}

macro_rules! write_nullable_value_at {
    ($name:ident,$variant:ident, $type:ty) => {
        pub(crate) unsafe fn $name(&self, index: usize, value: &Value<'a>) -> Result<()> {
            let total_fields = self.get_data_length();
            if index >= total_fields {
                return Err(Exception::ExecuteCodeError(Box::new(
                    AllocateError::IndexOutOfBounds,
                )));
            }
            let offset = self.data_offset() + 8 * index;
            let pointer = self.data.add(offset);
            match value {
                Value::$variant(v) => {
                    std::ptr::write(pointer as *mut $type, *v);
                    Ok(())
                }
                Value::Null => {
                    std::ptr::write(pointer as *mut u64, 0);
                    Ok(())
                }
                _ => Err(Exception::ExecuteCodeError(Box::new(
                    AllocateError::ValueTypeError,
                ))),
            }
        }
    };
}

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
    Array(Box<ArrayElement<'a>>),
}

impl<'a> ArrayElement<'a> {
    fn is_subclass_of(&self, target_element_type: &ArrayElement<'a>) -> bool {
        match self {
            ArrayElement::PrimaryValue(my_type) => {
                if let ArrayElement::PrimaryValue(target) = target_element_type {
                    my_type == target
                } else {
                    false
                }
            }
            ArrayElement::ClassReference(class_ref) => {
                if let ArrayElement::ClassReference(target) = target_element_type {
                    class_ref.is_subclass_of(&target.name)
                } else {
                    false
                }
            }
            ArrayElement::Array(inner) => {
                if let ArrayElement::Array(target) = target_element_type {
                    inner.is_subclass_of(target)
                } else {
                    false
                }
            }
        }
    }
}
struct ArrayHeader<'a> {
    element: ArrayElement<'a>,
    array_size: usize,
}

struct ObjectHeader<'a> {
    class_ref: ClassRef<'a>,
}

impl<'a> ArrayReference<'a> {
    fn get_array_header(&self) -> ArrayHeader {
        unsafe {
            let class_ref_ptr = self.data.add(ALLOC_HEADER_SIZE);
            std::ptr::read(class_ref_ptr as *const ArrayHeader)
        }
    }

    fn get_array_type(&self) -> ArrayElement {
        self.get_array_header().element
    }

    pub(crate) fn is_instance_of(&self, target_type: &ArrayElement<'a>) -> bool {
        self.get_array_type().is_subclass_of(target_type)
    }

    read_value_at!(read_byte, Byte, i8);
    read_value_at!(read_int, Int, i32);
    read_value_at!(read_char, Char, u16);
    read_value_at!(read_long, Long, i64);
    read_value_at!(read_short, Short, i16);
    read_value_at!(read_float, Float, f32);
    read_value_at!(read_double, Double, f64);
    read_value_at!(read_boolean, Boolean, bool);
    read_nullable_value_at!(read_object, ObjectRef, ObjectReference<'static>);
    read_nullable_value_at!(read_array, ArrayRef, ArrayReference<'static>);

    write_value_at!(write_byte, Byte, i8);
    write_value_at!(write_int, Int, i32);
    write_value_at!(write_char, Char, u16);
    write_value_at!(write_long, Long, i64);
    write_value_at!(write_short, Short, i16);
    write_value_at!(write_float, Float, f32);
    write_value_at!(write_double, Double, f64);
    write_value_at!(write_boolean, Boolean, bool);
    write_nullable_value_at!(write_object, ObjectRef, ObjectReference<'a>);
    write_nullable_value_at!(write_array, ArrayRef, ArrayReference<'a>);

    pub(crate) fn new_array(
        element: ArrayElement,
        array_size: usize,
        start_ptr: *const u8,
        size: usize,
    ) -> ArrayReference<'a> {
        unsafe {
            let next_ptr = write_allocate_header(
                start_ptr,
                AllocateHeader::new()
                    .with_kind(ReferenceValueType::Array)
                    .with_size(size),
            );
            std::ptr::write(
                next_ptr as *mut ArrayHeader,
                ArrayHeader {
                    element,
                    array_size,
                },
            );
        }
        ArrayReference {
            data: start_ptr as *mut u8,
            _marker: Default::default(),
        }
    }
}

impl<'a> ReferenceValue for ArrayReference<'a> {
    fn get_data_length(&self) -> usize {
        self.get_array_header().array_size
    }

    fn data_offset(&self) -> usize {
        ALLOC_HEADER_SIZE + ARRAY_HEADER_SIZE
    }

    fn get_header(&self) -> AllocateHeader {
        unsafe { read_allocate_header(self.data) }
    }
    fn set_field_by_name(&self, name: &str, value: &Value<'_>) -> Result<()> {
        self.set_field_by_offset(name.parse::<usize>().unwrap(), value)
    }

    fn set_field_by_offset(&self, offset: usize, value: &Value<'_>) -> Result<()> {
        let element = self.get_array_type();
        unsafe {
            match element {
                ArrayElement::PrimaryValue(v) => match v {
                    PrimaryType::Byte => self.write_byte(offset, value),
                    PrimaryType::Char => self.write_char(offset, value),
                    PrimaryType::Double => self.write_double(offset, value),
                    PrimaryType::Float => self.write_float(offset, value),
                    PrimaryType::Int => self.write_int(offset, value),
                    PrimaryType::Long => self.write_long(offset, value),
                    PrimaryType::Short => self.write_short(offset, value),
                    PrimaryType::Boolean => self.write_boolean(offset, value),
                },
                ArrayElement::ClassReference(_) => self.write_object(offset, value),
                ArrayElement::Array(_) => self.write_array(offset, value),
            }
        }
    }
    fn get_field_by_name(&self, name: &str) -> Result<Value<'static>> {
        self.get_field_by_offset(name.parse::<usize>().unwrap())
    }

    fn get_field_by_offset(&self, offset: usize) -> Result<Value<'static>> {
        let element_type = self.get_array_type();
        unsafe {
            match element_type {
                ArrayElement::PrimaryValue(v) => match v {
                    PrimaryType::Byte => self.read_byte(offset),
                    PrimaryType::Char => self.read_char(offset),
                    PrimaryType::Double => self.read_double(offset),
                    PrimaryType::Float => self.read_float(offset),
                    PrimaryType::Int => self.read_int(offset),
                    PrimaryType::Long => self.read_long(offset),
                    PrimaryType::Short => self.read_short(offset),
                    PrimaryType::Boolean => self.read_boolean(offset),
                },
                ArrayElement::ClassReference(_) => self.read_object(offset),
                ArrayElement::Array(_) => self.read_array(offset),
            }
        }
    }
}

impl<'a> ObjectReference<'a> {
    pub(crate) fn new_object(
        class_ref: ClassRef,
        start_ptr: *const u8,
        size: usize,
    ) -> ObjectReference<'a> {
        unsafe {
            let next_ptr = write_allocate_header(
                start_ptr,
                AllocateHeader::new()
                    .with_kind(ReferenceValueType::Object)
                    .with_size(size),
            );
            std::ptr::write(next_ptr as *mut ObjectHeader, ObjectHeader { class_ref });
        }
        ObjectReference {
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

    write_value_at!(write_byte, Byte, i8);
    write_value_at!(write_int, Int, i32);
    write_value_at!(write_char, Char, u16);
    write_value_at!(write_long, Long, i64);
    write_value_at!(write_short, Short, i16);
    write_value_at!(write_float, Float, f32);
    write_value_at!(write_double, Double, f64);
    write_value_at!(write_boolean, Boolean, bool);
    write_value_at!(write_object, ObjectRef, ObjectReference);
    write_value_at!(write_array, ArrayRef, ArrayReference);

    //TODO 校验Value与RuntimeFieldInfo是否一致
    unsafe fn write_value_at_offset(&self, field: FieldRef<'a>, value: &Value<'a>) -> Result<()> {
        let offset = field.offset - 1;
        assert!(offset > 0);
        match field.descriptor.as_str() {
            "B" => self.write_byte(offset, value),
            "C" => self.write_char(offset, value),
            "D" => self.write_double(offset, value),
            "F" => self.write_float(offset, value),
            "I" => self.write_int(offset, value),
            "J" => self.write_long(offset, value),
            "S" => self.write_short(offset, value),
            "Z" => self.write_boolean(offset, value),
            other => {
                if other.starts_with("[") {
                    self.write_array(offset, value)
                } else {
                    self.write_object(offset, value)
                }
            }
        }
    }

    //TODO 校验Value与RuntimeFieldInfo是否一致
    unsafe fn read_value_at_offset(&self, field: FieldRef) -> Result<Value<'static>> {
        let offset = field.offset - 1;
        assert!(offset > 0);
        match field.descriptor.as_str() {
            "B" => self.read_byte(offset),
            "C" => self.read_char(offset),
            "D" => self.read_double(offset),
            "F" => self.read_float(offset),
            "I" => self.read_int(offset),
            "J" => self.read_long(offset),
            "S" => self.read_short(offset),
            "Z" => self.read_boolean(offset),
            other => {
                if other.starts_with("[") {
                    self.read_array(offset)
                } else {
                    self.read_object(offset)
                }
            }
        }
    }

    read_value_at!(read_byte, Byte, i8);
    read_value_at!(read_int, Int, i32);
    read_value_at!(read_char, Char, u16);
    read_value_at!(read_long, Long, i64);
    read_value_at!(read_short, Short, i16);
    read_value_at!(read_float, Float, f32);
    read_value_at!(read_double, Double, f64);
    read_value_at!(read_boolean, Boolean, bool);
    read_value_at!(read_object, ObjectRef, ObjectReference);
    read_value_at!(read_array, ArrayRef, ArrayReference);

    pub fn is_instance_of(&self, class_ref: ClassRef<'a>) -> bool {
        self.get_class().is_subclass_of(&class_ref.name)
    }
}

impl<'a> ReferenceValue for ObjectReference<'a> {
    fn get_data_length(&self) -> usize {
        self.get_class().total_num_of_fields
    }

    fn data_offset(&self) -> usize {
        ALLOC_HEADER_SIZE + OBJECT_HEADER_SIZE
    }

    fn get_header(&self) -> AllocateHeader {
        unsafe { read_allocate_header(self.data) }
    }

    fn set_field_by_name(&self, name: &str, value: &Value<'_>) -> Result<()> {
        //先查找自身类中的field
        let class = self.get_class();
        if let Some(field) = class.fields.get(name) {
            unsafe {
                return self.write_value_at_offset(field, value);
            }
        }
        if let Some(super_class) = class.super_class {
            if let Some(field) = super_class.fields.get(name) {
                unsafe {
                    return self.write_value_at_offset(field, value);
                }
            }
        }
        Err(Exception::FieldNotFoundException(name.to_string()))
    }

    fn set_field_by_offset(&self, offset: usize, value: &Value<'_>) -> Result<()> {
        let class_ref = self.get_class();
        let field = class_ref.get_field(offset)?;
        unsafe { self.write_value_at_offset(field, value) }
    }

    fn get_field_by_name(&self, name: &str) -> Result<Value<'static>> {
        //先查找自身类中的field
        let class = self.get_class();
        if let Some(field) = class.fields.get(name) {
            return unsafe { self.read_value_at_offset(field) };
        }
        if let Some(super_class) = class.super_class {
            if let Some(field) = super_class.fields.get(name) {
                return unsafe { self.read_value_at_offset(field) };
            }
        }
        Err(Exception::FieldNotFoundException(name.to_string()))
    }

    fn get_field_by_offset(&self, offset: usize) -> Result<Value<'static>> {
        let class_ref = self.get_class();
        let field = class_ref.get_field(offset)?;
        unsafe { self.read_value_at_offset(field) }
    }
}
