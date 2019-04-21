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
            4 => deserialize_float(data),
            5 => deserialize_long(data),
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

fn deserialize_float(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    if data.remaining() < 4 {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing Float constant".to_string()));
    }

    return Ok(Constant::Float(data.get_f32_be()));
}

fn deserialize_long(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    if data.remaining() < 8 {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing Long constant".to_string()));
    }

    return Ok(Constant::Long(data.get_u64_be()));
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

    #[test]
    fn test_deserialize_float_smallest_possible_subnormal_number() {
        do_float_test(0x00000001, b"\x04\x00\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_float_0x00000002() {
        do_float_test(0x00000002, b"\x04\x00\x00\x00\x02");
    }

    #[test]
    fn test_deserialize_float_largest_subnormal_number() {
        do_float_test(0x007fffff, b"\x04\x00\x7f\xff\xff");
    }

    #[test]
    fn test_deserialize_float_smallest_possible_normal_number() {
        do_float_test(0x00800000, b"\x04\x00\x80\x00\x00");
    }

    #[test]
    fn test_deserialize_float_largest_normal_number() {
        do_float_test(0x7f7fffff, b"\x04\x7f\x7f\xff\xff");
    }

    #[test]
    fn test_deserialize_float_largest_number_less_than_one() {
        do_float_test(0x3f7fffff, b"\x04\x3f\x7f\xff\xff");
    }

    #[test]
    fn test_deserialize_float_equal_to_one() {
        do_float_test(0x3f800000, b"\x04\x3f\x80\x00\x00");
    }

    #[test]
    fn test_deserialize_float_smallest_number_larger_than_one() {
        do_float_test(0x3f800001, b"\x04\x3f\x80\x00\x01");
    }

    #[test]
    fn test_deserialize_float_negative_two() {
        do_float_test(0xc0000000, b"\x04\xc0\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_float_zero() {
        do_float_test(0x00000000, b"\x04\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_float_negative_zero() {
        do_float_test(0x80000000, b"\x04\x80\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_float_positive_infinity() {
        do_float_test(0x7f800000, b"\x04\x7f\x80\x00\x00");
    }

    #[test]
    fn test_deserialize_float_pi() {
        do_float_test(0x40490fdb, b"\x04\x40\x49\x0f\xdb");
    }

    #[test]
    fn test_deserialize_float_one_third() {
        do_float_test(0x3eaaaaab, b"\x04\x3e\xaa\xaa\xab");
    }

    #[test]
    fn test_deserialize_float_qnan() {
        // NaN != NaN so we have to check the result directly
        let bytes: &[u8] = b"\x04\xff\xc0\x00\x01";
        let result = Constant::deserialize(&mut bytes::Bytes::from(bytes).into_buf()).unwrap();
        match result {
            Constant::Float(ref float) => assert!(float.is_nan()),
            _ => panic!("Expected float; got unexpected constant {:#?}", result),
        }
    }

    #[test]
    fn test_deserialize_float_snan() {
        // NaN != NaN so we have to check the result directly
        let bytes: &[u8] = b"\x04\xff\x80\x00\x01";
        let result = Constant::deserialize(&mut bytes::Bytes::from(bytes).into_buf()).unwrap();
        match result {
            Constant::Float(ref float) => assert!(float.is_nan()),
            _ => panic!("Expected float; got unexpected constant {:#?}", result),
        }
    }

    #[test]
    fn test_deserialize_float_premature_termination_1() {
        assert_eof_in_constant(b"\x04");
    }

    #[test]
    fn test_deserialize_float_premature_termination_2() {
        assert_eof_in_constant(b"\x04\x00");
    }

    #[test]
    fn test_deserialize_float_premature_termination_3() {
        assert_eof_in_constant(b"\x04\x00\x00");
    }

    #[test]
    fn test_deserialize_float_premature_termination_4() {
        assert_eof_in_constant(b"\x04\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_long_0x0000000000000000() {
        assert_constant(Constant::Long(0), b"\x05\x00\x00\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_long_0x0000000000000001() {
        assert_constant(Constant::Long(1), b"\x05\x00\x00\x00\x00\x00\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_long_0x123456789abcdef0() {
        assert_constant(Constant::Long(0x123456789abcdef0), b"\x05\x12\x34\x56\x78\x9a\xbc\xde\xf0");
    }

    #[test]
    fn test_deserialize_long_premature_termination_1() {
        assert_eof_in_constant(b"\x05");
    }

    #[test]
    fn test_deserialize_long_premature_termination_2() {
        assert_eof_in_constant(b"\x05\x12");
    }

    #[test]
    fn test_deserialize_long_premature_termination_3() {
        assert_eof_in_constant(b"\x05\x12\x34");
    }

    #[test]
    fn test_deserialize_long_premature_termination_4() {
        assert_eof_in_constant(b"\x05\x12\x34\x56");
    }

    #[test]
    fn test_deserialize_long_premature_termination_5() {
        assert_eof_in_constant(b"\x05\x12\x34\x56\x78");
    }

    #[test]
    fn test_deserialize_long_premature_termination_6() {
        assert_eof_in_constant(b"\x05\x12\x34\x56\x78\x9a");
    }

    #[test]
    fn test_deserialize_long_premature_termination_7() {
        assert_eof_in_constant(b"\x05\x12\x34\x56\x78\x9a\xbc");
    }

    #[test]
    fn test_deserialize_long_premature_termination_8() {
        assert_eof_in_constant(b"\x05\x12\x34\x56\x78\x9a\xbc\xde");
    }

    fn do_float_test(float_bits: u32, input: &[u8]) {
        assert_constant(Constant::Float(f32::from_bits(float_bits)), input);
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
