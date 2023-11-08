use crate::class_finder::ClassPath;
use crate::java_exception::{InvokeMethodResult, MethodCallError};
use crate::jvm_values::{
    ArrayElement, ArrayReference, ObjectReference, PrimaryType, ReferenceValue, Value,
};
use crate::loaded_class::{ClassRef, ClassStatus, MethodRef};
use crate::method_area::MethodArea;
use crate::native_method_area::NativeMethodArea;
use crate::object_heap::ObjectHeap;
use crate::runtime_attribute_info::ConstantValueAttribute;
use crate::stack::CallStack;
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
    vm_stacks: Arena<CallStack<'a>>,
    static_area: StaticArea<'a>,
    native_method_area: NativeMethodArea<'a>,
}

impl<'a> VirtualMachine<'a> {
    pub fn new(heap_size: usize) -> VirtualMachine<'a> {
        VirtualMachine {
            method_area: MethodArea::default(),
            object_heap: ObjectHeap::new(heap_size),
            vm_stacks: Arena::new(),
            static_area: StaticArea::new(1024 * 1024),
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
        if let Some(v) = self.static_area.class_constant_pool.get(class_name) {
            Ok(*v)
        } else {
            // self.get_class_by_name(call_stack, class_name)?;
            let class_ref = self.get_class_by_name(call_stack, "java/lang/Class")?;
            let class_object = self.static_area.new_object(class_ref);
            let string_object = self.new_java_lang_string_object(call_stack, class_name)?;
            class_object.set_field_by_name("name", &Value::ObjectRef(string_object))?;
            Ok(class_object)
        }
    }

    pub fn new_java_lang_string_object(
        &mut self,
        call_stack: &mut CallStack<'a>,
        value: &str,
    ) -> Result<ObjectReference<'a>, MethodCallError<'a>> {
        if let Some(v) = self.static_area.string_constant_pool.get(value) {
            Ok(*v)
        } else {
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
            let string_class_ref =
                self.lookup_class_and_initialize(call_stack, "java/lang/String")?;
            let object = self.static_area.new_object(string_class_ref);
            object.set_field_by_name("value", &Value::ArrayRef(array_ref))?;
            object.set_field_by_name("hash", &Value::Int(0))?;
            self.static_area
                .string_constant_pool
                .insert(value.to_string(), object);
            Ok(object)
        }
    }

