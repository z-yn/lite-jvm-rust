use std::fmt::{Display, Formatter};

//栈帧信息，用来做异常调用栈回溯
pub struct StackTraceElement {
    pub declaring_class: String,
    pub method_name: String,
    pub file_name: Option<String>,
    pub line_number: u16,
}
impl Display for StackTraceElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\t{}.{}({}:{})",
            self.declaring_class,
            self.method_name,
            self.file_name
                .as_ref()
                .unwrap_or(&"<unknown source>".to_string()),
            self.line_number
        )
    }
}
