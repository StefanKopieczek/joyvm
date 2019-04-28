extern crate bytes;

use crate::classes::*;
use std::{error, fmt, str};

// Bytes.into_buf() is used later, but Rust wrongly claims this import is unused
#[allow(unused_imports)]
use bytes::IntoBuf;

// Trait for entities that can be unambiguously deserialized without reference to
// other sibling or parent entities.
trait Deserialize: Sized {
    fn deserialize(data: &mut bytes::Buf) -> Result<Self, ClassLoaderError>;
}

// Trait for entities that require information about the ConstantPool to be
// deserialized.
trait DeserializeWithConstants: Sized {
    fn deserialize(data: &mut bytes::Buf, constants: &Vec<Constant>) -> Result<Self, ClassLoaderError>;
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
            6 => deserialize_double(data),
            7 => deserialize_classref(data),
            8 => deserialize_string(data),
            9 => deserialize_fieldref(data),
            10 => deserialize_methodref(data),
            11 => deserialize_interface_method_ref(data),
            15 => deserialize_method_handle_ref(data),
            16 => deserialize_method_type(data),
            18 => deserialize_invoke_dynamic_info(data),
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

fn deserialize_double(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    if data.remaining() < 8 {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing Double constant".to_string()));
    }

    return Ok(Constant::Double(data.get_f64_be()));
}

fn deserialize_classref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    return deserialize_constant_index(data).map(Constant::ClassRef);
}

fn deserialize_string(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    return deserialize_constant_index(data).map(Constant::StringRef);
}

fn deserialize_fieldref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    let class = deserialize_constant_index(data)?;
    let name_and_type = deserialize_constant_index(data)?;
    return Ok(Constant::FieldRef {class: class, name_and_type: name_and_type});
}

fn deserialize_methodref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    let class = deserialize_constant_index(data)?;
    let name_and_type = deserialize_constant_index(data)?;
    return Ok(Constant::MethodRef {class: class, name_and_type: name_and_type});
}

fn deserialize_interface_method_ref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    let class = deserialize_constant_index(data)?;
    let name_and_type = deserialize_constant_index(data)?;
    return Ok(Constant::InterfaceMethodRef {class: class, name_and_type: name_and_type});
}

fn deserialize_method_handle_ref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    if data.remaining() == 0 {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing method handle ref".to_string()));
    }

    let kind = data.get_u8();
    let index = deserialize_constant_index(data)?;
    let handle = match kind {
        1 => Ok(MethodHandle::GetField(index)),
        2 => Ok(MethodHandle::GetStatic(index)),
        3 => Ok(MethodHandle::PutField(index)),
        4 => Ok(MethodHandle::PutStatic(index)),
        5 => Ok(MethodHandle::InvokeVirtual(index)),
        6 => Ok(MethodHandle::InvokeStatic(index)),
        7 => Ok(MethodHandle::InvokeSpecial(index)),
        8 => Ok(MethodHandle::NewInvokeSpecial(index)),
        9 => Ok(MethodHandle::InvokeInterface(index)),
        _ => Err(ClassLoaderError::InvalidMethodHandleKind(kind)),
    };

    return handle.map(|h| Constant::MethodHandleRef(h));
}

fn deserialize_method_type(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    return Ok(Constant::MethodType(deserialize_constant_index(data)?));
}

fn deserialize_invoke_dynamic_info(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    return Ok(Constant::InvokeDynamicInfo{
        bootstrap_method_attr: deserialize_method_index(data)?,
        name_and_type: deserialize_constant_index(data)?,
    });
}

fn deserialize_constant_index(data: &mut bytes::Buf) -> Result<ConstantIndex, ClassLoaderError> {
    if data.remaining() < 2 {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing constant index".to_string()));
    }

    return Ok(ConstantIndex(data.get_u16_be()));
}

fn deserialize_method_index(data: &mut bytes::Buf) -> Result<MethodIndex, ClassLoaderError> {
    if data.remaining() < 2 {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing method index".to_string()));
    }

    return Ok(MethodIndex(data.get_u16_be()));
}

impl DeserializeWithConstants for Attribute {
    fn deserialize(data: &mut bytes::Buf, constants: &Vec<Constant>) -> Result<Attribute, ClassLoaderError> {
        let attribute_type_index = deserialize_constant_index(data)?;
        let attribute_type_ref = attribute_type_index.lookup(constants)?;
        let attribute_type = match *attribute_type_ref {
            Constant::Utf8(ref attr_type) => Ok(attr_type),
            _ => Err(ClassLoaderError::InvalidAttributeType(attribute_type_ref.clone())),
        }?;

        match attribute_type.as_ref() {
            "ConstantValue" => deserialize_constant_value(attribute_type_index, data),
            _ => Err(ClassLoaderError::UnknownAttributeType(attribute_type.to_string()))
        }
    }
}

