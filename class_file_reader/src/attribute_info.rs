use crate::attribute_info::AttributeType::CustomType;
use crate::class_file_error::{ClassFileError, Result};
use std::fmt::{Display, Formatter};

/// ## 属性信息
/// 属性可以出现在，字段、方法，类中，是重要的扩展机制
/// [jvms-4.7](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7)
#[derive(Debug, PartialEq)]
pub struct AttributeInfo {
    pub name: AttributeType,
    pub info: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum AttributeType {
    ConstantValue,
    Code,
    StackMapTable,
    BootstrapMethods,
    NestHost,
    NestMembers,
    PermittedSubclasses,

    Exceptions,
    InnerClasses,
    EnclosingMethod,
    Synthetic,
    Signature,
    Record,
    SourceFile,
    LineNumberTable,
    LocalVariableTable,
    LocalVariableTypeTable,

    SourceDebugExtension,
    Deprecated,
    RuntimeVisibleAnnotations,
    RuntimeInvisibleAnnotations,
    RuntimeVisibleParameterAnnotations,
    RuntimeInvisibleParameterAnnotations,
    RuntimeVisibleTypeAnnotations,
    RuntimeInvisibleTypeAnnotations,
    AnnotationDefault,
    MethodParameters,
    Module,
    ModulePackages,
    ModuleMainClass,
    CustomType(String),
}

impl AttributeType {
    /// 解析ConstantValue
    pub fn to_constant_value(&self) -> Result<ConstantValue> {
        if let AttributeType::ConstantValue = self {}
        todo!()
    }
    pub fn by_name(str: &str) -> AttributeType {
        match str {
            "ConstantValue" => AttributeType::ConstantValue,
            "Code" => AttributeType::Code,
            "StackMapTable" => AttributeType::StackMapTable,
            "BootstrapMethods" => AttributeType::BootstrapMethods,
            "NestHost" => AttributeType::NestHost,
            "NestMembers" => AttributeType::NestMembers,
            "PermittedSubclasses" => AttributeType::PermittedSubclasses,
            "Exceptions" => AttributeType::Exceptions,
            "InnerClasses" => AttributeType::InnerClasses,
            "EnclosingMethod" => AttributeType::EnclosingMethod,
            "Synthetic" => AttributeType::Synthetic,
            "Signature" => AttributeType::Signature,
            "Record" => AttributeType::Record,
            "SourceFile" => AttributeType::SourceFile,
            "LineNumberTable" => AttributeType::LineNumberTable,
            "LocalVariableTable" => AttributeType::LocalVariableTable,
            "LocalVariableTypeTable" => AttributeType::LocalVariableTypeTable,
            "SourceDebugExtension" => AttributeType::SourceDebugExtension,
            "Deprecated" => AttributeType::Deprecated,
            "RuntimeVisibleAnnotations" => AttributeType::RuntimeVisibleAnnotations,
            "RuntimeInvisibleAnnotations" => AttributeType::RuntimeInvisibleAnnotations,
            "RuntimeVisibleParameterAnnotations" => {
                AttributeType::RuntimeVisibleParameterAnnotations
            }
            "RuntimeInvisibleParameterAnnotations" => {
                AttributeType::RuntimeInvisibleParameterAnnotations
            }
            "RuntimeVisibleTypeAnnotations" => AttributeType::RuntimeVisibleTypeAnnotations,
            "RuntimeInvisibleTypeAnnotations" => AttributeType::RuntimeInvisibleTypeAnnotations,
            "AnnotationDefault" => AttributeType::AnnotationDefault,
            "MethodParameters" => AttributeType::MethodParameters,
            "Module" => AttributeType::Module,
            "ModulePackages" => AttributeType::ModulePackages,
            "ModuleMainClass" => AttributeType::ModuleMainClass,
            t => CustomType(String::from(t)),
        }
    }
}
pub trait Attribute: Display + Sized {}
///https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7.2
#[allow(dead_code)]
pub enum ConstantValue {
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    String(String),
}

impl Display for ConstantValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstantValue::Int(v) => write!(f, "ConstantValue=>Int:{v}"),
            ConstantValue::Float(v) => write!(f, "ConstantValue=>Float:{v}"),
            ConstantValue::Long(v) => write!(f, "ConstantValue=>Long:{v}"),
            ConstantValue::Double(v) => write!(f, "ConstantValue=>Double:{v}"),
            ConstantValue::String(v) => write!(f, "ConstantValue=>String:{v}"),
        }
    }
}

impl Attribute for ConstantValue {}
