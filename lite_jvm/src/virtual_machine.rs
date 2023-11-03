use crate::call_stack::CallStack;
use crate::class_finder::ClassPath;
use crate::java_exception::{InvokeMethodResult, MethodCallError};
use crate::jvm_error::VmError;
use crate::loaded_class::{Class, ClassRef, ClassStatus, MethodRef};
use crate::method_area::MethodArea;
use crate::native_method_area::NativeMethodArea;
use crate::object_heap::ObjectHeap;
use crate::reference_value::{
    ArrayElement, ArrayReference, ObjectReference, PrimaryType, ReferenceValue, Value,
};
use crate::runtime_attribute_info::ConstantValueAttribute;
use crate::static_field_area::StaticArea;
use typed_arena::Arena;

/// 虚拟机实现。 虚拟机应该是总入口
///
/// Java虚拟机通过使用引导类加载器(BootstrapClassLoader)或者自定义类加载器，
/// 创建初始类或接口来启动，执行main方法。
///
/// 初始类或接口的其他选择是可能的，只要它们与上一段给出的规范一致。
/// - 初始类可以由命令行指定。
/// - Java虚拟机的实现本身可以提供一个初始类，该类设置一个类装入器，然后装入应用程序。
///
/// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-5.html#jvms-5.3
///
/// 类加载产生
/// - 初始化加载类
/// - 类或接口的创建由另一个类触发 =>依赖类加载器进行加载到方法区
/// - 数组类没有外部二进制表示;
/// - 类或接口的创建也可以由某些Java SE平台类库中的调用方法触发，如反射
/// - 类加载器可以直接加载也可以委托其他类加载器加载
///
/// 表示：
/// <N, Ld> =>类N由Ld直接加载  =>称之为 Ld defines N
///  N ^Li  =>类N由Li初始化加载(直接或者间接） =>L initiates loading of C
///
/// 类或接口的创建由另一个类触发时 加载规则:
///  N=>类名，D=>N指示的类，C=>通过D引用创建C
///  1. 如果N指示为非数组类或接口，D是由引导类加载器加载的 => C也由引导类加载器加载
///  2. 如果N指示为非数组类或接口，D由自定义类加载器加载的 => C由自定义类加载器加载
///  3. 如果N指示为数组类, JVM负责创建一个数组类C，然后将其标记为由D的类加载器加载的
///
/// 类加载后。类是由类名+类加载器共同标识的。
/// 每个这样的类或接口都属于单个运行时包。类或接口的运行时包由包名和类或接口的定义加载器决定。   
///

pub struct VirtualMachine<'a> {
    method_area: MethodArea<'a>,
    object_heap: ObjectHeap<'a>,
    call_stacks: Arena<CallStack<'a>>,
    static_area: StaticArea<'a>,
    native_method_area: NativeMethodArea<'a>,
}

