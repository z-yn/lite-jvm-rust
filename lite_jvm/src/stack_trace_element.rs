//栈帧信息，用来做异常调用栈回溯
pub struct StackTraceElement<'a> {
    pub declaring_class: &'a str,
    pub method_name: &'a str,
    pub file_name: &'a str,
    pub line_number: u16,
}
