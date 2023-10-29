use crate::class_file_error::{ClassFileError, Result};
use cesu8::from_java_cesu8;
use std::i16;
pub struct ByteBuffer<'a> {
    buffer: &'a [u8],
    pub position: usize,
}

impl<'a> ByteBuffer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        ByteBuffer {
            buffer: data,
            position: 0,
        }
    }

    fn advance(&mut self, size: usize) -> Result<&'a [u8]> {
        if self.position + size > self.buffer.len() {
            Err(ClassFileError::UnexpectedEndOfData)
        } else {
            let slice = &self.buffer[self.position..self.position + size];
            self.position += size;
            Ok(slice)
        }
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        self.advance(std::mem::size_of::<u8>())
            .map(|bytes| u8::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        self.advance(std::mem::size_of::<i8>())
            .map(|bytes| i8::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        self.advance(std::mem::size_of::<i16>())
            .map(|bytes| i16::from_be_bytes(bytes.try_into().unwrap()))
    }
    pub fn read_2_u16(&mut self) -> Result<(u16, u16)> {
        let first = self.read_u16()?;
        let second = self.read_u16()?;
        Ok((first, second))
    }

    pub fn read_u8_u16(&mut self) -> Result<(u8, u16)> {
        let first = self.read_u8()?;
        let second = self.read_u16()?;
        Ok((first, second))
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        self.advance(std::mem::size_of::<u16>())
            .map(|bytes| u16::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        self.advance(std::mem::size_of::<u32>())
            .map(|bytes| u32::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        self.advance(std::mem::size_of::<i32>())
            .map(|bytes| i32::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        self.advance(std::mem::size_of::<i64>())
            .map(|bytes| i64::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        self.advance(std::mem::size_of::<f32>())
            .map(|bytes| f32::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn read_f64(&mut self) -> Result<f64> {
        self.advance(std::mem::size_of::<f64>())
            .map(|bytes| f64::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn read_utf8(&mut self, len: usize) -> Result<String> {
        self.advance(len)
            .and_then(|bytes| {
                from_java_cesu8(bytes).map_err(|_| ClassFileError::InvalidCesu8String)
            })
            .map(|cow_string| cow_string.into_owned())
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        self.advance(len)
    }

    pub fn has_more_data(&self) -> bool {
        self.position < self.buffer.len()
    }

    pub fn jump_to(&mut self, position: usize) {
        assert!(position <= self.buffer.len());
        self.position = position;
    }
}

#[cfg(test)]
mod tests {
    use crate::cesu8_byte_buffer::ByteBuffer;

    #[test]
    fn buffer_works() {
        let data = vec![0x00, 0x00, 0x00, 0x42];
        let mut buffer = ByteBuffer::new(&data);
        assert!(buffer.has_more_data());
        assert_eq!(0x42u32, buffer.read_u32().unwrap());
        assert!(!buffer.has_more_data());
        assert!(buffer.read_u32().is_err());
    }
}
