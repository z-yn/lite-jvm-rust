use class_file_reader::class_file::ClassAccessFlags;
use class_file_reader::class_file_reader::read_buffer;
use class_file_reader::constant_pool::ConstantPool;

pub enum ClassStatus {
    Loaded,
    Linked,
    Initialized,
}

/// 表示加载的类，加载后该类会经过->链接->初始化过程最终加载完成。
///
pub(crate) struct LoadedClass<'a> {
    pub(crate) status: ClassStatus,
    pub(crate) name: String,
    pub(crate) constant_pool: ConstantPool,
    pub(crate) access_flags: ClassAccessFlags,
    pub(crate) super_class: Option<ClassRef<'a>>,
}

pub(crate) struct ClassesToInitialize<'a> {
    resolved_class: ClassRef<'a>,
    pub(crate) to_initialize: Vec<ClassRef<'a>>,
}

impl<'a> LoadedClass<'a> {}

//需要一个转换的通用方法。供所有class_load使用。
pub fn load_class<'a>(bytes: Vec<u8>) -> LoadedClass<'a> {
    let file = read_buffer(&bytes).unwrap();
    LoadedClass {
        status: ClassStatus::Loaded,
        name: file.this_class_name,
        constant_pool: file.constant_pool,
        access_flags: file.access_flags,
        super_class: None,
    }
}

pub type ClassRef<'a> = &'a LoadedClass<'a>;
