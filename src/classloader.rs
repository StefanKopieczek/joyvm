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

macro_rules! require {
    // E.g: require! my_data has 4 bytes for "attribute length"
    ($data:tt has $required:tt bytes for $context:tt) => {{
        if $data.remaining() < $required {
            return Err(ClassLoaderError::Eof(format!("Unexpected end of stream while parsing {}", $context.to_string())));
        }
    }};
    ($data:tt has 1 byte for $context:tt) => {{
        if $data.remaining() == 0 {
            return Err(ClassLoaderError::Eof(format!("Unexpected end of stream while parsing {}", $context.to_string())));
        }
    }};
}

impl Deserialize for Constant {
    fn deserialize(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
        require!(data has 1 byte for "constant tag");
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
    require!(data has 2 bytes for "length field of Utf8 constant");
    let length = data.get_u16_be() as usize;

    require!(data has length bytes for "Utf8 constant");
    let mut contents = vec![0; length as usize];
    data.copy_to_slice(&mut contents);

    str::from_utf8(&contents)
        .map(|slice| Constant::Utf8(slice.to_string()))
        .map_err(|err| ClassLoaderError::Utf8(err))
}

fn deserialize_integer(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    require!(data has 4 bytes for "Integer constant");
    Ok(Constant::Integer(data.get_u32_be()))
}

fn deserialize_float(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    require!(data has 4 bytes for "Float constant");
    Ok(Constant::Float(data.get_f32_be()))
}

fn deserialize_long(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    require!(data has 8 bytes for "Long constant");
    Ok(Constant::Long(data.get_u64_be()))
}

fn deserialize_double(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    require!(data has 8 bytes for "Double constant");
    Ok(Constant::Double(data.get_f64_be()))
}

fn deserialize_classref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    deserialize_constant_index(data).map(Constant::ClassRef)
}

fn deserialize_string(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    deserialize_constant_index(data).map(Constant::StringRef)
}

fn deserialize_fieldref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    let class = deserialize_constant_index(data)?;
    let name_and_type = deserialize_constant_index(data)?;
    Ok(Constant::FieldRef {class: class, name_and_type: name_and_type})
}

fn deserialize_methodref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    let class = deserialize_constant_index(data)?;
    let name_and_type = deserialize_constant_index(data)?;
    Ok(Constant::MethodRef {class: class, name_and_type: name_and_type})
}

fn deserialize_interface_method_ref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    let class = deserialize_constant_index(data)?;
    let name_and_type = deserialize_constant_index(data)?;
    Ok(Constant::InterfaceMethodRef {class: class, name_and_type: name_and_type})
}

fn deserialize_method_handle_ref(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    require!(data has 1 byte for "method handle ref kind");
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

    handle.map(|h| Constant::MethodHandleRef(h))
}

fn deserialize_method_type(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    Ok(Constant::MethodType(deserialize_constant_index(data)?))
}

fn deserialize_invoke_dynamic_info(data: &mut bytes::Buf) -> Result<Constant, ClassLoaderError> {
    Ok(Constant::InvokeDynamicInfo{
        bootstrap_method_attr: deserialize_method_index(data)?,
        name_and_type: deserialize_constant_index(data)?,
    })
}

fn deserialize_constant_index(data: &mut bytes::Buf) -> Result<ConstantIndex, ClassLoaderError> {
    require!(data has 2 bytes for "constant index");
    Ok(ConstantIndex(data.get_u16_be()))
}

fn deserialize_method_index(data: &mut bytes::Buf) -> Result<MethodIndex, ClassLoaderError> {
    require!(data has 2 bytes for "method index");
    Ok(MethodIndex(data.get_u16_be()))
}

impl DeserializeWithConstants for Attribute {
    fn deserialize(data: &mut bytes::Buf, constants: &Vec<Constant>) -> Result<Attribute, ClassLoaderError> {
        let attribute_type_index = deserialize_constant_index(data)?;
        let attribute_type_ref = attribute_type_index.lookup(constants)?;
        let attribute_type = match *attribute_type_ref {
            Constant::Utf8(ref attr_type) => Ok(attr_type),
            _ => Err(ClassLoaderError::InvalidAttributeType(attribute_type_ref.clone())),
        }?;

        require!(data has 4 bytes for "attribute length");
        let length = data.get_u32_be();

        match attribute_type.as_ref() {
            "ConstantValue" => deserialize_constant_value(attribute_type_index, length, data),
            "Code" => deserialize_code(attribute_type_index, constants, length, data),
            "StackMapTable" => deserialize_stack_map_table(attribute_type_index, length, data),
            _ => Err(ClassLoaderError::UnknownAttributeType(attribute_type.to_string()))
        }
    }
}

fn deserialize_constant_value(attribute_name: ConstantIndex, length: u32, data: &mut bytes::Buf) -> Result<Attribute, ClassLoaderError> {
    if length == 2 {
        Ok(Attribute::ConstantValue {
            attribute_name: attribute_name,
            constant_value: deserialize_constant_index(data)?,
        })
    } else {
        Err(ClassLoaderError::LengthMismatch {
            context: "ConstantValue attribute".to_string(),
            stated_length: length,
            inferred_length: 2
        })
    }
}

