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
        data.advance(1); // Skip tag byte

        let length = data.get_u16_be();
        let mut contents = vec![0; length as usize];
        data.copy_to_slice(&mut contents);

        return str::from_utf8(&contents)
            .map(|slice| Constant::Utf8(slice.to_string()))
            .map_err(|err| ClassLoaderError::Utf8(err));
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClassLoaderError {
    Utf8(str::Utf8Error),
    Other(String),
}

impl fmt::Display for ClassLoaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ClassLoaderError::Utf8(ref cause) => write!(f, "Failed to decode UTF-8: {}", cause),
            ClassLoaderError::Other(ref msg) => write!(f, "Unexpected error during classload: {}", msg),
        }
    }
}

impl error::Error for ClassLoaderError {
    fn description(&self) -> &str {
        match *self {
            ClassLoaderError::Utf8(_) => "Failed to decode Utf8 data",
            ClassLoaderError::Other(ref msg) => msg,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ClassLoaderError::Utf8(ref cause) => Some(cause),
            ClassLoaderError::Other(..) => None,
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

    fn assert_constant(constant: Constant, input: &str) {
        assert_eq!(Ok(constant), Constant::deserialize(&mut bytes::Bytes::from(input).into_buf()));
    }
}
