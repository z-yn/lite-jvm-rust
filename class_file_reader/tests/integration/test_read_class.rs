use class_file_reader::class_file_reader::read_buffer;

#[test]
fn test_read_class() {
    let class = read_buffer(include_bytes!("../resources/ExceptionsHandlers.class")).unwrap();
    println!("class")
}
