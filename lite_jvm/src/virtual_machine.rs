use crate::call_stack::CallStack;
use crate::jvm_exceptions::Result;
use crate::loaded_class::{ClassRef, MethodRef};
use crate::method_area::MethodArea;
use crate::object_heap::ObjectHeap;
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
    call_stacks: Arena<CallStack>,
}

impl<'a> VirtualMachine<'a> {
    pub fn new(heap_size: usize) -> VirtualMachine<'a> {
        VirtualMachine {
            method_area: MethodArea::new(),
            object_heap: ObjectHeap::new(heap_size),
            call_stacks: Arena::new(),
        }
    }

    fn link_class(&self, _class: ClassRef<'a>) -> Result<()> {
        Ok(())
    }
    //类的初始化。需要执行<clinit>方法。初始化一些变量。需要先实现方法执行
    fn initialize_class(&'a mut self, class: ClassRef<'a>) -> Result<()> {
        if let Ok(method) = class.get_method_info("<clinit>", "()V") {
            let class = self.method_area.get_mut(class).unwrap();
            //TODO 执行类初始化方法。将计算的字段信息存储到类中。需要先实现方法执行
        }
        Ok(())
    }
    pub fn lookup_class(&'a mut self, class_name: &str) -> Result<ClassRef> {
        let class = self.method_area.load_class(class_name)?;
        self.link_class(class)?;
        self.initialize_class(class)?;
        Ok(class)
    }

    pub fn look_method(
        &'a mut self,
        class_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> Result<(ClassRef, MethodRef)> {
        let class_ref = self.lookup_class(class_name)?;
        let method_ref = class_ref.get_method_info(method_name, descriptor)?;
        Ok((class_ref, method_ref))
    }
}
