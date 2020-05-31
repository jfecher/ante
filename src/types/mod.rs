
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TypeVariableId(usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TypeInfoId(usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PrimitiveType {
    IntegerType,      // : *
    FloatType,        // : *
    CharType,         // : *
    StringType,       // : *
    BooleanType,      // : *
    UnitType,         // : *
    ReferenceType,    // : * -> *
    Function,         // : ? -> *
}

#[derive(Debug)]
pub enum Type {
    /// int, char, bool, functions, etc
    Primitive(PrimitiveType),

    /// Any stand-in type e.g. a in Vec a. The original names are
    /// translated into unique TypeIds during name resolution.
    TypeVariable(TypeVariableId),

    /// Any user defined type defined via the `type` keyword
    /// These have a unique UserDefinedTypeId which points to
    /// additional information about the contents of the type
    /// not needed for most type checking.
    UserDefinedType(TypeInfoId),

    /// Any type in the form `a b`
    TypeApplication(Box<Type>, Box<Type>),

    /// These are currently used internally to indicate polymorphic
    /// type variables for let-polymorphism. There is no syntax to
    /// specify these explicitly in ante code.
    ForAll(Vec<TypeVariableId>, Box<Type>),
}

pub struct TypeConstructor<'a> {
    pub name: &'a str,
    pub args: Vec<Type>,
}

pub struct Field<'a> {
    pub name: &'a str,
    pub field_type: Type,
}

pub enum TypeInfo<'a> {
    Union(&'a str, Vec<TypeConstructor<'a>>),
    Struct(&'a str, Vec<Field<'a>>),
}

pub enum Kind {
    /// usize is the number of type arguments it takes before it returns a type of kind *.
    /// For example, the kind Normal(2) : * -> * -> *
    Normal(usize),

    /// A higher order kind where each element in the Vec is an argument. For example, the kind:
    /// HigherOrder(vec![ Normal(0), HigherOrder(vec![ Normal(0), Normal(1) ]), Normal(1) ])
    /// has kind: * -> (* -> (* -> *)) -> (* -> *)
    HigherOrder(Vec<Kind>),
}
