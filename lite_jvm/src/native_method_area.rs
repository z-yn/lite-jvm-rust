use crate::java_exception::{InvokeMethodResult, MethodCallError};
use crate::jvm_error::VmError;
use crate::jvm_values::{ObjectReference, ReferenceValue, Value};
use crate::virtual_machine::VirtualMachine;
use crate::virtual_machine_stack::VirtualMachineStack;
use class_file_reader::class_file_version::ClassFileVersion;
use std::collections::HashMap;

pub type NativeMethod<'a> = fn(
    &mut VirtualMachine<'a>,
    &mut VirtualMachineStack<'a>,
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
        area.registry_native_method(
            "java/lang/System",
            "arraycopy",
            "(Ljava/lang/Object;ILjava/lang/Object;II)V",
            Self::java_lang_system_arraycopy,
        );

        area.registry_native_method("java/lang/Object", "registerNatives", "()V", Self::nop);
        area.registry_native_method("java/lang/Class", "registerNatives", "()V", Self::nop);
        area
    }
    pub fn nop(
        _vm: &mut VirtualMachine<'a>,
        _call_stack: &mut VirtualMachineStack<'a>,
        _receiver: Option<ObjectReference<'a>>,
        _args: Vec<Value<'a>>,
    ) -> InvokeMethodResult<'a> {
        Ok(None)
    }

    pub fn java_lang_system_arraycopy(
        _vm: &mut VirtualMachine<'a>,
        _call_stack: &mut VirtualMachineStack<'a>,
        _receiver: Option<ObjectReference<'a>>,
        mut args: Vec<Value<'a>>,
    ) -> InvokeMethodResult<'a> {
        assert_eq!(args.len(), 5);
        let from_array = args[0].get_array()?;
        let from_start = args[1].get_int()?;

        let to_array = args[2].get_array()?;
        let to_start = args[3].get_int()?;
        let length = args[4].get_int()?;
        for offset in 0..length {
            let value = from_array.get_field_by_offset((offset + from_start) as usize)?;
            to_array.set_field_by_offset((offset + to_start) as usize, &value)?;
        }
        Ok(None)
    }
    pub fn java_lang_system_register_native(
        vm: &mut VirtualMachine<'a>,
        call_stack: &mut VirtualMachineStack<'a>,
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
