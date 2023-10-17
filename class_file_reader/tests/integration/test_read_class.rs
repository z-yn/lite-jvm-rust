use class_file_reader::class_file::ClassAccessFlags;
use class_file_reader::class_file_reader::read_buffer;
use class_file_reader::class_file_version::ClassFileVersion;

#[test]
fn test_read_class() {
    let class = read_buffer(include_bytes!("../resources/HelloWorld.class")).unwrap();
    assert_eq!(
        format!("{:b}", ClassAccessFlags::PUBLIC | ClassAccessFlags::SUPER),
        format!("{:b}", class.access_flags.bits())
    );
    assert_eq!(class.version, ClassFileVersion::Jdk21);
    assert_eq!(class.this_class_name, "HelloWorld");
    assert_eq!(class.super_class_name.unwrap(), "java/lang/Object");
    assert_eq!(class.method_info.len(), 2);
}
