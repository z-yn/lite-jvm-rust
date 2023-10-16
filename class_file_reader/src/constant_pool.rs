use std::fmt::{Display, Formatter};

use crate::class_file_error::ClassFileError;
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
//https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-5.html#jvms-5.4.3.5
#[derive(Debug, PartialEq, Eq)]
pub enum MethodHandlerKind {
    GetField,
    GetStatic,
    PutField,
    PutStatic,
    InvokeVirtual,
    InvokeStatic,
    InvokeSpecial,
    NewInvokeSpecial,
    InvokeInterface,
}

impl MethodHandlerKind {
    fn new(kind: u8) -> Result<MethodHandlerKind, ClassFileError> {
        match kind {
            1 => Ok(MethodHandlerKind::GetField),
            2 => Ok(MethodHandlerKind::GetStatic),
            3 => Ok(MethodHandlerKind::PutField),
            4 => Ok(MethodHandlerKind::PutStatic),
            5 => Ok(MethodHandlerKind::InvokeVirtual),
            6 => Ok(MethodHandlerKind::InvokeSpecial),
            7 => Ok(MethodHandlerKind::NewInvokeSpecial),
            8 => Ok(MethodHandlerKind::InvokeInterface),
            _ => Err(ClassFileError::InvalidMethodHandlerKind(kind)),
        }
    }
}
impl Display for MethodHandlerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodHandlerKind::GetField => write!(f, "getfield C.f:T"),
            MethodHandlerKind::GetStatic => write!(f, "getstatic C.f:T"),
            MethodHandlerKind::PutField => write!(f, "putfield C.f:T"),
            MethodHandlerKind::PutStatic => write!(f, "putstatic C.f:T"),
            MethodHandlerKind::InvokeVirtual => write!(f, "invokevirtual C.m:(A*)T"),
            MethodHandlerKind::InvokeStatic => write!(f, "invokestatic C.m:(A*)T"),
            MethodHandlerKind::InvokeSpecial => write!(f, "invokespecial C.m:(A*)T"),
            MethodHandlerKind::NewInvokeSpecial => {
                write!(f, "new C; dup; invokespecial C.<init>:(A*)V")
            }
            MethodHandlerKind::InvokeInterface => write!(f, "invokeinterface C.m:(A*)T"),
        }
    }
}

//面向32位计算机设计的。所以double和long会占用两个字节，使用空占位符占位，
#[derive(Debug)]
enum ConstantPoolPhysicalEntry {
    Entry(ConstantPoolEntry),
    PlaceHolder,
}

/// Implementation of the constant pool of a java class.
/// Note that constants are 1-based in java.
#[derive(Debug, Default)]
pub struct ConstantPool {
    entries: Vec<ConstantPoolPhysicalEntry>,
}

impl ConstantPool {
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

    pub fn get(&self, offset: u16) -> Result<&ConstantPoolEntry, ClassFileError> {
        let index = (offset - 1) as usize;
        if let Some(v) = self.entries.get(index) {
            match v {
                ConstantPoolPhysicalEntry::Entry(e) => Ok(e),
                ConstantPoolPhysicalEntry::PlaceHolder => {
                    Err(ClassFileError::InvalidConstantPoolIndexError(offset))
                }
            }
        } else {
            Err(ClassFileError::InvalidConstantPoolIndexError(offset))
        }
    }

    pub fn fmt_entry(&self, offset: ConstantPoolIndex) -> Result<String, ClassFileError> {
        let entry = self.get(offset)?;
        let text = match entry {
            ConstantPoolEntry::Utf8(ref s) => format!("String: \"{s}\""),
            ConstantPoolEntry::Integer(n) => format!("Integer: {n}"),
            ConstantPoolEntry::Float(n) => format!("Float: {n}"),
            ConstantPoolEntry::Long(n) => format!("Long: {n}"),
            ConstantPoolEntry::Double(n) => format!("Double: {n}"),
            ConstantPoolEntry::ClassReference(n) => {
                format!("ClassReference: {} => ({})", n, self.fmt_entry(*n)?)
            }
            ConstantPoolEntry::StringReference(n) => {
                format!("StringReference: {} => ({})", n, self.fmt_entry(*n)?)
            }
            ConstantPoolEntry::FieldReference(i, j) => {
                format!(
                    "FieldReference: {}, {} => ({}), ({})",
                    i,
                    j,
                    self.fmt_entry(*i)?,
                    self.fmt_entry(*j)?
                )
            }
            ConstantPoolEntry::MethodReference(i, j) => {
                format!(
                    "MethodReference: {}, {} => ({}), ({})",
                    i,
                    j,
                    self.fmt_entry(*i)?,
                    self.fmt_entry(*j)?
                )
            }
            ConstantPoolEntry::InterfaceMethodReference(i, j) => {
                format!(
                    "InterfaceMethodReference: {}, {} => ({}), ({})",
                    i,
                    j,
                    self.fmt_entry(*i)?,
                    self.fmt_entry(*j)?
                )
            }
            &ConstantPoolEntry::NameAndTypeDescriptor(i, j) => {
                format!(
                    "NameAndTypeDescriptor: {}, {} => ({}), ({})",
                    i,
                    j,
                    self.fmt_entry(i)?,
                    self.fmt_entry(j)?
                )
            }
            &ConstantPoolEntry::MethodHandler(i, j) => format!(
                "MethodHandler: {}, {} => ({}), ({})",
                i,
                j,
                MethodHandlerKind::new(i)?,
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
                .fmt_entry(index)
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
            *cp.get(1).unwrap()
        );
        assert_eq!(ConstantPoolEntry::Integer(1), *cp.get(2).unwrap());
        assert_eq!(ConstantPoolEntry::Float(2.1), *cp.get(3).unwrap());
        assert_eq!(ConstantPoolEntry::Long(123i64), *cp.get(4).unwrap());
        matches!(
            cp.get(5),
            Err(ClassFileError::InvalidConstantPoolIndexError(5)),
        );
        assert_eq!(ConstantPoolEntry::Double(3.56), *cp.get(6).unwrap());
        assert_eq!(
            Err(ClassFileError::InvalidConstantPoolIndexError(7)),
            cp.get(7)
        );
        assert_eq!(ConstantPoolEntry::ClassReference(1), *cp.get(8).unwrap());
        assert_eq!(ConstantPoolEntry::StringReference(1), *cp.get(9).unwrap());
        assert_eq!(
            ConstantPoolEntry::Utf8("joe".to_string()),
            *cp.get(10).unwrap()
        );
        assert_eq!(
            ConstantPoolEntry::FieldReference(1, 10),
            *cp.get(11).unwrap()
        );
        assert_eq!(
            ConstantPoolEntry::MethodReference(1, 10),
            *cp.get(12).unwrap()
        );
        assert_eq!(
            ConstantPoolEntry::InterfaceMethodReference(1, 10),
            *cp.get(13).unwrap()
        );
        assert_eq!(
            ConstantPoolEntry::NameAndTypeDescriptor(1, 10),
            *cp.get(14).unwrap()
        );
    }
}
