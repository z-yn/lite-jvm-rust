use lite_jvm::class_finder::{FileSystemClassPath, JarFileClassPath};
use lite_jvm::loaded_class::ClassStatus;
use lite_jvm::reference_value::Value;
use lite_jvm::virtual_machine::VirtualMachine;

fn main() {
    let mut vm = VirtualMachine::new(102400);
    let file_system_path = FileSystemClassPath::new(
        "/Users/zouyanan/Workspace/github/lite-jvm-rust/lite_jvm/resources",
    )
    .unwrap();
    vm.add_class_path(Box::new(file_system_path));
    let rt_jar_path = JarFileClassPath::new(
        "/Users/zouyanan/Workspace/github/lite-jvm-rust/lite_jvm/resources/rt.jar",
    )
    .unwrap();
    let call_stack = vm.allocate_call_stack();
    vm.add_class_path(Box::new(rt_jar_path));
    let class_ref = vm
        .lookup_class_and_initialize(call_stack, "FieldTest")
        .unwrap();
    assert!(matches!(class_ref.status, ClassStatus::Initialized));
    let an_int = vm.get_static(class_ref, "anInt");
    assert!(matches!(an_int, Some(Value::Int(2))));
    let main_method = class_ref
        .get_method("main", "([Ljava/lang/String;)V")
        .unwrap();
    vm.invoke_method(call_stack, class_ref, main_method, None, Vec::new())
        .unwrap();
}
