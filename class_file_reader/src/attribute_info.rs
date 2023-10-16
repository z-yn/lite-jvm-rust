use std::fmt::Display;

/// ## 属性信息
/// 属性可以出现在，字段、方法，类中，是重要的扩展机制
/// [jvms-4.7](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7)
#[derive(Debug, Default, PartialEq)]
pub struct AttributeInfo {
    pub name: String,
    pub info: Vec<u8>,
}

pub trait Attribute: Display {}
///https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7.2
pub enum ConstantValue {
    Int(String, i32),
    Float(String, f32),
    Long(String, i64),
    Double(String, f64),
    String(String, String),
}

impl Attribute for ConstantValue {}
