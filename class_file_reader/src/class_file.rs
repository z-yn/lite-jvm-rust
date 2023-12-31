use crate::class_file_version::ClassFileVersion;
use crate::constant_pool::ConstantPool;
use std::fmt::{Display, Formatter};

use crate::attribute_info::{AttributeInfo, AttributeType};
use crate::class_file_error::ClassFileError;
use crate::class_file_error::Result;
use crate::field_info::FieldInfo;
use crate::method_info::MethodInfo;
use bitflags::bitflags;
use cesu8::from_java_cesu8;
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl ClassFile {
    pub fn source_file(&self) -> Result<Option<String>> {
        for info in &self.attribute_info {
            if let AttributeType::SourceFile = info.name {
                return Ok(Some(
                    from_java_cesu8(&info.info)
                        .map_err(|_| ClassFileError::InvalidCesu8String)?
                        .to_string(),
                ));
            }
        }
        Ok(None)
    }
}

impl Display for ClassFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.access_flags.contains(ClassAccessFlags::PUBLIC) {
            write!(f, "public ")?;
        }
        if self.access_flags.contains(ClassAccessFlags::FINAL) {
            write!(f, "final ")?;
        }
        if self.access_flags.contains(ClassAccessFlags::ABSTRACT) {
            write!(f, "abstract ")?;
        }
        if self.access_flags.contains(ClassAccessFlags::INTERFACE) {
            write!(f, "interface {}", self.this_class_name)?;
        } else if self.access_flags.contains(ClassAccessFlags::ENUM) {
            write!(f, "enum {}", self.this_class_name)?;
        } else {
            write!(f, "class {}", self.this_class_name)?;
        }

        let version = self.version.version();
        writeln!(f, "minor version: {}", version.0)?;
        writeln!(f, "major version: {}", version.1)?;
        write!(f, "flags: ({:#06x}) ", self.access_flags.bits())?;
        writeln!(f, "this_class: {}", self.this_class_name)?;
        if let Some(super_class) = &self.super_class_name {
            writeln!(f, "super_class: {}", super_class)?;
        }
        writeln!(
            f,
            "interfaces: {}, fields: {}, method: {}, attributes: {}",
            self.interface_names.len(),
            self.field_info.len(),
            self.method_info.len(),
            self.attribute_info.len()
        )?;
        writeln!(f, "Constant pool:")?;
        write!(f, "{}", self.constant_pool)?;

        Ok(())
    }
}
