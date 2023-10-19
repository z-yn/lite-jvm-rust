use crate::runtime_constant_pool::RuntimeConstantPool;
use class_file_reader::attribute_info::AttributeInfo;
use class_file_reader::class_file::ClassAccessFlags;
use class_file_reader::field_info::FieldInfo;
use class_file_reader::method_info::MethodInfo;

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
    pub fields: Vec<FieldInfo>,
    //方法解析
    pub methods: Vec<MethodInfo>,
    //属性解析
    pub attributes: Vec<AttributeInfo>,

    pub super_class_name: Option<String>,
    pub interface_names: Vec<String>,
}

impl<'a> Class<'a> {}

pub type ClassRef<'a> = &'a Class<'a>;
