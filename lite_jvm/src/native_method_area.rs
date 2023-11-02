use crate::call_stack::CallStack;
use crate::java_exception::{InvokeMethodResult, MethodCallError};
use crate::jvm_error::VmError;
use crate::loaded_class::{ClassRef, MethodRef};
use crate::reference_value::{ObjectReference, Value};
use crate::runtime_method_info::MethodDescriptor;
use crate::virtual_machine::VirtualMachine;
use class_file_reader::class_file_version::ClassFileVersion;
use std::collections::HashMap;

pub type NativeMethod<'a> = fn(
    &mut VirtualMachine<'a>,
    &mut CallStack<'a>,
    Option<ObjectReference<'a>>,
    Vec<Value<'a>>,
) -> InvokeMethodResult<'a>;

pub struct NativeMethodArea<'a> {
    native_methods: HashMap<String, NativeMethod<'a>>,
}

impl<'a> NativeMethodArea<'a> {
    pub fn new_with_default_native() -> NativeMethodArea<'a> {
        let mut area = NativeMethodArea {
            native_methods: HashMap::new(),
        };
        area.registry_native_method(
            "java/lang/System",
            "registerNatives",
            "()V",
            Self::java_lang_system_register_native,
        );
        area
    }
    pub fn java_lang_system_register_native(
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut CallStack<'a>,
        _receiver: Option<ObjectReference<'a>>,
        _args: Vec<Value<'a>>,
    ) -> InvokeMethodResult<'a> {
        let class_ref = vm.get_class_by_name(call_stack, "java/lang/System")?;
        let method_ref = match class_ref.version {
            ClassFileVersion::Jdk8 => class_ref.get_method("initializeSystemClass", "()V")?,
            ClassFileVersion::Jdk17 => class_ref.get_method("initPhase1", "()V")?,
            _ => {
                return Err(MethodCallError::InternalError(
                    VmError::ClassVersionNotSupport,
                ))
            }
        };
        vm.invoke_method(call_stack, class_ref, method_ref, None, Vec::new())
    }

    pub fn registry_native_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        method_descriptor: &str,
        method: NativeMethod<'a>,
    ) {
        let key = format!("{}:{}{}", class_name, method_name, method_descriptor);
        self.native_methods.insert(key, method);
    }
    pub fn invoke_native_method(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_ref: ClassRef<'a>,
        method_ref: MethodRef<'a>,
        object: Option<ObjectReference<'a>>,
        args: Vec<Value<'a>>,
    ) -> InvokeMethodResult<'a> {
        let depth = "\t".repeat(call_stack.depth() - 1);
        println!(
            "{}=> invoke_native_method {}:{}{}",
            depth, class_ref.name, method_ref.name, method_ref.descriptor
        );

        Ok(None)
    }
    pub fn get_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        method_descriptor: &str,
    ) -> Option<&NativeMethod<'a>> {
        let key = format!("{}:{}{}", class_name, method_name, method_descriptor);
        self.native_methods.get(&key)
    }
}
