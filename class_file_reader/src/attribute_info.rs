use crate::attribute_info::AttributeType::CustomType;

/// ## 属性信息
/// 属性可以出现在，字段、方法，类中，是重要的扩展机制
/// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7
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
