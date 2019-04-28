use std::{error, fmt};

#[derive(PartialEq, Debug)]
pub struct Class {
    pub minor_version: u16,
    pub major_version: u16,
    pub constants: Vec<Constant>,
    pub flags: ClassFlags,
    pub this_class: ConstantIndex,
    pub super_class: ConstantIndex,
    pub interfaces: Vec<ConstantIndex>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ConstantIndex(pub u16);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct MethodIndex(pub u16);

#[derive(PartialEq, Clone, Debug)]
pub enum Constant {
    Utf8(String),
    Integer(u32),
    Float(f32),
    Long(u64),
    Double(f64),
    ClassRef(ConstantIndex),
    StringRef(ConstantIndex),
    FieldRef{class:ConstantIndex, name_and_type:ConstantIndex},
    MethodRef{class:ConstantIndex, name_and_type:ConstantIndex},
    InterfaceMethodRef{class:ConstantIndex, name_and_type:ConstantIndex},
    NameAndTypeRef{name:ConstantIndex, descriptor:ConstantIndex},
    MethodHandleRef(MethodHandle),
    MethodType(ConstantIndex),
    InvokeDynamicInfo{bootstrap_method_attr:MethodIndex, name_and_type:ConstantIndex},
    Dummy, // Necessary to fake Long and Double taking up two slots
}

impl Constant {
    pub fn get_tag(self) -> Option<u8> {
        match self {
            Constant::Utf8(_) => Some(1),
            Constant::Integer(_) => Some(3),
            Constant::Float(_) => Some(4),
            Constant::Long(_) => Some(5),
            Constant::Double(_) => Some(6),
            Constant::ClassRef(_) => Some(7),
            Constant::StringRef(_) => Some(8),
            Constant::FieldRef{..} => Some(9),
            Constant::MethodRef{..} => Some(10),
            Constant::InterfaceMethodRef{..} => Some(11),
            Constant::NameAndTypeRef{..} => Some(12),
            Constant::MethodHandleRef(_) => Some(15),
            Constant::MethodType(_) => Some(16),
            Constant::InvokeDynamicInfo{..} => Some(18),
            Constant::Dummy => None,
        }
    }
}

bitflags! {
    pub struct ClassFlags: u16 {
        const PUBLIC     = 0x0001;
        const FINAL      = 0x0010;
        const SUPER      = 0x0020;
        const INTERFACE  = 0x0200;
        const ABSTRACT   = 0x0040;
        const SYNTHETIC  = 0x1000;
        const ANNOTATION = 0x2000;
        const ENUM       = 0x4000;
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Field {
    pub flags: FieldFlags,
    pub name: ConstantIndex,
    pub descriptor: ConstantIndex,
    pub attributes: Vec<Attribute>,
}

bitflags! {
    pub struct FieldFlags: u16 {
        const PUBLIC    = 0x0001;
        const PRIVATE   = 0x0002;
        const PROTECTED = 0x0004;
        const STATIC    = 0x0008;
        const FINAL     = 0x0010;
        const VOLATILE  = 0x0040;
        const TRANSIENT = 0x0080;
        const SYNTHETIC = 0x1000;
        const ENUM      = 0x4000;
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Method {
    pub flags: MethodFlags,
    pub name: ConstantIndex,
    pub descriptor: ConstantIndex,
    pub attributes: Vec<Attribute>,
}

bitflags! {
    pub struct MethodFlags: u16 {
        const PUBLIC       = 0x0001;
        const PRIVATE      = 0x0002;
        const PROTECTED    = 0x0004;
        const STATIC       = 0x0008;
        const FINAL        = 0x0010;
        const SYNCHRONIZED = 0x0020;
        const BRIDGE       = 0x0040;
        const VARARGS      = 0x0080;
        const NATIVE       = 0x0100;
        const ABSTRACT     = 0x0400;
        const STRICT       = 0x0800;
        const SYNTHETIC    = 0x1000;
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Attribute {
    ConstantValue {attribute_name: ConstantIndex, constant_value: ConstantIndex},
    Code {
        attribute_name: ConstantIndex,
        max_stack: u16,
        max_locals: u16,
        code: Vec<u8>,
        exception_table: Vec<ExceptionTableRow>,
        attributes: Vec<Attribute>,
    },
    StackMapTable {attribute_name: ConstantIndex, entries: Vec<StackMapFrame>},
    Exceptions {attribute_name: ConstantIndex, index_table: Vec<ConstantIndex>},
    InnerClasses {attribute_name: ConstantIndex, classes: Vec<InnerClassInfo>},
    EnclosingMethod {
        attribute_name: ConstantIndex,
        class: ConstantIndex,
        method: ConstantIndex,
    },
    Synthetic {attribute_name: ConstantIndex},
    Signature {attribute_name: ConstantIndex, signature: ConstantIndex},
    SourceFile {attribute_name: ConstantIndex, source_file: ConstantIndex},
    SourceDebug {attribute_name: ConstantIndex, debug_extension: Vec<u8>},
    LineNumberTable {
        attribute_name: ConstantIndex,
        table: Vec<(u16, u16)>,
    },
    LocalVariableTable {
        attribute_name: ConstantIndex,
        variables: Vec<LocalVariable>,
    },
    LocalVariableTypeTable {
        attribute_name: ConstantIndex,
        variable_types: Vec<LocalVariableType>,
    },
    Deprecated {
        attribute_name: ConstantIndex
    },
    RuntimeVisibleAnnotations {
        attribute_name: ConstantIndex,
        annotations: Vec<Annotation>,
    },
    RuntimeInvisibleAnnotations {
        attribute_name: ConstantIndex,
        annotations: Vec<Annotation>,
    },
    RuntimeVisibleParameterAnnotations {
        attribute_name: ConstantIndex,
        annotations_by_param_index: Vec<ParameterAnnotations>,
    },
    RuntimeInvisibleParameterAnnotations {
        attribute_name: ConstantIndex,
        annotations_by_param_index: Vec<ParameterAnnotations>,
    },
    AnnotationDefault {
        attribute_name: ConstantIndex,
        value: ElementValue,
    },
    BootstrapMethods {
        attribute_name: ConstantIndex,
        methods: Vec<BootstrapMethod>,
    },
}

#[derive(PartialEq, Eq, Debug)]
pub struct ExceptionTableRow {
    start_pc: u16,
    end_pc: u16,
    handler_pc: u16,
    catch_type: ConstantIndex
}

#[derive(PartialEq, Eq, Debug)]
pub enum StackMapFrame {
    SameFrame {offset_delta: u8},
    SameLocalsOneStackItemFrame {offset_delta: u8, stack_item: VerificationType},
    SameLocalsOneStackFrameExtended {offset_delta: u16, stack_item: VerificationType},
    ChopFrame {offset_delta: u16, num_absent_locals: u8},
    SameFrameExtended {offset_delta: u16},
    AppendFrame {offset_delta: u16, new_locals: Vec<VerificationType>},
    FullFrame {
        offset_delta: u16,
        locals: Vec<VerificationType>,
        stack_items: Vec<VerificationType>,
    },
}

#[derive(PartialEq, Eq, Debug)]
pub enum VerificationType {
    Top,
    Integer,
    Float,
    Long,
    Double,
    Null,
    UninitializedThis,
    Object(ConstantIndex),
    Uninitialized,
}

#[derive(PartialEq, Eq, Debug)]
pub struct InnerClassInfo {
    inner_class: ConstantIndex,
    outer_class: ConstantIndex,
    inner_class_name: ConstantIndex,
    flags: InnerClassFlags,
}

bitflags! {
    pub struct InnerClassFlags: u16 {
        const PUBLIC     = 0x0001;
        const PRIVATE    = 0x0002;
        const PROTECTED  = 0x0004;
        const STATIC     = 0x0008;
        const FINAL      = 0x0010;
        const INTERFACE  = 0x0200;
        const ABSTRACT   = 0x0040;
        const SYNTHETIC  = 0x1000;
        const ANNOTATION = 0x2000;
        const ENUM       = 0x4000;
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct LocalVariable {
    start_pc: u16,
    length: u16,
    name: ConstantIndex,
    descriptor: ConstantIndex,
    index: u16,
}

#[derive(PartialEq, Eq, Debug)]
pub struct LocalVariableType {
    start_pc: u16,
    length: u16,
    name: ConstantIndex,
    signature: ConstantIndex,
    index: u16,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Annotation {
    type_index: ConstantIndex,
    indexes_with_values: Vec<(ConstantIndex, ElementValue)>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum ElementValue {
    Byte(ConstantIndex),
    Char(ConstantIndex),
    Double(ConstantIndex),
    Float(ConstantIndex),
    Integer(ConstantIndex),
    Long(ConstantIndex),
    Short(ConstantIndex),
    Boolean(ConstantIndex),
    String(ConstantIndex),
    Enum {enum_type: ConstantIndex, enum_value: ConstantIndex},
    Class(ConstantIndex),
    Annotation(Annotation),
    Array(Vec<ElementValue>),
}

#[derive(PartialEq, Eq, Debug)]
pub struct ParameterAnnotations(Vec<Annotation>);

#[derive(PartialEq, Eq, Debug)]
pub struct BootstrapMethod {
    method: ConstantIndex,
    arguments: Vec<ConstantIndex>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum MethodHandle {
    GetField(ConstantIndex),
    GetStatic(ConstantIndex),
    PutField(ConstantIndex),
    PutStatic(ConstantIndex),
    InvokeVirtual(ConstantIndex),
    InvokeStatic(ConstantIndex),
    InvokeSpecial(ConstantIndex),
    NewInvokeSpecial(ConstantIndex),
    InvokeInterface(ConstantIndex),
}

impl ConstantIndex {
    pub fn lookup(self, constant_pool: &Vec<Constant>) -> Result<&Constant, ConstantLookupError> {
        if self.0 == 0 {
            return Err(ConstantLookupError::ZeroIndex);
        } else if constant_pool.len() < self.0 as usize {
            return Err(ConstantLookupError::OutOfRange(self.0));
        }

        let constant = &constant_pool[(self.0 - 1) as usize];
        match *constant {
            Constant::Dummy => Err(ConstantLookupError::IndexInsideDoubleWidthConstant(self.0)),
            _ => Ok(constant),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConstantLookupError {
    OutOfRange(u16),
    ZeroIndex,
    IndexInsideDoubleWidthConstant(u16),
}

impl fmt::Display for ConstantLookupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConstantLookupError::OutOfRange(ref index) => write!(f, "Constant index out of range: {}", index),
            ConstantLookupError::ZeroIndex => write!(f, "Constant index 0 is invalid in this context"),
            ConstantLookupError::IndexInsideDoubleWidthConstant(ref index) => write!(f, "Index {} lies inside a double-width value", index),
        }
    }
}

impl error::Error for ConstantLookupError {
    fn description(&self) -> &str {
        match *self {
            ConstantLookupError::OutOfRange(_) => "Constant index out of range",
            ConstantLookupError::ZeroIndex => "Constant index 0 is invalid in this context",
            ConstantLookupError::IndexInsideDoubleWidthConstant(_) => "Constant index lies inside a double-width value",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_constant_1_when_exists() {
        let pool = vec![Constant::Integer(4)];
        assert_eq!(Ok(&Constant::Integer(4)), ConstantIndex(1).lookup(&pool));
    }

    #[test]
    fn test_lookup_constant_2_when_exists() {
        let pool = vec![
            Constant::Integer(42),
            Constant::Utf8("Hello!".to_string()),
        ];
        assert_eq!(Ok(&Constant::Utf8("Hello!".to_string())), ConstantIndex(2).lookup(&pool));
    }

    #[test]
    fn test_lookup_constant_0xffff_when_exists() {
        let mut pool = vec![];
        for idx in 1..0x10000 {
            pool.push(Constant::Integer(idx));
        }
        assert_eq!(Ok(&Constant::Integer(0xffff)), ConstantIndex(0xffff).lookup(&pool));
    }

    #[test]
    fn test_lookup_constant_2_in_singleton_pool_throws_out_of_range() {
        let pool = vec![Constant::Float(1.0)];
        assert_out_of_range(ConstantIndex(2), &pool);
    }

    #[test]
    fn test_lookup_constant_1_in_empty_pool_throws_out_of_range() {
        let pool = vec![];
        assert_out_of_range(ConstantIndex(1), &pool);
    }

    #[test]
    fn test_lookup_constant_0_in_singleton_pool_throws_zero_index() {
        let pool = vec![Constant::Integer(3)];
        assert_error(ConstantIndex(0), &pool, |err| match *err {
            ConstantLookupError::ZeroIndex => (),
            _ => panic!("Expected zero index error; got {:#?}", err),
        });
    }

    #[test]
    fn test_lookup_yielding_dummy_throws_index_inside_double_width_constant() {
        let pool = vec![Constant::Integer(3), Constant::Long(4), Constant::Dummy, Constant::Utf8("Foo".to_string())];
        assert_error(ConstantIndex(3), &pool, |err| match *err {
            ConstantLookupError::IndexInsideDoubleWidthConstant(_) => (),
            _ => panic!("Expected index-inside-double-width-constant error; got {:#?}", err),
        });
    }

    fn assert_out_of_range(index: ConstantIndex, pool: &Vec<Constant>) {
        assert_error(index, pool, |err| match *err {
            ConstantLookupError::OutOfRange(_) => (),
            _ => panic!("Expected out of range; got {:#?}", err),
        });
    }

    fn assert_error<H>(index: ConstantIndex, pool: &Vec<Constant>, handler: H)
       where H: Fn(&ConstantLookupError)
    {
        let err = index.lookup(&pool).expect_err("Expected an error; got unexpected result");
        handler(&err);
    }
}
