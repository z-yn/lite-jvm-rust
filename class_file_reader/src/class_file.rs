use crate::class_file_version::ClassFileVersion;
use crate::constant_pool::ConstantPool;
use std::fmt::{Display, Formatter};

use crate::attribute_info::AttributeInfo;
use crate::field_info::FieldInfo;
use crate::method_info::MethodInfo;
use bitflags::bitflags;
bitflags! {
    /// Class flags
    /// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.1-200-E.1
    ///
    /// | Flag Name |	Value |	Interpretation |
    /// | -----     |-----    | ------------------|
    /// |ACC_PUBLIC	|0x0001   |	Declared public; may be accessed from outside its package.|
    /// |ACC_FINAL	| 0x0010  | Declared final; no subclasses allowed.|
    /// |ACC_SUPER	| 0x0020	|Treat superclass methods specially when invoked by the invokespecial instruction.|
    /// |ACC_INTERFACE|	0x0200	|Is an interface, not a class.|
    /// |ACC_ABSTRACT	|0x0400	|Declared abstract; must not be instantiated.|
    /// |ACC_SYNTHETIC	|0x1000	|Declared synthetic; not present in the source code.|
    /// |ACC_ANNOTATION	|0x2000	|Declared as an annotation interface.|
    /// |ACC_ENUM	|0x4000	|Declared as an enum class.|
    /// |ACC_MODULE  |0x8000|	Is a module, not a class or interface.|
    ///

    pub struct ClassAccessFlags: u16 {
        const PUBLIC = 0x0001;
        const FINAL = 0x0010;
        const SUPER = 0x0020;
        const INTERFACE = 0x0200;
        const ABSTRACT = 0x0400;
        const SYNTHETIC = 0x1000;
        const ANNOTATION = 0x2000;
        const ENUM = 0x4000;
        const MODULE = 0x8000;
    }
}

impl Default for ClassAccessFlags {
    fn default() -> ClassAccessFlags {
        ClassAccessFlags::empty()
    }
}

#[allow(dead_code)]
pub struct ClassFile {
    pub version: ClassFileVersion,
    pub constant_pool: ConstantPool,
    pub access_flags: ClassAccessFlags,
    //常量池中数据，对应的是classInfo中只有名称常量是有用的。直接取出来
    pub this_class_name: String,
    pub super_class_name: Option<String>,
    pub interface_names: Vec<String>,

    pub field_info: Vec<FieldInfo>,
    pub method_info: Vec<MethodInfo>,
    pub attribute_info: Vec<AttributeInfo>,
}

impl Display for ClassFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Class {}", self.this_class_name)?;
        if let Some(super_class) = &self.super_class_name {
            write!(f, "(extends {})", super_class)?;
        }
        // writeln!(f, "accessFlag:{:?}", self.access_flags)?;
        writeln!(f, "version: {}", self.version)?;
        writeln!(f, "constants:")?;
        write!(f, "{}", self.constant_pool)?;
        Ok(())
    }
}
