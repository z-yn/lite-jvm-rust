use class_file_reader::class_file::ClassFile;
use log::info;

pub fn read_class_from_bytes(bytes: &[u8]) -> ClassFile {
    let class = class_file_reader::class_file_reader::read_buffer(bytes).unwrap();
    info!("read class file: {}", class);
    class
}
