extern crate bytes;

use crate::classes::*;
use std::{error, fmt, str};

// Bytes.into_buf() is used later, but Rust wrongly claims this import is unused
#[allow(unused_imports)]
use bytes::IntoBuf;

trait Deserialize: Sized {
    fn deserialize(data: &mut bytes::Buf) -> Result<Self, ClassLoaderError>;
}

impl Deserialize for Constant {
    fn deserialize(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
        if data.remaining() == 0 {
            return Err(ClassLoaderError::Eof("Unexpected end of stream; expected constant tag".to_string()));
        }

        let tag = data.get_u8();
        match tag {
            1 => deserialize_utf8(data),
            3 => deserialize_integer(data),
            _ => Err(ClassLoaderError::InvalidConstantType(tag)),
        }
    }
}

fn deserialize_utf8(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    if data.remaining() < 2 {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing length field on Utf8 constant".to_string()));
    }

    let length = data.get_u16_be() as usize;
    if data.remaining() < length {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing Utf8 constant".to_string()));
    }

    let mut contents = vec![0; length as usize];
    data.copy_to_slice(&mut contents);

    return str::from_utf8(&contents)
        .map(|slice| Constant::Utf8(slice.to_string()))
        .map_err(|err| ClassLoaderError::Utf8(err));
}

fn deserialize_integer(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    if data.remaining() < 4 {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing Integer constant".to_string()));
    }

    return Ok(Constant::Integer(data.get_u32_be()));
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClassLoaderError {
    Utf8(str::Utf8Error),
    Eof(String),
    InvalidConstantType(u8),
}

impl fmt::Display for ClassLoaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ClassLoaderError::Utf8(ref cause) => write!(f, "Failed to decode UTF-8: {}", cause),
            ClassLoaderError::Eof(ref msg) => write!(f, "Unexpected EOF: {}", msg),
            ClassLoaderError::InvalidConstantType(ref tag) => write!(f, "Unsupported constant type {}", tag),
        }
    }
}

impl error::Error for ClassLoaderError {
    fn description(&self) -> &str {
        match *self {
            ClassLoaderError::Utf8(_) => "Failed to decode Utf8 data",
            ClassLoaderError::Eof(ref msg) => msg,
            ClassLoaderError::InvalidConstantType(..) => "Unsupported constant type"
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ClassLoaderError::Utf8(ref cause) => Some(cause),
            ClassLoaderError::Eof(..) => None,
            ClassLoaderError::InvalidConstantType(..) => None,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_utf8() {
        assert_constant(Constant::Utf8("Hello".to_string()), b"\x01\x00\x05Hello");
    }

    #[test]
    fn test_deserialize_utf8_2() {
        assert_constant(Constant::Utf8("Some other string".to_string()), b"\x01\x00\x11Some other string");
    }

    #[test]
    fn test_deserialize_utf8_empty_string() {
        assert_constant(Constant::Utf8("".to_string()), b"\x01\x00\x00");
    }

    #[test]
    fn test_deserialize_constant_empty_buffer() {
        assert_eof_in_constant(b"");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_after_tag() {
        assert_eof_in_constant(b"\x01");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_after_first_length_byte() {
        assert_eof_in_constant(b"\x01\x00");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_before_body() {
        assert_eof_in_constant(b"\x01\x00\x01");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_in_body() {
        assert_eof_in_constant(b"\x01\x00\x20Hello world");
    }

    #[test]
    fn test_deserialize_utf8_invalid_two_octet_sequence() {
        assert_invalid_utf8(b"\x01\x00\x02\xc3\x28");
    }

    #[test]
    fn test_deserialize_utf8_invalid_three_octet_sequence_1() {
        assert_invalid_utf8(b"\x01\x00\x03\xe2\x28\xa1");
    }

    #[test]
    fn test_deserialize_utf8_invalid_three_octet_sequence_2() {
        assert_invalid_utf8(b"\x01\x00\x03\xe2\x82\x28");
    }

    #[test]
    fn test_deserialize_utf8_invalid_four_octet_sequence_1() {
        assert_invalid_utf8(b"\x01\x00\x04\xf0\x28\x8c\xbc");
    }

    #[test]
    fn test_deserialize_utf8_invalid_four_octet_sequence_2() {
        assert_invalid_utf8(b"\x01\x00\x04\xf0\x90\x28\xbc");
    }

    #[test]
    fn test_deserialize_utf8_invalid_four_octet_sequence_3() {
        assert_invalid_utf8(b"\x01\x00\x04\xf0\x28\x8c\x28");
    }

    #[test]
    fn test_deserialize_utf8_five_octet_sequence() {
        assert_invalid_utf8(b"\x01\x00\x05\xf8\xa1\xa1\xa1\xa1");
    }

    #[test]
    fn test_deserialize_utf8_six_octet_sequence() {
        assert_invalid_utf8(b"\x01\x00\x06\xfc\xa1\xa1\xa1\xa1\xa1");
    }

    #[test]
    fn test_deserialize_integer_0x00000000() {
        assert_constant(Constant::Integer(0x0000), b"\x03\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_integer_0x00000001() {
        assert_constant(Constant::Integer(0x0001), b"\x03\x00\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_integer_0x1234abcd() {
        assert_constant(Constant::Integer(0x1234abcd), b"\x03\x12\x34\xab\xcd");
    }

    #[test]
    fn test_deserialize_integer_premature_termination_1() {
        assert_eof_in_constant(b"\x03");
    }

    #[test]
    fn test_deserialize_integer_premature_termination_2() {
        assert_eof_in_constant(b"\x03\xff");
    }

    #[test]
    fn test_deserialize_integer_premature_termination_3() {
        assert_eof_in_constant(b"\x03\xff\xff");
    }

    #[test]
    fn test_deserialize_integer_premature_termination_4() {
        assert_eof_in_constant(b"\x03\xff\xff\xff");
    }

    fn assert_constant(constant: Constant, input: &[u8]) {
        assert_eq!(Ok(constant), Constant::deserialize(&mut bytes::Bytes::from(input).into_buf()));
    }

    fn assert_eof_in_constant(input: &[u8]) {
        deserialize_constant_expecting_error(input, |err| match *err {
            ClassLoaderError::Eof(_) => (),
            _ => panic!("Expected EOF, but got {:#?}", err),
        });
    }

    fn assert_invalid_utf8(input: &[u8]) {
        deserialize_constant_expecting_error(input, |err| match *err {
            ClassLoaderError::Utf8(_) => (),
            _ => panic!("Expected Utf8 parse error, but got {:#?}", err),
        });
    }

    fn deserialize_constant_expecting_error<F>(input: &[u8], handler: F) where
        F: Fn(&ClassLoaderError) {
        let res = Constant::deserialize(&mut bytes::Bytes::from(input).into_buf());
        match res {
            Ok(ref res) => panic!("Expected EOF, but got result {:#?}", res),
            Err(ref err) => handler(&err),
        }
    }
}
