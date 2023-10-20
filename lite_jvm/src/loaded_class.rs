use crate::jvm_exceptions::{Exception, Result};
use crate::runtime_constant_pool::RuntimeConstantPool;
use crate::runtime_field_info::RuntimeFieldInfo;
use crate::runtime_method_info::RuntimeMethodInfo;
use class_file_reader::class_file::ClassAccessFlags;

pub enum ClassStatus {
    Loaded,
    Linked,
    Initialized,
}

/// 表示加载的类，加载后该类会经过->链接->初始化过程最终加载完成。
///
pub struct Class<'a> {
    pub status: ClassStatus,
    pub name: String,
    //常量池解析
    pub constant_pool: RuntimeConstantPool,
    pub access_flags: ClassAccessFlags,
    //超类解析
    pub super_class: Option<ClassRef<'a>>,
    //接口解析
    pub interfaces: Vec<ClassRef<'a>>,
    // 先用数组存。后续再看是否需要改成map，以及是否需要改变结构
    //字段解析
    pub fields: Vec<RuntimeFieldInfo>,
    //方法解析
    pub methods: Vec<RuntimeMethodInfo>,

    pub super_class_name: Option<String>,
    pub interface_names: Vec<String>,
}

impl<'a> Class<'a> {
    pub(crate) fn get_method_info(
        &self,
        method_name: &str,
        descriptor: &str,
    ) -> Result<MethodRef<'a>> {
        for method in &self.methods {
            if method.name == method_name && method.descriptor == descriptor {
                //self的声明周期要大于classRef<'a>,实用unsafe 使得编译器能够编译
                let method_ref = unsafe {
                    let const_ptr: *const RuntimeMethodInfo = method;
                    &*const_ptr
                };
                return Ok(method_ref);
            }
        }
        //查找父类
        if let Some(supper_class) = self.super_class {
            for method in &supper_class.methods {
                if method.name == method_name && method.descriptor == descriptor {
                    return Ok(method);
                }
            }
        }
        //查找接口
        for interface in &self.interfaces {
            for method in &interface.methods {
                if method.name == method_name && method.descriptor == descriptor {
                    return Ok(method);
                }
            }
        }
        Err(Exception::MethodNotFoundException(method_name.to_string()))
    }
}

pub type ClassRef<'a> = &'a Class<'a>;

pub type MethodRef<'a> = &'a RuntimeMethodInfo;