fn deserialize_code(attribute_name: ConstantIndex, constants: &Vec<Constant>, declared_length: u32, data: &mut bytes::Buf) -> Result<Attribute, ClassLoaderError> {
    let initial_bytes_remaining = data.remaining();

    require!(data has 2 bytes for "Code attribute max stack size");
    let max_stack = data.get_u16_be();

    require!(data has 2 bytes for "Code attribute max locals count");
    let max_locals = data.get_u16_be();

    require!(data has 4 bytes for "Code attribute inner length");
    let code_length = data.get_u32_be() as usize;

    require!(data has code_length bytes for "Code attribute code body");
    let mut code = vec![0; code_length];
    for idx in 0..code_length {
        code[idx] = data.get_u8();
    }

    require!(data has 2 bytes for "Code attribute exception table length");
    let exception_row_count = data.get_u16_be() as usize;
    let exception_table = deserialize_multiple(exception_row_count, data)?;

    require!(data has 2 bytes for "Code attribute subattribute count");
    let attributes_count = data.get_u16_be() as usize;
    let attributes = deserialize_multiple_with_constants(attributes_count, data, constants)?;

    let actual_length = (initial_bytes_remaining - data.remaining()) as u32;
    if actual_length != declared_length {
        return Err(ClassLoaderError::LengthMismatch {
            context: "Code attribute".to_string(),
            stated_length: declared_length,
            inferred_length: actual_length,
        });
    }

    Ok(Attribute::Code {
        attribute_name: attribute_name,
        max_stack: max_stack,
        max_locals: max_locals,
        code: code,
        exception_table: exception_table,
        attributes: attributes,
    })
}

fn deserialize_stack_map_table(attribute_name: ConstantIndex, declared_length: u32, data: &mut bytes::Buf) -> Result<Attribute, ClassLoaderError> {
    let initial_bytes_remaining = data.remaining();

    require!(data has 2 bytes for "stack map table entry count");
    let num_entries = data.get_u16_be() as usize;
    let entries = deserialize_multiple(num_entries, data)?;

    let actual_length = (initial_bytes_remaining - data.remaining()) as u32;
    if actual_length != declared_length {
        return Err(ClassLoaderError::LengthMismatch {
            context: "StackMapTable attribute".to_string(),
            stated_length: declared_length,
            inferred_length: actual_length,
        });
    }

    return Ok(Attribute::StackMapTable {
        attribute_name: attribute_name,
        entries: entries,
    });
}

impl Deserialize for ExceptionTableRow {
    fn deserialize(data: &mut bytes::Buf) -> Result<ExceptionTableRow, ClassLoaderError> {
        require!(data has 8 bytes for "exception table row");
        Ok(ExceptionTableRow {
            start_pc: data.get_u16_be(),
            end_pc: data.get_u16_be(),
            handler_pc: data.get_u16_be(),
            catch_type: deserialize_constant_index(data)?,
        })
    }
}

impl Deserialize for StackMapFrame {
    fn deserialize(data: &mut bytes::Buf) -> Result<StackMapFrame, ClassLoaderError> {
        require!(data has 1 byte for "stack map frame type");
        let frame_type = data.get_u8();
        match frame_type {
            0...63 => Ok(StackMapFrame::SameFrame{offset_delta: frame_type}),
            64...127 => Ok(StackMapFrame::SameLocalsOneStackItemFrame {
                offset_delta: frame_type - 64,
                stack_item: VerificationType::deserialize(data)?,
            }),
            247 => {
                require!(data has 2 bytes for "extended stack frame offset");
                Ok(StackMapFrame::SameLocalsOneStackItemFrameExtended {
                    offset_delta: data.get_u16_be(),
                    stack_item: VerificationType::deserialize(data)?,
                })
            },
            248...250 => {
                require!(data has 2 bytes for "chop frame offset");
                Ok(StackMapFrame::ChopFrame {
                    offset_delta: data.get_u16_be(),
                    num_absent_locals: (251 - frame_type),
                })
            },
            251 => {
                require!(data has 2 bytes for "extended same-frame stack frame offset");
                Ok(StackMapFrame::SameFrameExtended {
                    offset_delta: data.get_u16_be(),
                })
            },
            252...254 => {
                require!(data has 2 bytes for "append frame offset");
                let offset_delta = data.get_u16_be();

                let num_locals = (frame_type - 251) as usize;
                let locals = deserialize_multiple(num_locals, data)?;

                Ok(StackMapFrame::AppendFrame {
                    offset_delta: offset_delta,
                    new_locals: locals,
                })
            },
            255 => {
                require!(data has 2 bytes for "full stack frame offset");
                let offset_delta = data.get_u16_be();

                require!(data has 2 bytes for "full stack frame locals count");
                let num_locals = data.get_u16_be() as usize;
                let locals = deserialize_multiple(num_locals, data)?;

                require!(data has 2 bytes for "full stack frame stack item count");
                let num_stack_items = data.get_u16_be() as usize;
                let stack_items = deserialize_multiple(num_stack_items, data)?;

                Ok(StackMapFrame::FullFrame {
                    offset_delta: offset_delta,
                    locals: locals,
                    stack_items: stack_items,
                })
            },
            _ => Err(ClassLoaderError::InvalidStackFrameType(frame_type)),
        }
    }
}

impl Deserialize for VerificationType {
    fn deserialize(data: &mut bytes::Buf) -> Result<VerificationType, ClassLoaderError> {
        require!(data has 1 byte for "verification type identifier");
        let type_id = data.get_u8();
        match type_id {
            0 => Ok(VerificationType::Top),
            1 => Ok(VerificationType::Integer),
            2 => Ok(VerificationType::Float),
            3 => Ok(VerificationType::Double),
            4 => Ok(VerificationType::Long),
            5 => Ok(VerificationType::Null),
            6 => Ok(VerificationType::UninitializedThis),
            7 => Ok(VerificationType::Object(deserialize_constant_index(data)?)),
            8 => {
                require!(data has 2 bytes for "uninitialized variable offset");
                Ok(VerificationType::Uninitialized(data.get_u16_be()))
            },
            _ => Err(ClassLoaderError::InvalidVerificationType(type_id)),
        }
    }
}

fn deserialize_multiple<D: Deserialize>(count: usize, data: &mut bytes::Buf) -> Result<Vec<D>, ClassLoaderError> {
    let mut res = vec![];
    for _ in 0..count {
        res.push(D::deserialize(data)?);
    }

    Ok(res)
}

fn deserialize_multiple_with_constants<D: DeserializeWithConstants>(count: usize, data: &mut bytes::Buf, constants: &Vec<Constant>) -> Result<Vec<D>, ClassLoaderError> {
    let mut res = vec![];
    for _ in 0..count {
        res.push(D::deserialize(data, constants)?);
    }

    Ok(res)
}

