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

#[derive(PartialEq, Eq, Debug)]
pub struct ConstantIndex(pub u16);

#[derive(PartialEq, Eq, Debug)]
pub struct MethodIndex(pub u16);

#[derive(PartialEq, Debug)]
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

#[derive(PartialEq, Eq, Debug)]
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