fn deserialize_constant_value(attribute_name: ConstantIndex, data: &mut bytes::Buf) -> Result<Attribute, ClassLoaderError> {
    if data.remaining() < 4 {
        return Err(ClassLoaderError::Eof("Unexpected end of stream while parsing attribute length".to_string()));
    }

    let length = data.get_u32_be();
    if length != 2 {
        return Err(ClassLoaderError::Misc("ConstantValue attribute must have length of exactly 2".to_string()));
    }

    return Ok(Attribute::ConstantValue {
        attribute_name: attribute_name,
        constant_value: deserialize_constant_index(data)?,
    });
}

#[derive(Debug, PartialEq)]
pub enum ClassLoaderError {
    Utf8(str::Utf8Error),
    Eof(String),
    InvalidConstantRef(ConstantLookupError),
    InvalidConstantType(u8),
    InvalidMethodHandleKind(u8),
    InvalidAttributeType(Constant),
    Misc(String),
    UnknownAttributeType(String),
}

impl std::convert::From<ConstantLookupError> for ClassLoaderError {
    fn from(cause: ConstantLookupError) -> ClassLoaderError {
        return ClassLoaderError::InvalidConstantRef(cause);
    }
}

impl fmt::Display for ClassLoaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ClassLoaderError::Utf8(ref cause) => write!(f, "Failed to decode UTF-8: {}", cause),
            ClassLoaderError::Eof(ref msg) => write!(f, "Unexpected EOF: {}", msg),
            ClassLoaderError::InvalidConstantRef(ref cause) => write!(f, "Invalid constant reference: {}", cause),
            ClassLoaderError::InvalidConstantType(ref tag) => write!(f, "Unsupported constant type {}", tag),
            ClassLoaderError::InvalidMethodHandleKind(ref kind) => write!(f, "Unsupported method handle kind {}", kind),
            ClassLoaderError::InvalidAttributeType(ref attribute_type) => write!(f, "Invalid attribute type {:#?}", attribute_type),
            ClassLoaderError::Misc(ref msg) => write!(f, "Unexpected error during class load: {}", msg),
            ClassLoaderError::UnknownAttributeType(ref type_name) => write!(f, "Unknown attribute type '{}'", type_name),
        }
    }
}