#[derive(Debug, PartialEq)]
pub enum ClassLoaderError {
    Utf8(str::Utf8Error),
    Eof(String),
    InvalidConstantRef(ConstantLookupError),
    InvalidConstantType(u8),
    InvalidMethodHandleKind(u8),
    InvalidAttributeType(Constant),
    InvalidStackFrameType(u8),
    InvalidVerificationType(u8),
    LengthMismatch{context: String, stated_length: u32, inferred_length: u32},
    Misc(String),
    UnknownAttributeType(String),
}

impl std::convert::From<ConstantLookupError> for ClassLoaderError {
    fn from(cause: ConstantLookupError) -> ClassLoaderError {
        ClassLoaderError::InvalidConstantRef(cause)
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
            ClassLoaderError::InvalidVerificationType(ref verification_type_tag) => write!(f, "Invalid verification type tag {:#?}", verification_type_tag),
            ClassLoaderError::InvalidStackFrameType(ref frame_type) => write!(f, "Invalid stack frame type {:#?}", frame_type),
            ClassLoaderError::LengthMismatch{ref context, ref stated_length, ref inferred_length} =>
                write!(f, "Stated length of {} disagrees with inferred length. Inferred length: {}; stated length: {}", context, inferred_length, stated_length),
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
            ClassLoaderError::InvalidVerificationType(..) => "Invalid verification type",
            ClassLoaderError::InvalidStackFrameType(..) => "Invalid stack frame type",
            ClassLoaderError::LengthMismatch{..} => "Stated length of entity disagrees with inferred length",
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
            ClassLoaderError::InvalidVerificationType(..) => None,
            ClassLoaderError::InvalidStackFrameType(..) => None,
            ClassLoaderError::LengthMismatch{..} => None,
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
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected LengthMismatch; got {}", err),
            }
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_with_length_1() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x01\xab\xcb",
            &vec![Constant::Utf8("ConstantValue".to_string())],
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected LengthMismatch; got {}", err),
            }
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_with_length_3() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x03\xab\xcb",
            &vec![Constant::Utf8("ConstantValue".to_string())],
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected LengthMismatch; got {}", err),
            }
        );
    }

    #[test]
    fn test_deserialize_constant_attribute_with_maximum_length() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\xff\xff\xff\xff\xab\xcb",
            &vec![Constant::Utf8("ConstantValue".to_string())],
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected LengthMismatch; got {}", err),
            }
        );
    }

    #[test]
    fn test_deserialize_exception_table_row_valid_1() {
        assert_deserialize(ExceptionTableRow {
            start_pc: 0,
            end_pc: 0,
            handler_pc: 0,
            catch_type: ConstantIndex(0),
        }, b"\x00\x00\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_exception_table_row_valid_2() {
        assert_deserialize(ExceptionTableRow {
            start_pc: 0x1234,
            end_pc: 0x5678,
            handler_pc: 0x9abc,
            catch_type: ConstantIndex(0xdef0),
        }, b"\x12\x34\x56\x78\x9a\xbc\xde\xf0");
    }

    #[test]
    fn test_deserialize_exception_table_row_premature_termination_1() {
        assert_eof(ExceptionTableRow::deserialize, b"\x12");
    }

    #[test]
    fn test_deserialize_exception_table_row_premature_termination_2() {
        assert_eof(ExceptionTableRow::deserialize, b"\x12\x34");
    }

    #[test]
    fn test_deserialize_exception_table_row_premature_termination_3() {
        assert_eof(ExceptionTableRow::deserialize, b"\x12\x34\x56");
    }

    #[test]
    fn test_deserialize_exception_table_row_premature_termination_4() {
        assert_eof(ExceptionTableRow::deserialize, b"\x12\x34\x56\x78");
    }

    #[test]
    fn test_deserialize_exception_table_row_premature_termination_5() {
        assert_eof(ExceptionTableRow::deserialize, b"\x12\x34\x56\x78\x9a");
    }

    #[test]
    fn test_deserialize_exception_table_row_premature_termination_6() {
        assert_eof(ExceptionTableRow::deserialize, b"\x12\x34\x56\x78\x9a\xbc");
    }

    #[test]
    fn test_deserialize_exception_table_row_premature_termination_7() {
        assert_eof(ExceptionTableRow::deserialize, b"\x12\x34\x56\x78\x9a\xbc\xde");
    }

    #[test]
    fn test_deserialize_trivial_code_block() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0,
            code: vec![],
            exception_table: vec![],
            attributes: vec![]
        };

        let bytes = b"\x00\x01\x00\x00\x00\x0c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_attribute_name_at_index_2() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(2),
            max_stack: 0,
            max_locals: 0,
            code: vec![],
            exception_table: vec![],
            attributes: vec![]
        };

        let bytes = b"\x00\x02\x00\x00\x00\x0c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("Constant".to_string()), Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_max_stack_1() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 1,
            max_locals: 0,
            code: vec![],
            exception_table: vec![],
            attributes: vec![]
        };

        let bytes = b"\x00\x01\x00\x00\x00\x0c\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_max_stack_ffff() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0xffff,
            max_locals: 0,
            code: vec![],
            exception_table: vec![],
            attributes: vec![]
        };

        let bytes = b"\x00\x01\x00\x00\x00\x0c\xff\xff\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_max_locals_1() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 1,
            code: vec![],
            exception_table: vec![],
            attributes: vec![]
        };

        let bytes = b"\x00\x01\x00\x00\x00\x0c\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_max_locals_ffff() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0xffff,
            code: vec![],
            exception_table: vec![],
            attributes: vec![]
        };

        let bytes = b"\x00\x01\x00\x00\x00\x0c\x00\x00\xff\xff\x00\x00\x00\x00\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_one_byte_of_code() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0,
            code: vec![0xcd],
            exception_table: vec![],
            attributes: vec![]
        };

        let bytes = b"\x00\x01\x00\x00\x00\x0d\x00\x00\x00\x00\x00\x00\x00\x01\xcd\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_ten_bytes_of_code() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0,
            code: vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa],
            exception_table: vec![],
            attributes: vec![]
        };

        let bytes = b"\x00\x01\x00\x00\x00\x16\x00\x00\x00\x00\x00\x00\x00\x0a\x11\x22\x33\x44\x55\x66\x77\x88\x99\xaa\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    #[ignore] // Takes a couple of minutes on my MBP 2018, so leaving ignored for now
    fn test_deserialize_code_with_large_code_body() {
        // Testing the maximum possible code body would take 4GB of memory, so we will settle for
        // testing a body that requires four bytes to hold the size.
        let mut code : Vec<u8> = vec![0; 0x01fffff3];
        for idx in 0..0x01fffff3 {
            // Arbitrary choice of bytes to fill up the vector
            code[idx] = ((idx as u16) % 256) as u8
        }

        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0,
            code: code.to_vec(),
            exception_table: vec![],
            attributes: vec![]
        };

        let mut bytes  = vec![];
        bytes.append(&mut b"\x00\x01\x01\xff\xff\xff\x00\x00\x00\x00\x01\xff\xff\xf3".to_vec());
        bytes.append(&mut code);
        bytes.append(&mut b"\x00\x00\x00\x00".to_vec());
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, &bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_one_exception_table_row() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0,
            code: vec![],
            exception_table: vec![
                ExceptionTableRow{start_pc: 0, end_pc:0, handler_pc: 0, catch_type: ConstantIndex(0)},
            ],
            attributes: vec![]
        };

        let bytes = b"\x00\x01\x00\x00\x00\x14\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_nontrivial_exception_table_row() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0,
            code: vec![],
            exception_table: vec![
                ExceptionTableRow{start_pc: 0xabcd, end_pc:0xcdef, handler_pc: 0xef12, catch_type: ConstantIndex(0x1234)},
            ],
            attributes: vec![]
        };

        let bytes = b"\x00\x01\x00\x00\x00\x14\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\xab\xcd\xcd\xef\xef\x12\x12\x34\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }
    #[test]
    fn test_deserialize_code_with_65536_exception_table_rows() {
        let mut exception_table = vec![];
        for row in 0..0xffff as u16 {
            exception_table.push(ExceptionTableRow {
                // Values here are chosen arbitrarily to make rows distinct.
                start_pc: row as u16,
                end_pc: row.wrapping_add(1) as u16,
                handler_pc: row.wrapping_add(2) as u16,
                catch_type: ConstantIndex(row.wrapping_add(3) as u16),
            });
        }

        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0,
            code: vec![],
            exception_table: exception_table,
            attributes: vec![]
        };

        let mut bytes = b"\x00\x01\x00\x08\x00\x04\x00\x00\x00\x00\x00\x00\x00\x00\xff\xff".to_vec();
        for row in 0..0xffff as u16 {
            // Bit wrangling to produce ExceptionTableRow data that matches the contents of
            // exception_table as defined at the start of this function.
            bytes.push((row >> 8) as u8);
            bytes.push(row as u8);
            bytes.push((row.wrapping_add(1) >> 8) as u8);
            bytes.push((row.wrapping_add(1) & 0x00ff) as u8);
            bytes.push((row.wrapping_add(2) >> 8) as u8);
            bytes.push((row.wrapping_add(2) & 0x00ff) as u8);
            bytes.push((row.wrapping_add(3) >> 8) as u8);
            bytes.push((row.wrapping_add(3) & 0x00ff) as u8);
        }
        bytes.push(0x00);
        bytes.push(0x00);

        let constants = vec![Constant::Utf8("Code".to_string())];
        assert_deserialize_with_constants(expected, &bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_one_attribute() {
        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0,
            code: vec![],
            exception_table: vec![],
            attributes: vec![Attribute::ConstantValue{attribute_name: ConstantIndex(2), constant_value: ConstantIndex(0)}],
        };

        let bytes = b"\x00\x01\x00\x00\x00\x14\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x02\x00\x00\x00\x02\x00\x00";
        let constants = vec![Constant::Utf8("Code".to_string()), Constant::Utf8("ConstantValue".to_string())];

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_with_65536_attributes() {
        let mut attributes = vec![];
        for idx in 0..0xffff {
            attributes.push(Attribute::ConstantValue {
                attribute_name: ConstantIndex(2),
                constant_value: ConstantIndex(idx as u16), // Arbitrary choice so the constants are different
            });
        }

        let expected = Attribute::Code {
            attribute_name: ConstantIndex(1),
            max_stack: 0,
            max_locals: 0,
            code: vec![],
            exception_table: vec![],
            attributes: attributes,
        };

        let mut bytes = b"\x00\x01\x00\x08\x00\x04\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xff\xff".to_vec();
        for idx in 0..0xffff as u32 {
            let value_index = idx % 0x10000;
            bytes.append(&mut b"\x00\x02\x00\x00\x00\x02".to_vec());
            bytes.push((value_index >> 8) as u8);
            bytes.push(value_index as u8);
        }

        let constants = vec![Constant::Utf8("Code".to_string()), Constant::Utf8("ConstantValue".to_string())];

        assert_deserialize_with_constants(expected, &bytes, &constants);
    }

    #[test]
    fn test_deserialize_code_terminating_before_attribute_length_dword() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_attribute_length_dword_1() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_attribute_length_dword_2() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_attribute_length_dword_3() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_before_max_stack() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_max_stack() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x01\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_before_max_locals() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x02\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_max_locals() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x03\x00\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_before_code_length() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x04\x00\x00\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_code_length_1() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x05\x00\x00\x00\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_code_length_2() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x06\x00\x00\x00\x00\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_code_length_3() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x07\x00\x00\x00\x00\x00\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_code_1() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x08\x00\x00\x00\x00\x00\x00\x00\x01", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_code_2() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x02\xff", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_code_3() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x0a\x00\x00\x00\x00\x00\x00\x00\x03\xff\xff", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_before_exception_table_length() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x00", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_exception_table_length() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x0a\x00\x00\x00\x00\x00\x00\x00\x00\xff", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_before_exception_table() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x0b\x00\x00\x00\x00\x00\x00\x00\x00\xff\xff", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_exception_table_inside_row() {
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x0d\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x12\x34", &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_exception_table_in_between_rows() {
        // Declare that there are two exception rows, but EOF after the first
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x13\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x12\x34\x56\x78\x9a\xbc\xde\xf0",
            &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_before_attribute_count() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0b\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_during_attribute_count() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xff",
            &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_before_attributes() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0d\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xff\xff",
            &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_inside_attribute() {
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0e\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x01",
            &vec![Constant::Utf8("Code".to_string())]);
    }

    #[test]
    fn test_deserialize_code_terminating_between_attributes() {
        // Declare that the Code attribute has two subattributes, but EOF after the first one.
        assert_eof_with_constants(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x15\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x02\x00\x00\x00\x02\x00\x00",
            &vec![Constant::Utf8("Code".to_string()), Constant::Utf8("ConstantValue".to_string())]);
    }

    #[test]
    fn test_deserializing_trivial_code_with_declared_length_of_0_throws_error() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            &vec![Constant::Utf8("Code".to_string())],
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected length mismatch; got {}", err),
            });
    }

    #[test]
    fn test_deserializing_trivial_code_with_declared_length_of_0x0b_throws_error() {
        // The correct length is 0x0c
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0b\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            &vec![Constant::Utf8("Code".to_string())],
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected length mismatch; got {}", err),
            });
    }

    #[test]
    fn test_deserializing_trivial_code_with_declared_length_of_0x0d_throws_error() {
        // The correct length is 0x0c
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0d\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            &vec![Constant::Utf8("Code".to_string())],
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected length mismatch; got {}", err),
            });
    }

    #[test]
    fn test_deserializing_code_with_nonzero_body_and_declared_length_of_0x0c_throws_error() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0c\x00\x00\x00\x00\x00\x00\x00\x01\xff\x00\x00\x00\x00",
            &vec![Constant::Utf8("Code".to_string())],
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected length mismatch; got {}", err),
            });
    }

    #[test]
    fn test_deserializing_code_with_exception_table_and_declared_length_of_0x0c_throws_error() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            &vec![Constant::Utf8("Code".to_string())],
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected length mismatch; got {}", err),
            });
    }

    #[test]
    fn test_deserializing_code_with_attribute_and_declared_length_of_0x0c_throws_error() {
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x02\x00\x00\x00\x02\x00\x00",
            &vec![Constant::Utf8("Code".to_string()), Constant::Utf8("ConstantValue".to_string())],
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Expected length mismatch; got {}", err),
            });
    }

    #[test]
    fn test_deserializing_verification_type_top() {
        assert_deserialize(VerificationType::Top, b"\x00");
    }

    #[test]
    fn test_deserializing_verification_type_throws_error_if_buffer_is_empty() {
        assert_eof(VerificationType::deserialize, b"");
    }

    #[test]
    fn test_deserializing_verification_type_integer() {
        assert_deserialize(VerificationType::Integer, b"\x01");
    }

    #[test]
    fn test_deserializing_verification_type_float() {
        assert_deserialize(VerificationType::Float, b"\x02");
    }

    #[test]
    fn test_deserializing_verification_type_long() {
        assert_deserialize(VerificationType::Long, b"\x04");
    }

    #[test]
    fn test_deserializing_verification_type_double() {
        assert_deserialize(VerificationType::Double, b"\x03");
    }

    #[test]
    fn test_deserializing_verification_type_null() {
        assert_deserialize(VerificationType::Null, b"\x05");
    }

    #[test]
    fn test_deserializing_verification_type_uninitialized_this() {
        assert_deserialize(VerificationType::UninitializedThis, b"\x06");
    }

    #[test]
    fn test_deserializing_verification_type_object_at_index_0() {
        assert_deserialize(VerificationType::Object(ConstantIndex(0)), b"\x07\x00\x00");
    }

    #[test]
    fn test_deserializing_verification_type_object_at_index_1() {
        assert_deserialize(VerificationType::Object(ConstantIndex(1)), b"\x07\x00\x01");
    }

    #[test]
    fn test_deserializing_verification_type_object_at_index_0xffff() {
        assert_deserialize(VerificationType::Object(ConstantIndex(0xffff)), b"\x07\xff\xff");
    }

    #[test]
    fn test_deserializing_verification_type_object_premature_termination_1() {
        assert_eof(VerificationType::deserialize, b"\x07");
    }

    #[test]
    fn test_deserializing_verification_type_object_premature_termination_2() {
        assert_eof(VerificationType::deserialize, b"\x07\x00");
    }

    #[test]
    fn test_deserializing_verification_type_uninitialized_with_offset_0() {
        assert_deserialize(VerificationType::Uninitialized(0), b"\x08\x00\x00");
    }

    #[test]
    fn test_deserializing_verification_type_uninitialized_with_offset_1() {
        assert_deserialize(VerificationType::Uninitialized(1), b"\x08\x00\x01");
    }

    #[test]
    fn test_deserializing_verification_type_uninitialized_with_offset_0xffff() {
        assert_deserialize(VerificationType::Uninitialized(0xffff), b"\x08\xff\xff");
    }

    #[test]
    fn test_deserializing_verification_type_uninitialized_premature_termination_1() {
        assert_eof(VerificationType::deserialize, b"\x08");
    }

    #[test]
    fn test_deserializing_verification_type_uninitialized_premature_termination_2() {
        assert_eof(VerificationType::deserialize, b"\x08\xff");
    }

    #[test]
    fn test_verification_type_9_is_invalid() {
        deserialize_expecting_error(VerificationType::deserialize, b"\x09", |err| match *err {
            ClassLoaderError::InvalidVerificationType(..) => (),
            _ => panic!("Expected InvalidVerificationType but got {}", err),
        });
    }

    #[test]
    fn test_verification_type_255_is_invalid() {
        deserialize_expecting_error(VerificationType::deserialize, b"\xff", |err| match *err {
            ClassLoaderError::InvalidVerificationType(..) => (),
            _ => panic!("Expected InvalidVerificationType but got {}", err),
        });
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_frame_with_offset_0() {
        assert_deserialize(StackMapFrame::SameFrame{offset_delta: 0}, b"\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_frame_with_offset_1() {
        assert_deserialize(StackMapFrame::SameFrame{offset_delta: 1}, b"\x01");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_frame_with_offset_0x3f() {
        assert_deserialize(StackMapFrame::SameFrame{offset_delta: 0x3f}, b"\x3f");
    }

    #[test]
    fn test_deserialize_stack_map_frame_empty_stream() {
        assert_eof(StackMapFrame::deserialize, b"");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_with_offset_0_and_integer_on_stack() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrame {
            offset_delta: 0,
            stack_item: VerificationType::Integer
        }, b"\x40\x01");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_with_offset_0_and_double_on_stack() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrame {
            offset_delta: 0,
            stack_item: VerificationType::Double
        }, b"\x40\x03");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_with_offset_0_and_object_on_stack() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrame {
            offset_delta: 0,
            stack_item: VerificationType::Object(ConstantIndex(0x1234)),
        }, b"\x40\x07\x12\x34");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_with_offset_0_and_uninitialized_item_on_stack() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrame {
            offset_delta: 0,
            stack_item: VerificationType::Uninitialized(0xabcd),
        }, b"\x40\x08\xab\xcd");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_with_offset_17() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrame {
            offset_delta: 17,
            stack_item: VerificationType::Double
        }, b"\x51\x03");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_with_offset_63() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrame {
            offset_delta: 63,
            stack_item: VerificationType::Double
        }, b"\x7f\x03");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_premature_termination_1() {
        assert_eof(StackMapFrame::deserialize, b"\x40");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_premature_termination_2() {
        assert_eof(StackMapFrame::deserialize, b"\x40\x08");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_premature_termination_3() {
        assert_eof(StackMapFrame::deserialize, b"\x40\x08\x12");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_extended_with_offset_0_and_stack_item_null() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrameExtended {
            offset_delta: 0,
            stack_item: VerificationType::Null,
        }, b"\xf7\x00\x00\x05");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_extended_with_offset_0_and_stack_item_top() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrameExtended {
            offset_delta: 0,
            stack_item: VerificationType::Top,
        }, b"\xf7\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_extended_with_offset_1() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrameExtended {
            offset_delta: 1,
            stack_item: VerificationType::Top,
        }, b"\xf7\x00\x01\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_extended_with_offset_0xabcd() {
        assert_deserialize(StackMapFrame::SameLocalsOneStackItemFrameExtended {
            offset_delta: 0xabcd,
            stack_item: VerificationType::Top,
        }, b"\xf7\xab\xcd\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_extended_premature_termination_1() {
        assert_eof(StackMapFrame::deserialize, b"\xf7");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_extended_premature_termination_2() {
        assert_eof(StackMapFrame::deserialize, b"\xf7\xab");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_extended_premature_termination_3() {
        assert_eof(StackMapFrame::deserialize, b"\xf7\xab\xcd");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_locals_one_stack_item_frame_extended_premature_termination_4() {
        assert_eof(StackMapFrame::deserialize, b"\xf7\xab\xcd\x08");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_chop_frame_with_delta_0_and_1_absent_local() {
        assert_deserialize(StackMapFrame::ChopFrame {
            offset_delta: 0,
            num_absent_locals: 1,
        }, b"\xfa\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_chop_frame_with_delta_0_and_2_absent_locals() {
        assert_deserialize(StackMapFrame::ChopFrame {
            offset_delta: 0,
            num_absent_locals: 2,
        }, b"\xf9\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_chop_frame_with_delta_0_and_3_absent_locals() {
        assert_deserialize(StackMapFrame::ChopFrame {
            offset_delta: 0,
            num_absent_locals: 3,
        }, b"\xf8\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_chop_frame_with_delta_0x1234_and_2_absent_locals() {
        assert_deserialize(StackMapFrame::ChopFrame {
            offset_delta: 0x1234,
            num_absent_locals: 2,
        }, b"\xf9\x12\x34");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_chop_frame_premature_termination_1() {
        assert_eof(StackMapFrame::deserialize, b"\xf8");
        assert_eof(StackMapFrame::deserialize, b"\xf9");
        assert_eof(StackMapFrame::deserialize, b"\xfa");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_chop_frame_premature_termination_2() {
        assert_eof(StackMapFrame::deserialize, b"\xf8\xab");
        assert_eof(StackMapFrame::deserialize, b"\xf9\xab");
        assert_eof(StackMapFrame::deserialize, b"\xfa\xab");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_frame_extended_with_offset_0() {
        assert_deserialize(StackMapFrame::SameFrameExtended{offset_delta: 0}, b"\xfb\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_frame_extended_with_offset_1() {
        assert_deserialize(StackMapFrame::SameFrameExtended{offset_delta: 1}, b"\xfb\x00\x01");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_frame_extended_with_offset_0xffff() {
        assert_deserialize(StackMapFrame::SameFrameExtended{offset_delta: 0xffff}, b"\xfb\xff\xff");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_frame_premature_termination_1() {
        assert_eof(StackMapFrame::deserialize, b"\xfb");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_same_frame_premature_termination_2() {
        assert_eof(StackMapFrame::deserialize, b"\xfb\x01");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_frame_with_offset_0_and_1_new_local_of_type_integer() {
        assert_deserialize(StackMapFrame::AppendFrame {
            offset_delta: 0,
            new_locals: vec![VerificationType::Integer],
        }, b"\xfc\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_frame_with_offset_0xffff_and_1_new_local_of_type_integer() {
        assert_deserialize(StackMapFrame::AppendFrame {
            offset_delta: 0xffff,
            new_locals: vec![VerificationType::Integer],
        }, b"\xfc\xff\xff\x01");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_frame_with_offset_0_and_1_new_local_of_type_object() {
        assert_deserialize(StackMapFrame::AppendFrame {
            offset_delta: 0,
            new_locals: vec![VerificationType::Object(ConstantIndex(0xbeef))],
        }, b"\xfc\x00\x00\x07\xbe\xef");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_frame_with_two_locals() {
        assert_deserialize(StackMapFrame::AppendFrame {
            offset_delta: 0,
            new_locals: vec![VerificationType::Integer, VerificationType::Long],
        }, b"\xfd\x00\x00\x01\x04");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_frame_with_two_nontrivial_locals() {
        assert_deserialize(StackMapFrame::AppendFrame {
            offset_delta: 0,
            new_locals: vec![VerificationType::Object(ConstantIndex(0xdead)), VerificationType::Uninitialized(0xbeef)],
        }, b"\xfd\x00\x00\x07\xde\xad\x08\xbe\xef");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_frame_with_three_locals() {
        assert_deserialize(StackMapFrame::AppendFrame {
            offset_delta: 0,
            new_locals: vec![
                VerificationType::Uninitialized(0x1234),
                VerificationType::Object(ConstantIndex(0x5678)),
                VerificationType::Uninitialized(0x789a),
            ]}, b"\xfe\x00\x00\x08\x12\x34\x07\x56\x78\x08\x78\x9a");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_premature_termination_1() {
        assert_eof(StackMapFrame::deserialize, b"\xfc");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_premature_termination_2() {
        assert_eof(StackMapFrame::deserialize, b"\xfc\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_premature_termination_3() {
        assert_eof(StackMapFrame::deserialize, b"\xfc\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_premature_termination_4() {
        assert_eof(StackMapFrame::deserialize, b"\xfc\x00\x00\x07");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_premature_termination_5() {
        assert_eof(StackMapFrame::deserialize, b"\xfc\x00\x00\x07\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_premature_termination_6() {
        assert_eof(StackMapFrame::deserialize, b"\xfd\x00\x00\x07\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_append_premature_termination_7() {
        assert_eof(StackMapFrame::deserialize, b"\xfe\x00\x00\x07\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_with_trivial_contents() {
        assert_deserialize(StackMapFrame::FullFrame {
            offset_delta: 0,
            locals: vec![],
            stack_items: vec![],
        }, b"\xff\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_with_offset_delta_of_1() {
        assert_deserialize(StackMapFrame::FullFrame {
            offset_delta: 1,
            locals: vec![],
            stack_items: vec![],
        }, b"\xff\x00\x01\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_with_offset_delta_of_ffff() {
        assert_deserialize(StackMapFrame::FullFrame {
            offset_delta: 0xffff,
            locals: vec![],
            stack_items: vec![],
        }, b"\xff\xff\xff\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_with_one_local() {
        assert_deserialize(StackMapFrame::FullFrame {
            offset_delta: 0,
            locals: vec![VerificationType::Null],
            stack_items: vec![],
        }, b"\xff\x00\x00\x00\x01\x05\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_with_5_locals() {
        assert_deserialize(StackMapFrame::FullFrame {
            offset_delta: 0,
            locals: vec![
                VerificationType::Null,
                VerificationType::Integer,
                VerificationType::Uninitialized(0xcafe),
                VerificationType::Long,
                VerificationType::Top],
            stack_items: vec![],
        }, b"\xff\x00\x00\x00\x05\x05\x01\x08\xca\xfe\x04\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_with_one_stack_item() {
        assert_deserialize(StackMapFrame::FullFrame {
            offset_delta: 0,
            locals: vec![],
            stack_items: vec![VerificationType::Null],
        }, b"\xff\x00\x00\x00\x00\x00\x01\x05");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_with_5_stack_items() {
        assert_deserialize(StackMapFrame::FullFrame {
            offset_delta: 0,
            locals: vec![],
            stack_items: vec![
                VerificationType::Null,
                VerificationType::Integer,
                VerificationType::Uninitialized(0xcafe),
                VerificationType::Long,
                VerificationType::Top],
        }, b"\xff\x00\x00\x00\x00\x00\x05\x05\x01\x08\xca\xfe\x04\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_with_locals_and_stack_items() {
        assert_deserialize(StackMapFrame::FullFrame {
            offset_delta: 0,
            locals: vec![
                VerificationType::Float,
                VerificationType::Long,
                VerificationType::Top,
                VerificationType::Object(ConstantIndex(0xbaba))],
            stack_items: vec![
                VerificationType::Uninitialized(0x1234),
                VerificationType::Null]
        }, b"\xff\x00\x00\x00\x04\x02\x04\x00\x07\xba\xba\x00\x02\x08\x12\x34\x05");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_during_offset_delta_1() {
        assert_eof(StackMapFrame::deserialize, b"\xff");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_during_offset_delta_2() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_during_local_count_1() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_during_local_count_2() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_before_locals() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_during_local() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00\x00\x01\x08");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_between_locals() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00\x00\x02\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_during_stack_item_count_1() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_during_stack_item_count_2() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_before_stack_items() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00\x00\x00\x00\x01");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_during_stack_items() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00\x00\x00\x00\x01\x07");
    }

    #[test]
    fn test_deserialize_stack_map_frame_of_type_full_frame_premature_termination_between_stack_items() {
        assert_eof(StackMapFrame::deserialize, b"\xff\x00\x00\x00\x00\x00\x02\x02");
    }

    #[test]
    fn test_stack_map_frame_types_128_to_246_are_invalid() {
        for frame_type in 128..=246 {
            let data = vec![frame_type];
            deserialize_expecting_error(StackMapFrame::deserialize, &data, |err| match *err {
                ClassLoaderError::InvalidStackFrameType(ref reported_frame_type) =>
                    if frame_type != *reported_frame_type {
                        panic!("InvalidStackFrameType error reported incorrect type; expected {}, was {}", frame_type, reported_frame_type);
                    },
                _ => panic!("Unexpected error; wanted InvalidStackFrameType, but got {:#?}", err),
            });
        }
    }

    #[test]
    fn test_deserialize_empty_stack_map_table() {
        let expected = Attribute::StackMapTable {
            attribute_name: ConstantIndex(1),
            entries: vec![],
        };

        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        let bytes = b"\x00\x01\x00\x00\x00\x02\x00\x00";

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_with_different_constant_index() {
        let expected = Attribute::StackMapTable {
            attribute_name: ConstantIndex(3),
            entries: vec![],
        };

        let constants = vec![Constant::Integer(1), Constant::Integer(2), Constant::Utf8("StackMapTable".to_string())];
        let bytes = b"\x00\x03\x00\x00\x00\x02\x00\x00";

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_with_one_frame_of_type_same_frame() {
        let expected = Attribute::StackMapTable {
            attribute_name: ConstantIndex(1),
            entries: vec![StackMapFrame::SameFrame {
                offset_delta: 0x3f,
            }],
        };

        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        let bytes = b"\x00\x01\x00\x00\x00\x03\x00\x01\x3f";

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_with_one_frame_of_type_full_frame() {
        let expected = Attribute::StackMapTable {
            attribute_name: ConstantIndex(1),
            entries: vec![StackMapFrame::FullFrame {
                offset_delta: 0x72,
                locals: vec![VerificationType::Integer, VerificationType::Top],
                stack_items: vec![VerificationType::Null],
            }],
        };

        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        let bytes = b"\x00\x01\x00\x00\x00\x0c\x00\x01\xff\x00\x72\x00\x02\x01\x00\x00\x01\x05";

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_with_several_frames() {
        let expected = Attribute::StackMapTable {
            attribute_name: ConstantIndex(1),
            entries: vec![
                StackMapFrame::FullFrame {
                    offset_delta: 0x40,
                    locals: vec![VerificationType::Integer, VerificationType::Float],
                    stack_items: vec![],
                },
                StackMapFrame::ChopFrame {
                    offset_delta: 0x50,
                    num_absent_locals: 1,
                },
                StackMapFrame::AppendFrame {
                    offset_delta: 0x5f,
                    new_locals: vec![VerificationType::Null],
                },
            ],
        };

        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        let bytes = b"\x00\x01\x00\x00\x00\x12\x00\x03\xff\x00\x40\x00\x02\x01\x02\x00\x00\xfa\x00\x50\xfc\x00\x5f\x05";

        assert_deserialize_with_constants(expected, bytes, &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_premature_termination_during_attribute_length_1() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01", &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_premature_termination_during_attribute_length_2() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00", &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_premature_termination_during_attribute_length_3() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00", &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_premature_termination_during_attribute_length_4() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00", &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_premature_termination_during_entry_count_1() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x00", &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_premature_termination_during_entry_count_2() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x01\x00", &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_premature_termination_before_entries() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x02\x00\x01", &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_premature_termination_during_entry() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x03\x00\x01\xff", &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_premature_termination_between_entries() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        assert_eof_with_constants(Attribute::deserialize, b"\x00\x01\x00\x00\x00\x03\x00\x02\x00", &constants);
    }

    #[test]
    fn test_deserialize_stack_map_table_errors_if_declared_length_is_zero() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x00\x00\x00",
            &constants,
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Unexpected error; expected LengthMismatch; got {:#?}", err),
            });
    }

    #[test]
    fn test_deserialize_stack_map_table_errors_if_declared_length_is_one() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x01\x00\x00",
            &constants,
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Unexpected error; expected LengthMismatch; got {:#?}", err),
            });
    }

    #[test]
    fn test_deserialize_stack_map_table_errors_if_declared_length_is_shorter_than_one_byte_table() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x02\x00\x01\x01",
            &constants,
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Unexpected error; expected LengthMismatch; got {:#?}", err),
            });
    }

    #[test]
    fn test_deserialize_stack_map_table_errors_if_declared_length_is_longer_than_one_byte_table() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x04\x00\x01\x01",
            &constants,
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Unexpected error; expected LengthMismatch; got {:#?}", err),
            });
    }

    #[test]
    fn test_deserialize_stack_map_table_errors_if_declared_length_is_shorter_than_long_single_entry_table() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0d\x00\x01\xff\xab\xcd\x00\x02\x00\x05\x00\x03\x01\x00\x02",
            &constants,
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Unexpected error; expected LengthMismatch; got {:#?}", err),
            });
    }

    #[test]
    fn test_deserialize_stack_map_table_errors_if_declared_length_is_longer_than_long_single_entry_table() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0f\x00\x01\xff\xab\xcd\x00\x02\x00\x05\x00\x03\x01\x00\x02",
            &constants,
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Unexpected error; expected LengthMismatch; got {:#?}", err),
            });
    }

    #[test]
    fn test_deserialize_stack_map_table_errors_if_declared_length_is_shorter_than_multi_entry_table() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x0f\x00\x03\x02\xff\xab\xcd\x00\x03\x01\x00\x02\x00\x02\x00\x05\x3e",
            &constants,
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Unexpected error; expected LengthMismatch; got {:#?}", err),
            });
    }

    #[test]
    fn test_deserialize_stack_map_table_errors_if_declared_length_is_longer_than_multi_entry_table() {
        let constants = vec![Constant::Utf8("StackMapTable".to_string())];
        deserialize_with_constants_expecting_error(
            Attribute::deserialize,
            b"\x00\x01\x00\x00\x00\x11\x00\x03\x02\xff\xab\xcd\x00\x03\x01\x00\x02\x00\x02\x00\x05\x3e",
            &constants,
            |err| match *err {
                ClassLoaderError::LengthMismatch{..} => (),
                _ => panic!("Unexpected error; expected LengthMismatch; got {:#?}", err),
            });
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
