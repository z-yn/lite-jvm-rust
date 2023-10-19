use crate::cesu8_byte_buffer::ByteBuffer;
use crate::class_file_error::{ClassFileError, Result};
use std::fmt::{Display, Formatter};
pub type ConstantPoolIndex = u16;
//https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4
#[derive(Debug, PartialEq)]
pub enum ConstantPoolEntry {
    Utf8(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    ClassReference(ConstantPoolIndex),
    StringReference(ConstantPoolIndex),
    FieldReference(ConstantPoolIndex, ConstantPoolIndex),
    MethodReference(ConstantPoolIndex, ConstantPoolIndex),
    InterfaceMethodReference(ConstantPoolIndex, ConstantPoolIndex),
    NameAndTypeDescriptor(ConstantPoolIndex, ConstantPoolIndex),
    MethodHandler(u8, ConstantPoolIndex),
    MethodType(ConstantPoolIndex),
    Dynamic(u16, ConstantPoolIndex),
    InvokeDynamic(u16, ConstantPoolIndex),
    Module(ConstantPoolIndex),
    Package(ConstantPoolIndex),
}
/// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4
/// ```c
/// cp_info {
///     u1 tag;
///     u1 info[];
/// }
/// ```
/// tag 确定了字段类型，依据类型读取后续的信息。后续信息是个不定长的信息。
///
///
impl ConstantPoolEntry {
    pub fn read_from_bytes(buffer: &mut ByteBuffer) -> Result<ConstantPoolEntry> {
        let flag = buffer.read_u8()?;
        match flag {
            1 => ConstantPoolEntry::read_utf8(buffer),
            3 => buffer.read_i32().map(ConstantPoolEntry::Integer),
            4 => buffer.read_f32().map(ConstantPoolEntry::Float),
            5 => buffer.read_i64().map(ConstantPoolEntry::Long),
            6 => buffer.read_f64().map(ConstantPoolEntry::Double),
            7 => buffer.read_u16().map(ConstantPoolEntry::ClassReference),
            8 => buffer.read_u16().map(ConstantPoolEntry::StringReference),
            9 => buffer
                .read_2_u16()
                .map(|(f1, f2)| ConstantPoolEntry::FieldReference(f1, f2)),
            10 => buffer
                .read_2_u16()
                .map(|(f1, f2)| ConstantPoolEntry::MethodReference(f1, f2)),
            11 => buffer
                .read_2_u16()
                .map(|(f1, f2)| ConstantPoolEntry::InterfaceMethodReference(f1, f2)),
            12 => buffer
                .read_2_u16()
                .map(|(f1, f2)| ConstantPoolEntry::NameAndTypeDescriptor(f1, f2)),
            15 => buffer
                .read_u8_u16()
                .map(|(f1, f2)| ConstantPoolEntry::MethodHandler(f1, f2)),
            16 => buffer.read_u16().map(ConstantPoolEntry::MethodType),
            17 => buffer
                .read_2_u16()
                .map(|(f1, f2)| ConstantPoolEntry::Dynamic(f1, f2)),
            18 => buffer
                .read_2_u16()
                .map(|(f1, f2)| ConstantPoolEntry::InvokeDynamic(f1, f2)),
            19 => buffer.read_u16().map(ConstantPoolEntry::Module),
            20 => buffer.read_u16().map(ConstantPoolEntry::Package),
            t => Err(ClassFileError::ConstantPoolTagNotSupport(t)),
        }
    }

