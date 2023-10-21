use crate::loaded_class::ClassRef;
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