impl<'a> VirtualMachine<'a> {
    pub fn new(heap_size: usize) -> VirtualMachine<'a> {
        VirtualMachine {
            method_area: MethodArea::new(),
            object_heap: ObjectHeap::new(heap_size),
            call_stacks: Arena::new(),
            static_area: StaticArea::new(),
            native_method_area: NativeMethodArea::new_with_default_native(),
        }
    }

    pub fn add_class_path(&mut self, class_path: Box<dyn ClassPath>) {
        self.method_area.add_class_path(class_path);
    }

    pub fn new_java_lang_class_object(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
    ) -> Result<ObjectReference<'a>, MethodCallError<'a>> {
        let class_ref = self.get_class_by_name(call_stack, class_name)?;
        let class_object = self.new_object_by_class_name(call_stack, "java/lang/Class")?;
        let string_object = self.new_java_lang_string_object(call_stack, &class_ref.name)?;
        class_object.set_field_by_name("name", &Value::ObjectRef(string_object))?;
        Ok(class_object)
    }

    pub fn new_java_lang_string_object(
        &mut self,
        call_stack: &mut CallStack<'a>,
        value: &str,
    ) -> Result<ObjectReference<'a>, MethodCallError<'a>> {
        let char_array: Vec<Value<'a>> =
            value.encode_utf16().map(|c| Value::Int(c as i32)).collect();
        let array_ref = self.new_array(
            ArrayElement::PrimaryValue(PrimaryType::Char),
            char_array.len(),
        );
        char_array
            .into_iter()
            .enumerate()
            .for_each(|(index, value)| array_ref.set_field_by_offset(index, &value).unwrap());

        let object = self.new_object_by_class_name(call_stack, "java/lang/String")?;
        object.set_field_by_name("value", &Value::ArrayRef(array_ref))?;
        object.set_field_by_name("hash", &Value::Int(0))?;
        Ok(object)
    }

    fn init_static_fields(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_ref: &mut Class<'a>,
        ro_class_ref: ClassRef<'a>,
    ) -> Result<(), MethodCallError<'a>> {
        for (field_name, field) in &class_ref.fields {
            if field.is_static() {
                if let Some(v) = &field.constant_value {
                    let value = match v {
                        ConstantValueAttribute::Int(i) => Value::Int(*i),
                        ConstantValueAttribute::Float(f) => Value::Float(*f),
                        ConstantValueAttribute::Long(l) => Value::Long(*l),
                        ConstantValueAttribute::Double(d) => Value::Double(*d),
                        ConstantValueAttribute::String(str) => Value::ObjectRef(
                            self.new_java_lang_string_object(call_stack, str).unwrap(),
                        ),
                    };
                    self.static_area
                        .set_static_field(ro_class_ref, field_name, value)
                }
            };
            //TODO 动态初始化实现
        }
        Ok(())
    }

    fn link_class(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_ref: ClassRef<'a>,
    ) -> Result<(), MethodCallError<'a>> {
        if let ClassStatus::Loaded = class_ref.status {
            if let Some(mut_class_ref) = self.method_area.get_mut(class_ref) {
                self.init_static_fields(call_stack, mut_class_ref, class_ref)?;
                mut_class_ref.status = ClassStatus::Linked;
            }
        }
        Ok(())
    }
    //类的初始化。需要执行<clinit>方法。初始化一些变量。需要先实现方法执行
    fn initialize_class(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_ref: ClassRef<'a>,
    ) -> Result<(), MethodCallError<'a>> {
        if let ClassStatus::Linked = class_ref.status {
            if let Ok(method_ref) = class_ref.get_method("<clinit>", "()V") {
                self.invoke_method(call_stack, class_ref, method_ref, None, Vec::new())?;
            }
            if let Some(mut_class_ref) = self.method_area.get_mut(class_ref) {
                mut_class_ref.status = ClassStatus::Initialized;
            }
        }
        Ok(())
    }
    pub fn lookup_class_and_initialize(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
    ) -> Result<ClassRef<'a>, MethodCallError<'a>> {
        let class = self.method_area.load_class(class_name)?;
        self.link_class(call_stack, class)?;
        self.initialize_class(call_stack, class)?;
        Ok(class)
    }
    pub fn look_method(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> Result<(ClassRef, MethodRef), MethodCallError<'a>> {
        let class_ref = self.lookup_class_and_initialize(call_stack, class_name)?;
        let method_ref = class_ref.get_method_by_checking_super(method_name, descriptor)?;
        Ok((class_ref, method_ref))
    }

    pub fn new_object(&mut self, class_ref: ClassRef) -> ObjectReference<'a> {
        self.object_heap.allocate_object(class_ref).unwrap()
    }

    pub fn new_object_by_class_name(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
    ) -> Result<ObjectReference<'a>, MethodCallError<'a>> {
        let class_ref = self.lookup_class_and_initialize(call_stack, class_name)?;
        Ok(self.new_object(class_ref))
    }

    pub fn new_array(&mut self, array_element: ArrayElement, length: usize) -> ArrayReference<'a> {
        self.object_heap
            .allocate_array(array_element, length)
            .unwrap()
    }

    pub fn get_static(&self, class_ref: ClassRef<'a>, field_name: &str) -> Option<&Value<'a>> {
        self.static_area.get_static_field(class_ref, field_name)
    }
    pub fn get_class_by_name(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
    ) -> Result<ClassRef<'a>, MethodCallError<'a>> {
        //防止重复加载
        let class_ref = if !self.method_area.is_class_loaded(class_name) {
            self.lookup_class_and_initialize(call_stack, class_name)?
        } else {
            self.method_area.load_class(class_name)?
        };
        Ok(class_ref)
    }
    pub fn get_static_field_by_class_name(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
        field_name: &str,
    ) -> Result<Option<&Value<'a>>, MethodCallError<'a>> {
        //防止重复加载
        let class_ref = self.get_class_by_name(call_stack, class_name)?;
        let value = self.static_area.get_static_field(class_ref, field_name);
        Ok(value)
    }

    pub fn set_static_field_by_class_name(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
        field_name: &str,
        value: Value<'a>,
    ) -> Result<(), MethodCallError<'a>> {
        let class_ref = self.get_class_by_name(call_stack, class_name)?;
        self.static_area
            .set_static_field(class_ref, field_name, value);
        Ok(())
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
        self.native_method_area
            .get_method(&class_ref.name, &method_ref.name, &method_ref.descriptor)
            .unwrap()(self, call_stack, object, args)
    }

    pub fn invoke_method(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_ref: ClassRef<'a>,
        method_ref: MethodRef<'a>,
        object: Option<ObjectReference<'a>>,
        args: Vec<Value<'a>>,
    ) -> InvokeMethodResult<'a> {
        if method_ref.is_native() {
            return self.invoke_native_method(call_stack, class_ref, method_ref, object, args);
        }
        let mut frame = call_stack.new_frame(class_ref, method_ref, object, args)?;
        let result = frame.as_mut().execute(self, call_stack)?;
        call_stack.pop_frame();
        Ok(result)
    }

    pub fn allocate_call_stack(&mut self) -> &'a mut CallStack<'a> {
        let stack = self.call_stacks.alloc(CallStack::new());
        unsafe {
            let stack_ptr: *mut CallStack<'a> = stack;
            &mut *stack_ptr
        }
    }
}

mod tests {

    #[test]
    fn test_exec() {
        use crate::class_finder::{FileSystemClassPath, JarFileClassPath};
        use crate::loaded_class::ClassStatus;
        use crate::reference_value::Value;
        use crate::virtual_machine::VirtualMachine;
        let mut vm = VirtualMachine::new(102400);
        let file_system_path = FileSystemClassPath::new("./resources").unwrap();
        vm.add_class_path(Box::new(file_system_path));
        let rt_jar_path = JarFileClassPath::new("./resources/rt.jar").unwrap();
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
}