    fn init_static_fields(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_ref: ClassRef<'a>,
    ) -> Result<(), MethodCallError<'a>> {
        for (field_name, field) in &class_ref.fields {
            if field.is_static() {
                let value = if let Some(v) = &field.constant_value {
                    match v {
                        ConstantValueAttribute::Int(i) => Value::Int(*i),
                        ConstantValueAttribute::Float(f) => Value::Float(*f),
                        ConstantValueAttribute::Long(l) => Value::Long(*l),
                        ConstantValueAttribute::Double(d) => Value::Double(*d),
                        ConstantValueAttribute::String(str) => Value::ObjectRef(
                            self.new_java_lang_string_object(call_stack, str).unwrap(),
                        ),
                    }
                } else {
                    match field.descriptor.as_str() {
                        "B" | "C" | "I" | "S" | "Z" => Value::Int(0),
                        "F" => Value::Float(0f32),
                        "D" => Value::Double(0f64),
                        "J" => Value::Long(0),
                        _ => Value::Null,
                    }
                };

                self.static_area
                    .set_static_field(class_ref, field_name, value)
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
        if class_ref.status == ClassStatus::Loaded {
            self.set_class_stage(class_ref, ClassStatus::Linking);
            self.init_static_fields(call_stack, class_ref)?;
            self.set_class_stage(class_ref, ClassStatus::Linked);
        }
        Ok(())
    }
    fn set_class_stage(&mut self, class_ref: ClassRef<'a>, class_status: ClassStatus) {
        if let Some(mut_class_ref) = self.method_area.get_mut(class_ref) {
            mut_class_ref.status = class_status;
        }
    }
    //类的初始化。需要执行<clinit>方法。初始化一些变量。
    fn initialize_class(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_ref: ClassRef<'a>,
    ) -> Result<(), MethodCallError<'a>> {
        if let ClassStatus::Linked = class_ref.status {
            self.set_class_stage(class_ref, ClassStatus::Initializing);

            if let Ok(method_ref) = class_ref.get_method("<clinit>", "()V") {
                self.invoke_method(
                    call_stack,
                    class_ref,
                    method_ref,
                    None::<ObjectReference>,
                    Vec::new(),
                )?;
            }
            self.set_class_stage(class_ref, ClassStatus::Initialized);
        }
        Ok(())
    }
    pub fn lookup_class_and_initialize(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
    ) -> Result<ClassRef<'a>, MethodCallError<'a>> {
        let class_name = if class_name.starts_with('[') {
            "java/lang/Object"
        } else {
            class_name
        };
        let class = self.method_area.load_class(class_name)?;
        self.link_class(call_stack, class)?;
        self.initialize_class(call_stack, class)?;
        Ok(class)
    }
    pub fn lookup_method(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> Result<(ClassRef<'a>, MethodRef<'a>), MethodCallError<'a>> {
        let class_ref = self.lookup_class_and_initialize(call_stack, class_name)?;
        let method_ref = class_ref.get_method_by_checking_super(method_name, descriptor)?;
        Ok(method_ref)
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
        object: Option<impl ReferenceValue<'a>>,
        args: Vec<Value<'a>>,
    ) -> InvokeMethodResult<'a> {
        let depth = "\t".repeat(call_stack.depth() - 1);
        println!(
            "{}=> invoke_native_method {}:{}{}",
            depth, class_ref.name, method_ref.name, method_ref.descriptor
        );
        let native_method = self.native_method_area.get_method(
            &class_ref.name,
            &method_ref.name,
            &method_ref.descriptor,
        );
        native_method.unwrap()(self, call_stack, object.map(|e| e.as_value()), args)
    }

    pub fn invoke_method(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_ref: ClassRef<'a>,
        method_ref: MethodRef<'a>,
        object: Option<impl ReferenceValue<'a>>,
        args: Vec<Value<'a>>,
    ) -> InvokeMethodResult<'a> {
        if method_ref.is_native() {
            return self.invoke_native_method(call_stack, class_ref, method_ref, object, args);
        }
        let mut frame = call_stack.new_frame(class_ref, method_ref, object, args)?;
        let result = frame.as_mut().execute(self, call_stack);
        call_stack.pop_frame();
        result
    }

    pub fn allocate_call_stack(&mut self) -> &'a mut CallStack<'a> {
        let stack = self.vm_stacks.alloc(CallStack::new());
        unsafe {
            let stack_ptr: *mut CallStack<'a> = stack;
            &mut *stack_ptr
        }
    }
}

mod tests {
    use crate::jvm_values::ObjectReference;

    #[test]
    fn test_hello() {
        use crate::class_finder::{FileSystemClassPath, JarFileClassPath};
        use crate::loaded_class::ClassStatus;
        use crate::virtual_machine::VirtualMachine;
        let mut vm = VirtualMachine::new(102400);
        let file_system_path = FileSystemClassPath::new("./resources").unwrap();
        vm.add_class_path(Box::new(file_system_path));
        let rt_jar_path = JarFileClassPath::new("./resources/rt.jar").unwrap();
        let call_stack = vm.allocate_call_stack();
        vm.add_class_path(Box::new(rt_jar_path));
        let class_ref = vm
            .lookup_class_and_initialize(call_stack, "HelloWorld")
            .unwrap();
        assert_eq!(class_ref.status, ClassStatus::Initialized);
        // 实现System.out.println有点复杂。

        // let method_ref = class_ref
        //     .get_method("main", "([Ljava/lang/String;)V")
        //     .unwrap();
        // vm.invoke_method(call_stack, class_ref, method_ref, None, Vec::new())
        //     .unwrap();
    }