impl error::Error for ClassLoaderError {
    fn description(&self) -> &str {
        match *self {
            ClassLoaderError::Utf8(_) => "Failed to decode Utf8 data",
            ClassLoaderError::Eof(ref msg) => msg,
            ClassLoaderError::InvalidConstantRef(_) => "Invalid constant reference",
            ClassLoaderError::InvalidConstantType(..) => "Unsupported constant type",
            ClassLoaderError::InvalidMethodHandleKind(..) => "Unsupported method handle kind",
            ClassLoaderError::InvalidAttributeType(..) => "Invalid attribute type",
            ClassLoaderError::Misc(ref msg) => msg,
            ClassLoaderError::UnknownAttributeType(..) => "Unknown attribute type",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ClassLoaderError::Utf8(ref cause) => Some(cause),
            ClassLoaderError::InvalidConstantRef(ref cause) => Some(cause),
            ClassLoaderError::Eof(..) => None,
            ClassLoaderError::InvalidConstantType(..) => None,
            ClassLoaderError::InvalidMethodHandleKind(..) => None,
            ClassLoaderError::InvalidAttributeType(..) => None,
            ClassLoaderError::Misc(..) => None,
            ClassLoaderError::UnknownAttributeType(..) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fmt::Debug;

    #[test]
    fn test_deserialize_utf8() {
        assert_deserialize(Constant::Utf8("Hello".to_string()), b"\x01\x00\x05Hello");
    }

    #[test]
    fn test_deserialize_utf8_2() {
        assert_deserialize(Constant::Utf8("Some other string".to_string()), b"\x01\x00\x11Some other string");
    }

    #[test]
    fn test_deserialize_utf8_empty_string() {
        assert_deserialize(Constant::Utf8("".to_string()), b"\x01\x00\x00");
    }

    #[test]
    fn test_deserialize_constant_empty_buffer() {
        assert_eof(Constant::deserialize, b"");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_after_tag() {
        assert_eof(Constant::deserialize, b"\x01");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_after_first_length_byte() {
        assert_eof(Constant::deserialize, b"\x01\x00");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_before_body() {
        assert_eof(Constant::deserialize, b"\x01\x00\x01");
    }

    #[test]
    fn test_deserialize_utf8_premature_termination_in_body() {
        assert_eof(Constant::deserialize, b"\x01\x00\x20Hello world");
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
        assert_deserialize(Constant::Integer(0x0000), b"\x03\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_integer_0x00000001() {
        assert_deserialize(Constant::Integer(0x0001), b"\x03\x00\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_integer_0x1234abcd() {
        assert_deserialize(Constant::Integer(0x1234abcd), b"\x03\x12\x34\xab\xcd");
    }

    #[test]
    fn test_deserialize_integer_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x03");
    }

    #[test]
    fn test_deserialize_integer_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x03\xff");
    }

    #[test]
    fn test_deserialize_integer_premature_termination_3() {
        assert_eof(Constant::deserialize, b"\x03\xff\xff");
    }

    #[test]
    fn test_deserialize_integer_premature_termination_4() {
        assert_eof(Constant::deserialize, b"\x03\xff\xff\xff");
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
        let result = Constant::deserialize(&mut bytes::Bytes::from(bytes).into_buf())
            .expect("Failed to parse serialized float constant");
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
        assert_eof(Constant::deserialize, b"\x04");
    }

    #[test]
    fn test_deserialize_float_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x04\x00");
    }

    #[test]
    fn test_deserialize_float_premature_termination_3() {
        assert_eof(Constant::deserialize, b"\x04\x00\x00");
    }

    #[test]
    fn test_deserialize_float_premature_termination_4() {
        assert_eof(Constant::deserialize, b"\x04\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_long_0x0000000000000000() {
        assert_deserialize(Constant::Long(0), b"\x05\x00\x00\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_long_0x0000000000000001() {
        assert_deserialize(Constant::Long(1), b"\x05\x00\x00\x00\x00\x00\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_long_0x123456789abcdef0() {
        assert_deserialize(Constant::Long(0x123456789abcdef0), b"\x05\x12\x34\x56\x78\x9a\xbc\xde\xf0");
    }

    #[test]
    fn test_deserialize_long_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x05");
    }

    #[test]
    fn test_deserialize_long_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x05\x12");
    }

    #[test]
    fn test_deserialize_long_premature_termination_3() {
        assert_eof(Constant::deserialize, b"\x05\x12\x34");
    }

    #[test]
    fn test_deserialize_long_premature_termination_4() {
        assert_eof(Constant::deserialize, b"\x05\x12\x34\x56");
    }

    #[test]
    fn test_deserialize_long_premature_termination_5() {
        assert_eof(Constant::deserialize, b"\x05\x12\x34\x56\x78");
    }

    #[test]
    fn test_deserialize_long_premature_termination_6() {
        assert_eof(Constant::deserialize, b"\x05\x12\x34\x56\x78\x9a");
    }

    #[test]
    fn test_deserialize_long_premature_termination_7() {
        assert_eof(Constant::deserialize, b"\x05\x12\x34\x56\x78\x9a\xbc");
    }

    #[test]
    fn test_deserialize_long_premature_termination_8() {
        assert_eof(Constant::deserialize, b"\x05\x12\x34\x56\x78\x9a\xbc\xde");
    }

    #[test]
    fn test_deserialize_double_equal_to_1() {
        do_double_test(0x3FF0000000000000, b"\x06\x3f\xf0\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_double_smallest_number_greater_than_1() {
        do_double_test(0x3FF0000000000001, b"\x06\x3f\xf0\x00\x00\x00\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_double_equal_to_2() {
        do_double_test(0x4000000000000000, b"\x06\x40\x00\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_double_equal_to_negative_2() {
        do_double_test(0xc000000000000000, b"\x06\xc0\x00\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_double_approx_one_third() {
        do_double_test(0x3fd5555555555555, b"\x06\x3f\xd5\x55\x55\x55\x55\x55\x55");
    }

    #[test]
    fn test_deserialize_double_approx_pi() {
        do_double_test(0x400921fb54442d18, b"\x06\x40\x09\x21\xfb\x54\x44\x2d\x18");
    }

    #[test]
    fn test_deserialize_double_positive_zero() {
        do_double_test(0, b"\x06\x00\x00\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_double_negative_zero() {
        do_double_test(0x8000000000000000, b"\x06\x80\x00\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_double_positive_infinity() {
        do_double_test(0x7ff0000000000000, b"\x06\x7f\xf0\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_double_negative_infinity() {
        do_double_test(0xfff0000000000000, b"\x06\xff\xf0\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_double_snan() {
        // NaN != NaN so we have to check the result directly
        let bytes: &[u8] = b"\x06\x7f\xff\x00\x00\x00\x00\x00\x00\x00\x01";
        let res = Constant::deserialize(&mut bytes::Bytes::from(bytes).into_buf())
            .expect("Failed to parse serialized double constant");
        match res {
            Constant::Double(ref double) => assert!(double.is_nan()),
            _ => panic!("Unexpected constant; expected double, got {:#?}", res),
        }
    }

    #[test]
    fn test_deserialize_double_qnan() {
        // NaN != NaN so we have to check the result directly
        let bytes: &[u8] = b"\x06\x7f\xff\x80\x00\x00\x00\x00\x00\x00\x01";
        let res = Constant::deserialize(&mut bytes::Bytes::from(bytes).into_buf())
            .expect("Failed to parse serialized double constant");
        match res {
            Constant::Double(ref double) => assert!(double.is_nan()),
            _ => panic!("Unexpected constant; expected double, got {:#?}", res),
        }
    }

    #[test]
    fn test_deserialize_double_alt_nan() {
        // NaN != NaN so we have to check the result directly
        let bytes: &[u8] = b"\x06\x7f\xff\xff\xff\xff\xff\xff\xff";
        let res = Constant::deserialize(&mut bytes::Bytes::from(bytes).into_buf())
            .expect("Failed to parse serialized double constant");
        match res {
            Constant::Double(ref double) => assert!(double.is_nan()),
            _ => panic!("Unexpected constant; expected double, got {:#?}", res),
        }
    }

    #[test]
    fn test_deserialize_double_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x06");
    }

    #[test]
    fn test_deserialize_double_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x06\x12");
    }

    #[test]
    fn test_deserialize_double_premature_termination_3() {
        assert_eof(Constant::deserialize, b"\x06\x12\x34");
    }

    #[test]
    fn test_deserialize_double_premature_termination_4() {
        assert_eof(Constant::deserialize, b"\x06\x12\x34\x56");
    }

    #[test]
    fn test_deserialize_double_premature_termination_5() {
        assert_eof(Constant::deserialize, b"\x06\x12\x34\x56\x78");
    }

    #[test]
    fn test_deserialize_double_premature_termination_6() {
        assert_eof(Constant::deserialize, b"\x06\x12\x34\x56\x78\x9a");
    }

    #[test]
    fn test_deserialize_double_premature_termination_7() {
        assert_eof(Constant::deserialize, b"\x06\x12\x34\x56\x78\x9a\xbc");
    }

    #[test]
    fn test_deserialize_double_premature_termination_8() {
        assert_eof(Constant::deserialize, b"\x06\x12\x34\x56\x78\x9a\xbc\xde");
    }

    #[test]
    fn test_deserialize_class_with_name_index_0() {
        assert_deserialize(Constant::ClassRef(ConstantIndex(0)), b"\x07\x00\x00");
    }

    #[test]
    fn test_deserialize_class_with_name_index_1() {
        assert_deserialize(Constant::ClassRef(ConstantIndex(1)), b"\x07\x00\x01");
    }

    #[test]
    fn test_deserialize_class_with_name_index_abcd() {
        assert_deserialize(Constant::ClassRef(ConstantIndex(0xabcd)), b"\x07\xab\xcd");
    }

    #[test]
    fn test_deserialize_class_with_name_index_ffff() {
        assert_deserialize(Constant::ClassRef(ConstantIndex(0xffff)), b"\x07\xff\xff");
    }

    #[test]
    fn test_deserialize_class_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x07");
    }

    #[test]
    fn test_deserialize_class_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x07\xab");
    }

    #[test]
    fn test_deserialize_string_with_utf_index_0() {
        assert_deserialize(Constant::StringRef(ConstantIndex(0)), b"\x08\x00\x00");
    }

    #[test]
    fn test_deserialize_string_with_utf_index_1() {
        assert_deserialize(Constant::StringRef(ConstantIndex(1)), b"\x08\x00\x01");
    }

    #[test]
    fn test_deserialize_string_with_utf_index_abcd() {
        assert_deserialize(Constant::StringRef(ConstantIndex(0xabcd)), b"\x08\xab\xcd");
    }

    #[test]
    fn test_deserialize_string_with_utf_index_ffff() {
        assert_deserialize(Constant::StringRef(ConstantIndex(0xffff)), b"\x08\xff\xff");
    }

    #[test]
    fn test_deserialize_string_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x08");
    }

    #[test]
    fn test_deserialize_string_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x08\x01");
    }

    #[test]
    fn test_deserialize_field_ref_with_0000_and_0000() {
        assert_deserialize(Constant::FieldRef {
            class: ConstantIndex(0),
            name_and_type: ConstantIndex(0),
        }, b"\x09\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_field_ref_with_abcd_and_1234() {
        assert_deserialize(Constant::FieldRef {
            class: ConstantIndex(0xabcd),
            name_and_type: ConstantIndex(0x1234),
        }, b"\x09\xab\xcd\x12\x34");
    }

    #[test]
    fn test_deserialize_field_ref_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x09");
    }

    #[test]
    fn test_deserialize_field_ref_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x09\x00");
    }

    #[test]
    fn test_deserialize_field_ref_premature_termination_3() {
        assert_eof(Constant::deserialize, b"\x09\x00\x00");
    }

    #[test]
    fn test_deserialize_field_ref_premature_termination_4() {
        assert_eof(Constant::deserialize, b"\x09\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_method_ref_with_0000_and_0000() {
        assert_deserialize(Constant::MethodRef {
            class: ConstantIndex(0),
            name_and_type: ConstantIndex(0),
        }, b"\x0a\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_method_ref_with_abcd_and_1234() {
        assert_deserialize(Constant::MethodRef {
            class: ConstantIndex(0xabcd),
            name_and_type: ConstantIndex(0x1234),
        }, b"\x0a\xab\xcd\x12\x34");
    }

    #[test]
    fn test_deserialize_method_ref_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x0a");
    }

    #[test]
    fn test_deserialize_method_ref_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x0a\x00");
    }

    #[test]
    fn test_deserialize_method_ref_premature_termination_3() {
        assert_eof(Constant::deserialize, b"\x0a\x00\x00");
    }

    #[test]
    fn test_deserialize_method_ref_premature_termination_4() {
        assert_eof(Constant::deserialize, b"\x0a\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_interface_method_ref_with_0000_and_0000() {
        assert_deserialize(Constant::InterfaceMethodRef {
            class: ConstantIndex(0),
            name_and_type: ConstantIndex(0),
        }, b"\x0b\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_interface_method_ref_with_abcd_and_1234() {
        assert_deserialize(Constant::InterfaceMethodRef {
            class: ConstantIndex(0xabcd),
            name_and_type: ConstantIndex(0x1234),
        }, b"\x0b\xab\xcd\x12\x34");
    }

    #[test]
    fn test_deserialize_interface_method_ref_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x0b");
    }

    #[test]
    fn test_deserialize_interface_method_ref_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x0b\x00");
    }

    #[test]
    fn test_deserialize_interface_method_ref_premature_termination_3() {
        assert_eof(Constant::deserialize, b"\x0b\x00\x00");
    }

    #[test]
    fn test_deserialize_interface_method_ref_premature_termination_4() {
        assert_eof(Constant::deserialize, b"\x0b\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_method_handle_of_type_get_field() {
        assert_method_handle(MethodHandle::GetField(ConstantIndex(0x1234)), b"\x0f\x01\x12\x34");
    }

    #[test]
    fn test_deserialize_method_handle_of_type_get_static() {
        assert_method_handle(MethodHandle::GetStatic(ConstantIndex(0x1f2b)), b"\x0f\x02\x1f\x2b");
    }

    #[test]
    fn test_deserialize_method_handle_of_type_put_field() {
        assert_method_handle(MethodHandle::PutField(ConstantIndex(0x1789)), b"\x0f\x03\x17\x89");
    }

    #[test]
    fn test_deserialize_method_handle_of_type_put_static() {
        assert_method_handle(MethodHandle::PutStatic(ConstantIndex(0xabcd)), b"\x0f\x04\xab\xcd");
    }

    #[test]
    fn test_deserialize_method_handle_of_type_invoke_virtual() {
        assert_method_handle(MethodHandle::InvokeVirtual(ConstantIndex(0x1337)), b"\x0f\x05\x13\x37");
    }

    #[test]
    fn test_deserialize_method_handle_of_type_invoke_static() {
        assert_method_handle(MethodHandle::InvokeStatic(ConstantIndex(0x8fc0)), b"\x0f\x06\x8f\xc0");
    }

    #[test]
    fn test_deserialize_method_handle_of_type_invoke_special() {
        assert_method_handle(MethodHandle::InvokeSpecial(ConstantIndex(0xcafe)), b"\x0f\x07\xca\xfe");
    }

    #[test]
    fn test_deserialize_method_handle_of_type_new_invoke_special() {
        assert_method_handle(MethodHandle::NewInvokeSpecial(ConstantIndex(0xbabe)), b"\x0f\x08\xba\xbe");
    }

    #[test]
    fn test_deserialize_method_handle_of_type_invoke_interface() {
        assert_method_handle(MethodHandle::InvokeInterface(ConstantIndex(0xbeef)), b"\x0f\x09\xbe\xef");
    }

    #[test]
    fn test_deserialize_method_handle_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x0f");
    }

    #[test]
    fn test_deserialize_method_handle_premature_termination_2_put_static() {
        assert_eof(Constant::deserialize, b"\x0f\x04");
    }

    #[test]
    fn test_deserialize_method_handle_premature_termination_2_invoke_virtual() {
        assert_eof(Constant::deserialize, b"\x0f\x05");
    }

    #[test]
    fn test_deserialize_method_handle_premature_termination_3_new_invoke_special() {
        assert_eof(Constant::deserialize, b"\x0f\x08\xff");
    }

    #[test]
    fn test_deserialize_method_handle_premature_termination_4_get_field() {
        assert_eof(Constant::deserialize, b"\x0f\x01\xab");
    }

    #[test]
    fn test_deserialize_method_handle_with_invalid_type_0x0a() {
        deserialize_expecting_error(Constant::deserialize, b"\x0f\x0a\xab\xcd", |err| match *err {
            ClassLoaderError::InvalidMethodHandleKind(kind) => assert_eq!(0x0a, kind),
            _ => panic!("Expected InvalidMethodHandleKind, but got {:#?}", err)
        });
    }

    #[test]
    fn test_deserialize_method_handle_with_invalid_type_0xff() {
        deserialize_expecting_error(Constant::deserialize, b"\x0f\xff\x12\x34", |err| match *err {
            ClassLoaderError::InvalidMethodHandleKind(kind) => assert_eq!(0xff, kind),
            _ => panic!("Expected InvalidMethodHandleKind, but got {:#?}", err)
        });
    }

    #[test]
    fn test_deserialize_method_handle_with_invalid_type_0x00() {
        deserialize_expecting_error(Constant::deserialize, b"\x0f\x00\x13\xf7", |err| match *err {
            ClassLoaderError::InvalidMethodHandleKind(kind) => assert_eq!(0x00, kind),
            _ => panic!("Expected InvalidMethodHandleKind, but got {:#?}", err)
        });
    }

    #[test]
    fn test_deserialize_method_type_with_index_0x0000() {
        assert_deserialize(Constant::MethodType(ConstantIndex(0x0000)), b"\x10\x00\x00");
    }

    #[test]
    fn test_deserialize_method_type_with_index_0x1234() {
        assert_deserialize(Constant::MethodType(ConstantIndex(0x1234)), b"\x10\x12\x34");
    }

    #[test]
    fn test_deserialize_method_type_with_index_0xffff() {
        assert_deserialize(Constant::MethodType(ConstantIndex(0xffff)), b"\x10\xff\xff");
    }

    #[test]
    fn test_deserialize_method_type_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x10");
    }

    #[test]
    fn test_deserialize_method_type_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x10\x5b");
    }

    #[test]
    fn test_deserialize_invoke_dynamic_info_with_indexes_0000_and_0000() {
        assert_deserialize(Constant::InvokeDynamicInfo {
            bootstrap_method_attr: MethodIndex(0),
            name_and_type: ConstantIndex(0),
        }, b"\x12\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_invoke_dynamic_info_with_indexes_abcd_and_1234() {
        assert_deserialize(Constant::InvokeDynamicInfo {
            bootstrap_method_attr: MethodIndex(0xabcd),
            name_and_type: ConstantIndex(0x1234),
        }, b"\x12\xab\xcd\x12\x34");
    }

    #[test]
    fn test_deserialize_invoke_dynamic_info_premature_termination_1() {
        assert_eof(Constant::deserialize, b"\x12");
    }

    #[test]
    fn test_deserialize_invoke_dynamic_info_premature_termination_2() {
        assert_eof(Constant::deserialize, b"\x12\x12");
    }

    #[test]
    fn test_deserialize_invoke_dynamic_info_premature_termination_3() {
        assert_eof(Constant::deserialize, b"\x12\x12\x34");
    }

    #[test]
    fn test_deserialize_invoke_dynamic_info_premature_termination_4() {
        assert_eof(Constant::deserialize, b"\x12\x12\x34\x56");
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_integer() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::Integer(42)];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_integer_2() {
        // Here the constant pool does contain a valid type name, but the Attribute object
        // instead points at the Integer in the pool instead.
        let bytes = b"\x00\x02\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("ConstantValue".to_string()), Constant::Integer(42)];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_float() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::Float(7.0)];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_long() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::Long(1337)];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_double() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::Double(14.0)];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_class_ref() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::ClassRef(ConstantIndex(0))];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_string_ref() {
        // Attribute types should be Utf8. Here the type is a String that points to a Utf8, which
        // is not permitted.
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::StringRef(ConstantIndex(2)), Constant::Utf8("ConstantRef".to_string())];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_field_ref() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::FieldRef{class: ConstantIndex(0), name_and_type: ConstantIndex(0)}];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_method_ref() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::MethodRef{class: ConstantIndex(0), name_and_type: ConstantIndex(0)}];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_interface_method_ref() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::InterfaceMethodRef{class: ConstantIndex(0), name_and_type: ConstantIndex(0)}];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_name_and_type_ref() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::NameAndTypeRef{name: ConstantIndex(0), descriptor: ConstantIndex(0)}];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_method_handle_get_field() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::MethodHandleRef(MethodHandle::GetField(ConstantIndex(0)))];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_method_handle_put_static() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::MethodHandleRef(MethodHandle::PutStatic(ConstantIndex(0)))];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_method_type() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::MethodType(ConstantIndex(0))];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_invoke_dynamic_info() {
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::InvokeDynamicInfo{bootstrap_method_attr: MethodIndex(0), name_and_type: ConstantIndex(0)}];
        assert_invalid_attribute_type(bytes, &constants);
    }

    #[test]
    fn test_deserialize_attribute_where_type_ref_is_dummy() {
        // Here we're testing that an error is thrown if the attribute type index points into the
        // middle of a Long or Double value (these take up two consecutive slots in the constant
        // pool)
        let bytes = b"\x00\x01\x00\x00\x00\x00";
        let constants = vec![Constant::Dummy];
        deserialize_with_constants_expecting_error(Attribute::deserialize, bytes, &constants, |err| match *err {
            ClassLoaderError::InvalidConstantRef(_) => (),
            _ => panic!("Expected invalid constant index; got {:#?}", err),
        });
    }

    #[test]
    fn test_deserialize_attribute_ending_before_type_ref() {
        assert_eof_with_constants(Attribute::deserialize, b"", &vec![]);
    }

    #[test]
    fn test_deserialize_attribute_ending_during_type_ref() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00", &vec![]);
    }

    #[test]
    fn test_deserialize_constant_attribute_at_0x0001_and_0x0002() {
        let bytes = b"\x00\x01\x00\x00\x00\x02\x00\x02";
        let constants = vec![Constant::Utf8("ConstantValue".to_string())];
        let expected = Attribute::ConstantValue {
            attribute_name: ConstantIndex(0x0001),
            constant_value: ConstantIndex(0x0002),
        };

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_constant_attribute_at_0x1234_and_0x5678() {
        let bytes = b"\x12\x34\x00\x00\x00\x02\x56\x78";
        let mut constants = vec![];
        for _ in 0..0x1233 {
            constants.push(Constant::Integer(4));
        }

        constants.push(Constant::Utf8("ConstantValue".to_string()));
        let expected = Attribute::ConstantValue {
            attribute_name: ConstantIndex(0x1234),
            constant_value: ConstantIndex(0x5678),
        };

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_constant_attribute_premature_termination_1() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01",
            &vec![Constant::Utf8("ConstantValue".to_string())]
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_premature_termination_2() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00",
            &vec![Constant::Utf8("ConstantValue".to_string())]
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_premature_termination_3() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00",
            &vec![Constant::Utf8("ConstantValue".to_string())]
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_premature_termination_4() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00",
            &vec![Constant::Utf8("ConstantValue".to_string())]
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_premature_termination_5() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x02",
            &vec![Constant::Utf8("ConstantValue".to_string())]
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_premature_termination_6() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x02\xff",
            &vec![Constant::Utf8("ConstantValue".to_string())]
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_with_length_0() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x00\xab\xcb",
            &vec![Constant::Utf8("ConstantValue".to_string())],
            |_| ()  // Allow any error here
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_with_length_1() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x01\xab\xcb",
            &vec![Constant::Utf8("ConstantValue".to_string())],
            |_| ()  // Allow any error here
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_with_length_3() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x03\xab\xcb",
            &vec![Constant::Utf8("ConstantValue".to_string())],
            |_| ()  // Allow any error here
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_with_maximum_length() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\xff\xff\xff\xff\xab\xcb",
            &vec![Constant::Utf8("ConstantValue".to_string())],
            |_| ()  // Allow any error here
        );
    }


    fn do_float_test(float_bits: u32, input: &[u8]) {
        assert_deserialize(Constant::Float(f32::from_bits(float_bits)), input);
    }

    fn do_double_test(double_bits: u64, input: &[u8]) {
        assert_deserialize(Constant::Double(f64::from_bits(double_bits)), input);
    }

    fn assert_method_handle(handle: MethodHandle, input: &[u8]) {
        assert_deserialize(Constant::MethodHandleRef(handle), input);
    }

    fn assert_deserialize<D: Deserialize+Debug+PartialEq>(expected: D, input: &[u8]) {
        assert_eq!(Ok(expected), D::deserialize(&mut bytes::Bytes::from(input).into_buf()));
    }

    fn assert_deserialize_with_constants<D: DeserializeWithConstants+Debug+PartialEq>(expected: D, input: &[u8], constants: &Vec<Constant>) {
        assert_eq!(Ok(expected), D::deserialize(&mut bytes::Bytes::from(input).into_buf(), constants));
    }

    fn assert_eof<D: Deserialize+Debug, F> (deserializer: F, input: &[u8])
        where F: Fn(&mut bytes::Buf) -> Result<D, ClassLoaderError> {
            deserialize_expecting_error(deserializer, input, |err| match *err {
                ClassLoaderError::Eof(_) => (),
                _ => panic!("Expected EOF, but got {:#?}", err),
            });
    }

    fn assert_eof_with_constants<D: DeserializeWithConstants+Debug, F> (deserializer: F, input: &[u8], constants: &Vec<Constant>)
        where F: Fn(&mut bytes::Buf, &Vec<Constant>) -> Result<D, ClassLoaderError> {
            deserialize_with_constants_expecting_error(deserializer, input, constants, |err| match *err {
                ClassLoaderError::Eof(_) => (),
                _ => panic!("Expected EOF, but got {:#?}", err),
            });
    }

    fn assert_invalid_attribute_type(input: &[u8], constants: &Vec<Constant>) {
        deserialize_with_constants_expecting_error(Attribute::deserialize, input, constants, |err| match *err {
            ClassLoaderError::InvalidAttributeType(_) => (),
            _ => panic!("Expected InvalidAttributeType, but got {:#?}", err),
        });
    }

    fn assert_invalid_utf8(input: &[u8]) {
        deserialize_expecting_error(Constant::deserialize, input, |err| match *err {
            ClassLoaderError::Utf8(_) => (),
            _ => panic!("Expected Utf8 parse error, but got {:#?}", err),
        });
    }

    fn deserialize_expecting_error<D: Deserialize+fmt::Debug, F, G>(deserializer: F, input: &[u8], handler: G) where
        F: Fn(&mut bytes::Buf) -> Result<D, ClassLoaderError>,
        G: Fn(&ClassLoaderError) {
            let res = deserializer(&mut bytes::Bytes::from(input).into_buf());
            match res {
                Ok(ref res) => panic!("Expected error, but got result {:#?}", res),
                Err(ref err) => handler(&err),
            }
    }

    fn deserialize_with_constants_expecting_error<D: DeserializeWithConstants+Debug, F, G>(deserializer: F, input: &[u8], constants: &Vec<Constant>, handler: G) where
        F: Fn(&mut bytes::Buf, &Vec<Constant>) -> Result<D, ClassLoaderError>,
        G: Fn(&ClassLoaderError) {
            let res = deserializer(&mut bytes::Bytes::from(input).into_buf(), constants);
            match res{
                Ok(ref res) => panic!("Expected error, but got result {:#?}", res),
                Err(ref err) => handler(&err),
            }
    }
}
