use crate::call_stack::CallStack;
use crate::class_finder::ClassPath;
use crate::java_exception::{InvokeMethodResult, MethodCallError};
use crate::jvm_error::VmExecResult;
use crate::loaded_class::{ClassRef, ClassStatus, MethodRef};
use crate::method_area::MethodArea;
use crate::object_heap::ObjectHeap;
use crate::reference_value::{ArrayElement, ArrayReference, ObjectReference, Value};
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
}

impl<'a> VirtualMachine<'a> {
    pub fn new(heap_size: usize) -> VirtualMachine<'a> {
        VirtualMachine {
            method_area: MethodArea::new(),
            object_heap: ObjectHeap::new(heap_size),
            call_stacks: Arena::new(),
            static_area: StaticArea::new(),
        }
    }

    pub fn add_class_path(&mut self, class_path: Box<dyn ClassPath>) {
        self.method_area.add_class_path(class_path);
    }

    fn link_class(&mut self, class_ref: ClassRef<'a>) -> VmExecResult<()> {
        if let ClassStatus::Loaded = class_ref.status {
            if let Some(mut_class_ref) = self.method_area.get_mut(class_ref) {
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
    pub fn lookup_class(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
    ) -> Result<ClassRef<'a>, MethodCallError<'a>> {
        let class = self.method_area.load_class(class_name)?;
        assert_eq!(class.name, "FieldTest");
        self.link_class(class)?;
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
        let class_ref = self.lookup_class(call_stack, class_name)?;
        let method_ref = class_ref.get_method_by_checking_super(method_name, descriptor)?;
        Ok((class_ref, method_ref))
    }

    pub fn new_object(&mut self, class_ref: ClassRef) -> ObjectReference<'static> {
        self.object_heap.allocate_object(class_ref).unwrap()
    }

    pub fn new_array(
        &mut self,
        array_element: ArrayElement,
        length: usize,
    ) -> ArrayReference<'static> {
        self.object_heap
            .allocate_array(array_element, length)
            .unwrap()
    }

    pub fn get_static_field(
        &mut self,
        class_name: &str,
        field_name: &str,
    ) -> Result<Value<'a>, MethodCallError<'a>> {
        let class_ref = self.method_area.load_class(class_name)?;
        let value = self.static_area.get_static_field(class_ref, field_name);
        Ok(value)
    }

    pub fn set_static_field(
        &mut self,
        class_name: &str,
        field_name: &str,
        value: Value<'a>,
    ) -> Result<(), MethodCallError<'a>> {
        let class_ref = self.method_area.load_class(class_name)?;
        self.static_area
            .set_static_field(class_ref, field_name, value);
        Ok(())
    }

    pub fn invoke_native_method(
        &mut self,
        class_ref: ClassRef<'a>,
        method_ref: MethodRef<'a>,
        object: Option<ObjectReference<'a>>,
        args: Vec<Value<'a>>,
    ) -> InvokeMethodResult<'a> {
        todo!()
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
            return self.invoke_native_method(class_ref, method_ref, object, args);
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
        use crate::virtual_machine::VirtualMachine;
        let mut vm = VirtualMachine::new(102400);
        let file_system_path = FileSystemClassPath::new("./resources").unwrap();
        vm.add_class_path(Box::new(file_system_path));
        let rt_jar_path = JarFileClassPath::new("./resources/rt.jar").unwrap();
        let call_stack = vm.allocate_call_stack();
        vm.add_class_path(Box::new(rt_jar_path));
        let class_ref = vm.lookup_class(call_stack, "FieldTest").unwrap();
        assert!(matches!(class_ref.status, ClassStatus::Initialized));
    }
}