    #[test]
    fn test_field_value() {
        use crate::class_finder::{FileSystemClassPath, JarFileClassPath};
        use crate::jvm_values::ReferenceValue;
        use crate::jvm_values::Value;
        use crate::loaded_class::ClassStatus;
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
        assert_eq!(class_ref.status, ClassStatus::Initialized);
        //测试初始化数据
        //由ConstantValue设置的初始值
        let name = vm.get_static(class_ref, "NAME").unwrap();
        assert_eq!(name.get_string().unwrap(), "static");
        //由<clinit>的初始值
        let an_int = vm
            .get_static(class_ref, "anInt")
            .unwrap()
            .get_int()
            .unwrap();
        assert_eq!(2, an_int);
        //初始化对象
        let object_ref = vm.new_object(class_ref);
        let a = object_ref.get_field_by_name("a").unwrap();
        assert_eq!(a.get_int().unwrap(), 0i32);
        let b = object_ref.get_field_by_name("b").unwrap();
        assert_eq!(b, Value::Null);

        let field_double = object_ref.get_field_by_name("fieldDouble").unwrap();
        assert_eq!(field_double.get_double().unwrap(), 0f64);

        //调用初始化方法
        let init_method = class_ref.get_method("<init>", "()V").unwrap();
        vm.invoke_method(
            call_stack,
            class_ref,
            init_method,
            Some(object_ref),
            Vec::new(),
        )
        .unwrap();
        let field_double = object_ref.get_field_by_name("fieldDouble").unwrap();
        //初始化后fieldDouble应该是100
        assert_eq!(field_double.get_double().unwrap(), 100f64);
        let field_float = object_ref.get_field_by_name("fieldFloat").unwrap();
        //初始化后fieldFloat应该是50
        assert_eq!(field_float.get_float().unwrap(), 50f32);

        //测试方法调用
        let main_method = class_ref.get_method("increaseInt", "()V").unwrap();
        vm.invoke_method(
            call_stack,
            class_ref,
            main_method,
            None::<ObjectReference>,
            Vec::new(),
        )
        .unwrap();
        let an_int = vm.get_static(class_ref, "anInt");
        assert!(matches!(an_int, Some(Value::Int(3))));
    }

    #[test]
    fn test_exception() {
        use crate::class_finder::{FileSystemClassPath, JarFileClassPath};
        use crate::java_exception::MethodCallError;
        use crate::jvm_values::Value;
        use crate::loaded_class::ClassStatus;
        use crate::virtual_machine::VirtualMachine;
        let mut vm = VirtualMachine::new(102400);
        let file_system_path = FileSystemClassPath::new("./resources").unwrap();
        vm.add_class_path(Box::new(file_system_path));
        let rt_jar_path = JarFileClassPath::new("./resources/rt.jar").unwrap();
        let call_stack = vm.allocate_call_stack();
        vm.add_class_path(Box::new(rt_jar_path));
        let class_ref = vm
            .lookup_class_and_initialize(call_stack, "ExceptionTest")
            .unwrap();
        assert_eq!(class_ref.status, ClassStatus::Initialized);
        let obj_ref = vm.new_object(class_ref);

        //测试异常try-catch
        let method_recovery = class_ref.get_method("methodRecovery", "()I").unwrap();
        let result = vm
            .invoke_method(
                call_stack,
                class_ref,
                method_recovery,
                Some(obj_ref),
                Vec::new(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(result, Value::Int(2));

        //测试抛出异常
        let throw_null_pointer_exception = class_ref
            .get_method("throwNullPointException", "()I")
            .unwrap();
        let result = vm.invoke_method(
            call_stack,
            class_ref,
            throw_null_pointer_exception,
            Some(obj_ref),
            Vec::new(),
        );
        if let Err(MethodCallError::ExceptionThrown(exp)) = result {
            let x = exp.get_class();
            assert_eq!(x.name, "java/lang/NullPointerException")
        }

        //测试异常堆栈信息
        let throw_null_pointer_exception = class_ref
            .get_method("methodStackTrace", "()[Ljava/lang/StackTraceElement;")
            .unwrap();
        let result = vm
            .invoke_method(
                call_stack,
                class_ref,
                throw_null_pointer_exception,
                Some(obj_ref),
                Vec::new(),
            )
            .unwrap();
    }
}
