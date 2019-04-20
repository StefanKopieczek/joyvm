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

        // For now assume all constants are Utf8 literals.
        // We'll add other constant types later.
        data.advance(1);
        return deserialize_utf8(data);
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

#[derive(Debug, PartialEq, Eq)]
pub enum ClassLoaderError {
    Utf8(str::Utf8Error),
    Eof(String),
}

impl fmt::Display for ClassLoaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ClassLoaderError::Utf8(ref cause) => write!(f, "Failed to decode UTF-8: {}", cause),
            ClassLoaderError::Eof(ref msg) => write!(f, "Unexpected EOF: {}", msg),
        }
    }
}

impl error::Error for ClassLoaderError {
    fn description(&self) -> &str {
        match *self {
            ClassLoaderError::Utf8(_) => "Failed to decode Utf8 data",
            ClassLoaderError::Eof(ref msg) => msg,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ClassLoaderError::Utf8(ref cause) => Some(cause),
            ClassLoaderError::Eof(..) => None,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_utf8() {
        assert_constant(Constant::Utf8("Hello".to_string()), &"\x01\x00\x05Hello");
    }

    #[test]
    fn test_deserialize_utf8_2() {
        assert_constant(Constant::Utf8("Some other string".to_string()), &"\x01\x00\x11Some other string");
    }

    #[test]
    fn test_deserialize_utf8_empty_string() {
        assert_constant(Constant::Utf8("".to_string()), &"\x01\x00\x00");
    }

    #[test]
    fn test_deserialize_constant_empty_buffer() {
        assert_eof_in_constant(&"");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_after_tag() {
        assert_eof_in_constant(&"\x01");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_after_first_length_byte() {
        assert_eof_in_constant(&"\x01\x00");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_before_body() {
        assert_eof_in_constant(&"\x01\x00\x01");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_in_body() {
        assert_eof_in_constant(&"\x01\x00\x20Hello world");
    }

    fn assert_constant(constant: Constant, input: &str) {
        assert_eq!(Ok(constant), Constant::deserialize(&mut bytes::Bytes::from(input).into_buf()));
    }

    fn assert_eof_in_constant(input: &str) {
        deserialize_constant_expecting_error(input, |err| match *err {
            ClassLoaderError::Eof(_) => (),
            _ => panic!("Expected EOF, but got errpr {:#?}", err),
        });
    }

    fn deserialize_constant_expecting_error<F>(input: &str, handler: F) where
        F: Fn(&ClassLoaderError) {
        let res = Constant::deserialize(&mut bytes::Bytes::from(input).into_buf());
        match res {
            Ok(ref res) => panic!("Expected EOF, but got result {:#?}", res),
            Err(ref err) => handler(&err),
        }
    }
}
