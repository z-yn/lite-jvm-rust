use crate::cesu8_byte_buffer::ByteBuffer;
use crate::class_file_error::{ClassFileError, Result};

//https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-6.html#jvms-6.5
pub enum Instruction {
    Aaload,
    Aastore,
}

pub fn read_one_instruction(code: &[u8]) -> Result<Instruction> {
    let mut buffer = ByteBuffer::new(code);
    let op_code = buffer.read_u8()?;
    let instruction = match op_code {
        0x32 => Instruction::Aaload,
        0x53 => Instruction::Aastore,
        op_code => {
            return Err(ClassFileError::InvalidCode(format!(
                "Invalid Op Code {op_code}"
            )));
        }
    };
    Ok(instruction)
}
