use crate::jvm_error::{VmError, VmExecResult};
use crate::loaded_class::{ClassRef, FieldRef};

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
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ReturnAddress(u32),
    ObjectRef(ObjectReference<'a>),
    ArrayRef(ArrayReference<'a>),
    Null,
}

macro_rules! generate_get_value {
    ($name:ident, $variant:ident, $type:ty) => {
        pub fn $name(&self) -> VmExecResult<$type> {
            if let Value::$variant(v) = self {
                Ok(*v)
            } else {
                Err(VmError::ValueTypeMissMatch)
            }
        }
    };
}

impl<'a> Value<'a> {
    generate_get_value!(get_int, Int, i32);
    generate_get_value!(get_long, Long, i64);
    generate_get_value!(get_float, Float, f32);
    generate_get_value!(get_double, Double, f64);
    generate_get_value!(get_object, ObjectRef, ObjectReference<'a>);
    generate_get_value!(get_array, ArrayRef, ArrayReference<'a>);
    pub fn get_string(&self) -> VmExecResult<String> {
        let string_object = self.get_object()?;
        assert_eq!(string_object.get_class().name, "java/lang/String");
        let bytes: Vec<u16> = string_object
            .get_field_by_name("value")?
            .get_array()?
            .read_all()
            .iter()
            .map(|v| v.get_int().unwrap() as u16)
            .collect();
        Ok(String::from_utf16_lossy(&bytes))
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    Primary(PrimaryType),
    Object(String),
    //类型，dimension
    PrimaryArray(PrimaryType, usize),
    ObjectArray(String, usize),
    Void,
}

pub trait ReferenceValue<'a> {
    fn ptr(&self) -> *mut u8;
    fn get_data_length(&self) -> usize;
    fn data_offset(&self) -> usize;
    fn get_header(&self) -> AllocateHeader;
    fn set_field_by_name(&self, name: &str, value: &Value<'_>) -> VmExecResult<()>;
    fn set_field_by_offset(&self, offset: usize, value: &Value<'_>) -> VmExecResult<()>;
    fn get_field_by_name(&self, name: &str) -> VmExecResult<Value<'a>>;
    fn get_field_by_offset(&self, offset: usize) -> VmExecResult<Value<'a>>;

    fn as_value(&self) -> Value<'a>;
    fn hash_code(&self) -> i32;

    fn outbound(&self) -> usize {
        self.data_offset() + self.get_data_length() * 8
    }

    fn copy_to(&self, to: &Self) {
        unsafe { std::ptr::copy(self.ptr(), to.ptr(), self.outbound()) }
    }
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
pub(crate) const ALLOC_HEADER_SIZE: usize = align_to_8_bytes(size_of::<AllocateHeader>());
pub(crate) const OBJECT_HEADER_SIZE: usize = align_to_8_bytes(size_of::<ObjectHeader>());
pub(crate) const ARRAY_HEADER_SIZE: usize = align_to_8_bytes(size_of::<ArrayHeader>());

macro_rules! read_value_at {
    ($name:ident,$variant:ident, $type:ty) => {
        pub(crate) unsafe fn $name(&self, index: usize) -> VmExecResult<Value<'a>> {
            let total_fields = self.get_data_length();
            if index >= total_fields {
                return Err(VmError::IndexOutOfBounds);
            }
            let offset = self.data_offset() + 8 * index;
            let pointer = self.data.add(offset);
            Ok(Value::$variant(std::ptr::read(pointer as *mut $type)))
        }
    };
}

macro_rules! read_nullable_value_at {
    ($name:ident,$variant:ident, $type:ty) => {
        pub(crate) unsafe fn $name(&self, index: usize) -> VmExecResult<Value<'a>> {
            let total_fields = self.get_data_length();
            if index >= total_fields {
                return Err(VmError::IndexOutOfBounds);
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
        pub(crate) unsafe fn $name(&self, index: usize, value: &Value<'a>) -> VmExecResult<()> {
            let total_fields = self.get_data_length();
            if index >= total_fields {
                return Err(VmError::IndexOutOfBounds);
            }
            let offset = self.data_offset() + 8 * index;
            let pointer = self.data.add(offset);
            if let Value::$variant(v) = value {
                std::ptr::write(pointer as *mut $type, *v);
                Ok(())
            } else {
                Err(VmError::ValueTypeMissMatch)
            }
        }
    };
}

macro_rules! write_nullable_value_at {
    ($name:ident,$variant:ident, $type:ty) => {
        pub(crate) unsafe fn $name(&self, index: usize, value: &Value<'a>) -> VmExecResult<()> {
            let total_fields = self.get_data_length();
            if index >= total_fields {
                return Err(VmError::IndexOutOfBounds);
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
                _ => Err(VmError::ValueTypeMissMatch),
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
pub struct ArrayHeader<'a> {
    pub(crate) element: ArrayElement<'a>,
    pub(crate) array_size: usize,
}

pub struct ObjectHeader<'a> {
    class_ref: ClassRef<'a>,
}

impl<'a> ArrayReference<'a> {
    pub fn get_array_header(&self) -> ArrayHeader {
        unsafe {
            let class_ref_ptr = self.data.add(ALLOC_HEADER_SIZE);
            std::ptr::read(class_ref_ptr as *const ArrayHeader)
        }
    }

    pub fn read_all(&self) -> Vec<Value<'a>> {
        let header = self.get_array_header();
        (0..header.array_size)
            .map(|i| self.get_field_by_offset(i).unwrap())
            .collect()
    }

    pub fn get_array_type(&self) -> ArrayElement {
        self.get_array_header().element
    }

    pub(crate) fn is_instance_of(&self, target_type: &ArrayElement<'a>) -> bool {
        self.get_array_type().is_subclass_of(target_type)
    }

    read_value_at!(read_int, Int, i32);
    read_value_at!(read_long, Long, i64);
    read_value_at!(read_float, Float, f32);
    read_value_at!(read_double, Double, f64);
    read_nullable_value_at!(read_object, ObjectRef, ObjectReference<'a>);
    read_nullable_value_at!(read_array, ArrayRef, ArrayReference<'a>);

    write_value_at!(write_int, Int, i32);
    write_value_at!(write_long, Long, i64);
    write_value_at!(write_float, Float, f32);
    write_value_at!(write_double, Double, f64);
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

impl<'a> ReferenceValue<'a> for ArrayReference<'a> {
    fn ptr(&self) -> *mut u8 {
        self.data
    }

    fn get_data_length(&self) -> usize {
        self.get_array_header().array_size
    }

    fn data_offset(&self) -> usize {
        ALLOC_HEADER_SIZE + ARRAY_HEADER_SIZE
    }
    fn get_header(&self) -> AllocateHeader {
        unsafe { read_allocate_header(self.data) }
    }

    fn set_field_by_name(&self, name: &str, value: &Value<'_>) -> VmExecResult<()> {
        self.set_field_by_offset(name.parse::<usize>().unwrap(), value)
    }
    fn set_field_by_offset(&self, offset: usize, value: &Value<'_>) -> VmExecResult<()> {
        let element = self.get_array_type();
        unsafe {
            match element {
                ArrayElement::PrimaryValue(v) => match v {
                    PrimaryType::Byte
                    | PrimaryType::Short
                    | PrimaryType::Boolean
                    | PrimaryType::Char
                    | PrimaryType::Int => self.write_int(offset, value),
                    PrimaryType::Double => self.write_double(offset, value),
                    PrimaryType::Float => self.write_float(offset, value),
                    PrimaryType::Long => self.write_long(offset, value),
                },
                ArrayElement::ClassReference(_) => self.write_object(offset, value),
                ArrayElement::Array(_) => self.write_array(offset, value),
            }
        }
    }

    fn get_field_by_name(&self, name: &str) -> VmExecResult<Value<'a>> {
        self.get_field_by_offset(name.parse::<usize>().unwrap())
    }

    fn get_field_by_offset(&self, offset: usize) -> VmExecResult<Value<'a>> {
        let element_type = self.get_array_type();
        unsafe {
            match element_type {
                ArrayElement::PrimaryValue(v) => match v {
                    PrimaryType::Double => self.read_double(offset),
                    PrimaryType::Float => self.read_float(offset),
                    PrimaryType::Long => self.read_long(offset),
                    PrimaryType::Int
                    | PrimaryType::Byte
                    | PrimaryType::Char
                    | PrimaryType::Short
                    | PrimaryType::Boolean => self.read_int(offset),
                },
                ArrayElement::ClassReference(_) => self.read_object(offset),
                ArrayElement::Array(_) => self.read_array(offset),
            }
        }
    }

    fn as_value(&self) -> Value<'a> {
        Value::ArrayRef(*self)
    }

    fn hash_code(&self) -> i32 {
        self.data as i32
    }
}

impl<'a> ObjectReference<'a> {
    pub(crate) fn hash_code(&self) -> i32 {
        self.data as i32
    }
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

    pub(crate) fn get_class(&self) -> ClassRef<'a> {
        unsafe {
            let class_ref_ptr = self.data.add(ALLOC_HEADER_SIZE);
            std::ptr::read(class_ref_ptr as *const ObjectHeader).class_ref
        }
    }

    write_value_at!(write_int, Int, i32);
    write_value_at!(write_long, Long, i64);
    write_value_at!(write_float, Float, f32);
    write_value_at!(write_double, Double, f64);
    pub(crate) unsafe fn write_reference(
        &self,
        index: usize,
        value: &Value<'a>,
    ) -> VmExecResult<()> {
        let total_fields = self.get_data_length();
        if index >= total_fields {
            return Err(VmError::IndexOutOfBounds);
        }
        let offset = self.data_offset() + 8 * index;
        let pointer = self.data.add(offset);
        match value {
            Value::ObjectRef(v) => {
                std::ptr::write(pointer as *mut ObjectReference, *v);
                Ok(())
            }
            Value::ArrayRef(v) => {
                std::ptr::write(pointer as *mut ArrayReference, *v);
                Ok(())
            }
            Value::Null => {
                std::ptr::write(pointer as *mut u64, 0);
                Ok(())
            }
            _ => Err(VmError::ValueTypeMissMatch),
        }
    }
    write_nullable_value_at!(write_object, ObjectRef, ObjectReference);
    write_nullable_value_at!(write_array, ArrayRef, ArrayReference);

    //TODO 校验Value与RuntimeFieldInfo是否一致
    unsafe fn write_value_at_offset(
        &self,
        field: FieldRef<'a>,
        value: &Value<'a>,
    ) -> VmExecResult<()> {
        let offset = field.offset - 1;
        match field.descriptor.as_str() {
            "B" => self.write_int(offset, value),
            "C" => self.write_int(offset, value),
            "D" => self.write_double(offset, value),
            "F" => self.write_float(offset, value),
            "I" => self.write_int(offset, value),
            "J" => self.write_long(offset, value),
            "S" => self.write_int(offset, value),
            "Z" => self.write_int(offset, value),
            //Object是所有的父类
            "Ljava/lang/Object;" => self.write_reference(offset, value),
            other => {
                if other.starts_with('[') {
                    self.write_array(offset, value)
                } else {
                    self.write_object(offset, value)
                }
            }
        }
    }

    //TODO 校验Value与RuntimeFieldInfo是否一致
    unsafe fn read_value_at_offset(&self, field: FieldRef) -> VmExecResult<Value<'a>> {
        assert!(field.offset > 0);
        let offset = field.offset - 1;
        match field.descriptor.as_str() {
            "B" => self.read_int(offset),
            "C" => self.read_int(offset),
            "D" => self.read_double(offset),
            "F" => self.read_float(offset),
            "I" => self.read_int(offset),
            "J" => self.read_long(offset),
            "S" => self.read_int(offset),
            "Z" => self.read_int(offset),
            //Object是所有的父类
            "Ljava/lang/Object;" => self.read_reference(offset),
            other => {
                if other.starts_with('[') {
                    self.read_array(offset)
                } else {
                    self.read_object(offset)
                }
            }
        }
    }

    read_value_at!(read_int, Int, i32);
    read_value_at!(read_long, Long, i64);
    read_value_at!(read_float, Float, f32);
    read_value_at!(read_double, Double, f64);
    pub(crate) unsafe fn read_reference(&self, index: usize) -> VmExecResult<Value<'a>> {
        let total_fields = self.get_data_length();
        if index >= total_fields {
            return Err(VmError::IndexOutOfBounds);
        }
        let value_type = self.get_header().kind();

        let offset = self.data_offset() + 8 * index;
        let pointer = self.data.add(offset);
        let data = std::ptr::read(pointer as *mut u64);
        if data == 0 {
            Ok(Value::Null)
        } else {
            match value_type {
                ReferenceValueType::Object => Ok(Value::ObjectRef(std::ptr::read(
                    pointer as *mut ObjectReference,
                ))),
                ReferenceValueType::Array => Ok(Value::ArrayRef(std::ptr::read(
                    pointer as *mut ArrayReference,
                ))),
            }
        }
    }
    read_nullable_value_at!(read_object, ObjectRef, ObjectReference);
    read_nullable_value_at!(read_array, ArrayRef, ArrayReference);

    pub fn is_instance_of(&self, class_ref: ClassRef<'a>) -> bool {
        self.get_class().is_subclass_of(&class_ref.name)
    }
}

impl<'a> ReferenceValue<'a> for ObjectReference<'a> {
    fn ptr(&self) -> *mut u8 {
        self.data
    }

    fn get_data_length(&self) -> usize {
        self.get_class().total_num_of_fields
    }

    fn data_offset(&self) -> usize {
        ALLOC_HEADER_SIZE + OBJECT_HEADER_SIZE
    }

    fn get_header(&self) -> AllocateHeader {
        unsafe { read_allocate_header(self.data) }
    }

    fn set_field_by_name(&self, name: &str, value: &Value<'_>) -> VmExecResult<()> {
        //先查找自身类中的field
        let class = self.get_class();
        let field = class.get_field_by_name(name)?;
        unsafe { self.write_value_at_offset(field, value) }
    }

    fn set_field_by_offset(&self, offset: usize, value: &Value<'_>) -> VmExecResult<()> {
        let class_ref = self.get_class();
        let field = class_ref.get_field(offset)?;
        unsafe { self.write_value_at_offset(field, value) }
    }

    fn get_field_by_name(&self, name: &str) -> VmExecResult<Value<'a>> {
        //先查找自身类中的field
        let class_ref = self.get_class();
        let field = class_ref.get_field_by_name(name)?;
        unsafe { self.read_value_at_offset(field) }
    }

    fn get_field_by_offset(&self, offset: usize) -> VmExecResult<Value<'a>> {
        let class_ref = self.get_class();
        let field = class_ref.get_field(offset)?;
        unsafe { self.read_value_at_offset(field) }
    }

    fn as_value(&self) -> Value<'a> {
        Value::ObjectRef(*self)
    }

    fn hash_code(&self) -> i32 {
        self.data as i32
    }
}

mod tests {

    #[test]
    fn test_value() {
        use crate::jvm_values::Value;

        assert_eq!(Value::Int(1), Value::Int(1));
        assert_ne!(Value::Int(1), Value::Double(1f64));
        assert_ne!(Value::Int(1), Value::Null);
    }
}
