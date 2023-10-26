use crate::cesu8_byte_buffer::ByteBuffer;
use crate::class_file_error::{ClassFileError, Result};

//https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-6.html#jvms-6.5

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instruction {
    Aaload,
    Aastore,
    Aconst_null,
    Aload(u8),
    Aload_0,
    Aload_1,
    Aload_2,
    Aload_3,
    Anewarray(u16),
    Areturn,
    Arraylength,
    Astore(u8),
    Astore_0,
    Astore_1,
    Astore_2,
    Astore_3,
    Athrow,
    Baload,
    Bastore,
    Bipush(u8),
    Caload,
    Castore,
    Checkcast(u16),
}

pub fn read_one_instruction(buffer: &mut ByteBuffer) -> Result<Instruction> {
    // let mut buffer = ByteBuffer::new(code);
    let op_code = buffer.read_u8()?;
    let instruction = match op_code {
        0x32 => Instruction::Aaload,
        0x53 => Instruction::Aastore,
        0x1 => Instruction::Aconst_null,
        0x19 => Instruction::Aload(buffer.read_u8()?),
        0x2a => Instruction::Aload_0,
        0x2b => Instruction::Aload_1,
        0x2c => Instruction::Aload_2,
        0x2d => Instruction::Aload_3,
        0xbd => Instruction::Anewarray(buffer.read_u16()?),
        0xb0 => Instruction::Areturn,
        0xbe => Instruction::Arraylength,
        0x3a => Instruction::Astore(buffer.read_u8()?),
        0x4b => Instruction::Astore_0,
        0x4c => Instruction::Astore_1,
        0x4d => Instruction::Astore_2,
        0x4e => Instruction::Astore_3,
        0xbf => Instruction::Athrow,
        0x33 => Instruction::Baload,
        0x54 => Instruction::Bastore,
        0x10 => Instruction::Bipush(buffer.read_u8()?),
        0x34 => Instruction::Caload,
        0x55 => Instruction::Castore,
        0xc0 => Instruction::Checkcast(buffer.read_u16()?),
        op_code => {
            return Err(ClassFileError::InvalidCode(format!(
                "Invalid Op Code {op_code}"
            )));
        }
    };
    Ok(instruction)
}