    fn read_utf8(buffer: &mut ByteBuffer) -> Result<ConstantPoolEntry> {
        let result = buffer.read_u16()?;
        buffer
            .read_utf8(result as usize)
            .map(ConstantPoolEntry::Utf8)
            .map_err(|err| err.into())
    }
}

//面向32位计算机设计的。所以double和long会占用两个字节，使用空占位符占位，
#[derive(Debug)]
pub enum ConstantPoolPhysicalEntry {
    Entry(ConstantPoolEntry),
    PlaceHolder,
}

/// Implementation of the constant pool of a java class.
/// Note that constants are 1-based in java.
#[derive(Debug, Default)]
pub struct ConstantPool {
    pub entries: Vec<ConstantPoolPhysicalEntry>,
}

impl ConstantPool {
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn new() -> ConstantPool {
        ConstantPool::default()
    }
    pub fn add(&mut self, entry: ConstantPoolEntry) {
        let take_two_words = matches!(
            &entry,
            ConstantPoolEntry::Long(_) | ConstantPoolEntry::Double(_)
        );
        self.entries.push(ConstantPoolPhysicalEntry::Entry(entry));
        if take_two_words {
            self.entries.push(ConstantPoolPhysicalEntry::PlaceHolder)
        }
    }
    pub fn try_get_string(&self, offset: &ConstantPoolIndex) -> Option<String> {
        if let Some(ConstantPoolEntry::Utf8(value)) = self.try_get(offset) {
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn get_string(&self, offset: &ConstantPoolIndex) -> Result<String> {
        if let ConstantPoolEntry::Utf8(value) = self.get(offset)? {
            Ok(value.clone())
        } else {
            Err(ClassFileError::InvalidClassData(format!(
                "Should be utf8 String at {offset}"
            )))
        }
    }

    pub fn try_get_class_name(&self, offset: &ConstantPoolIndex) -> Option<String> {
        if let Some(ConstantPoolEntry::ClassReference(value)) = self.try_get(offset) {
            self.try_get_string(value)
        } else {
            None
        }
    }

    pub fn get_class_name(&self, offset: &ConstantPoolIndex) -> Result<String> {
        if let ConstantPoolEntry::ClassReference(value) = self.get(offset)? {
            self.get_string(value)
        } else {
            Err(ClassFileError::InvalidClassData(format!(
                "Should be utf8 String at {offset}"
            )))
        }
    }

    pub fn get(&self, offset: &ConstantPoolIndex) -> Result<&ConstantPoolEntry> {
        let index = (offset - 1) as usize;
        if let Some(v) = self.entries.get(index) {
            match v {
                ConstantPoolPhysicalEntry::Entry(e) => Ok(e),
                ConstantPoolPhysicalEntry::PlaceHolder => {
                    Err(ClassFileError::InvalidConstantPoolIndexError(*offset))
                }
            }
        } else {
            Err(ClassFileError::InvalidConstantPoolIndexError(*offset))
        }
    }

    pub fn try_get(&self, offset: &ConstantPoolIndex) -> Option<&ConstantPoolEntry> {
        if *offset == 0 {
            return None;
        }
        let index = (offset - 1) as usize;
        if let Some(v) = self.entries.get(index) {
            match v {
                ConstantPoolPhysicalEntry::Entry(e) => Some(e),
                ConstantPoolPhysicalEntry::PlaceHolder => None,
            }
        } else {
            None
        }
    }

    pub fn fmt_entry(&self, offset: &ConstantPoolIndex) -> Result<String> {
        let entry = self.get(offset)?;
        let text = match entry {
            ConstantPoolEntry::Utf8(ref s) => format!("String: \"{s}\""),
            ConstantPoolEntry::Integer(n) => format!("Integer: {n}"),
            ConstantPoolEntry::Float(n) => format!("Float: {n}"),
            ConstantPoolEntry::Long(n) => format!("Long: {n}"),
            ConstantPoolEntry::Double(n) => format!("Double: {n}"),
            ConstantPoolEntry::ClassReference(n) => {
                format!("ClassReference: {} => ({})", n, self.fmt_entry(n)?)
            }
            ConstantPoolEntry::StringReference(n) => {
                format!("StringReference: {} => ({})", n, self.fmt_entry(n)?)
            }
            ConstantPoolEntry::FieldReference(i, j) => {
                format!(
                    "FieldReference: {}, {} => ({}), ({})",
                    i,
                    j,
                    self.fmt_entry(i)?,
                    self.fmt_entry(j)?
                )
            }
            ConstantPoolEntry::MethodReference(i, j) => {
                format!(
                    "MethodReference: {}, {} => ({}), ({})",
                    i,
                    j,
                    self.fmt_entry(i)?,
                    self.fmt_entry(j)?
                )
            }
            ConstantPoolEntry::InterfaceMethodReference(i, j) => {
                format!(
                    "InterfaceMethodReference: {}, {} => ({}), ({})",
                    i,
                    j,
                    self.fmt_entry(i)?,
                    self.fmt_entry(j)?
                )
            }
            ConstantPoolEntry::NameAndTypeDescriptor(i, j) => {
                format!(
                    "NameAndTypeDescriptor: {}, {} => ({}), ({})",
                    i,
                    j,
                    self.fmt_entry(i)?,
                    self.fmt_entry(j)?
                )
            }
            ConstantPoolEntry::MethodHandler(i, j) => format!(
                "MethodHandler: {}, {} => ({}), ({})",
                i,
                j,
                i,
                self.fmt_entry(j)?
            ),
            ConstantPoolEntry::MethodType(_) => todo!(),
            ConstantPoolEntry::Dynamic(_, _) => todo!(),
            ConstantPoolEntry::InvokeDynamic(_, _) => todo!(),
            ConstantPoolEntry::Module(_) => todo!(),
            ConstantPoolEntry::Package(_) => todo!(),
        };
        Ok(text)
    }
}

impl Display for ConstantPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Constant pool: (size: {})", self.entries.len())?;
        for (raw_idx, _) in self.entries.iter().enumerate() {
            let index = (raw_idx + 1) as u16;
            let entry_text = self
                .fmt_entry(&index)
                .map_err(|_| std::fmt::Error::default())?;
            writeln!(f, "    {}, {}", index, entry_text)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::class_file_error::ClassFileError;
    use crate::constant_pool::{ConstantPool, ConstantPoolEntry};

    #[test]
    fn constant_pool_works() {
        let mut cp = ConstantPool::new();
        cp.add(ConstantPoolEntry::Utf8("hey".to_string()));
        cp.add(ConstantPoolEntry::Integer(1));
        cp.add(ConstantPoolEntry::Float(2.1));
        cp.add(ConstantPoolEntry::Long(123));
        cp.add(ConstantPoolEntry::Double(3.56));
        cp.add(ConstantPoolEntry::ClassReference(1));
        cp.add(ConstantPoolEntry::StringReference(1));
        cp.add(ConstantPoolEntry::Utf8("joe".to_string()));
        cp.add(ConstantPoolEntry::FieldReference(1, 10));
        cp.add(ConstantPoolEntry::MethodReference(1, 10));
        cp.add(ConstantPoolEntry::InterfaceMethodReference(1, 10));
        cp.add(ConstantPoolEntry::NameAndTypeDescriptor(1, 10));

        assert_eq!(
            ConstantPoolEntry::Utf8("hey".to_string()),
            *cp.get(&1).unwrap()
        );
        assert_eq!(ConstantPoolEntry::Integer(1), *cp.get(&2).unwrap());
        assert_eq!(ConstantPoolEntry::Float(2.1), *cp.get(&3).unwrap());
        assert_eq!(ConstantPoolEntry::Long(123i64), *cp.get(&4).unwrap());
        matches!(
            cp.get(&5),
            Err(ClassFileError::InvalidConstantPoolIndexError(5)),
        );
        assert_eq!(ConstantPoolEntry::Double(3.56), *cp.get(&6).unwrap());
        assert_eq!(
            Err(ClassFileError::InvalidConstantPoolIndexError(7)),
            cp.get(&7)
        );
        assert_eq!(ConstantPoolEntry::ClassReference(1), *cp.get(&8).unwrap());
        assert_eq!(ConstantPoolEntry::StringReference(1), *cp.get(&9).unwrap());
        assert_eq!(
            ConstantPoolEntry::Utf8("joe".to_string()),
            *cp.get(&10).unwrap()
        );
        assert_eq!(
            ConstantPoolEntry::FieldReference(1, 10),
            *cp.get(&11).unwrap()
        );
        assert_eq!(
            ConstantPoolEntry::MethodReference(1, 10),
            *cp.get(&12).unwrap()
        );
        assert_eq!(
            ConstantPoolEntry::InterfaceMethodReference(1, 10),
            *cp.get(&13).unwrap()
        );
        assert_eq!(
            ConstantPoolEntry::NameAndTypeDescriptor(1, 10),
            *cp.get(&14).unwrap()
        );
    }
}
