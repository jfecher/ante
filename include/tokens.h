#ifndef TOKENS_H
#define TOKENS_H

#define IS_LITERAL(t) ((t) < 258)

/* 
 *  Basic enum containing all primitive types as well
 *  as the basic classification of some others.  Intended for
 *  quick reference rather than in-depth analysis.
 */
enum TypeTag{
    TT_I8,  TT_I16, TT_I32, TT_I64, //signed integer types
    TT_U8,  TT_U16, TT_U32, TT_U64, //unsigned integer types
    TT_F16, TT_F32, TT_F64, //float types
    TT_Isz, TT_Usz, //pointer-sized integer types
    TT_C8,  TT_C32, //char types
    TT_Bool,
    TT_StrLit,
    TT_Tuple, //anonymous tuples
    TT_Array,
    TT_Ptr,
    TT_Data, //all previously declared UserTypes
    TT_TypeVar,
    TT_Function,
    TT_Method,
    TT_TaggedUnion,
    TT_MetaFunction,
    TT_Void
};

enum TokenType{
    Tok_Ident = 258,
    Tok_UserType,
    Tok_TypeVar,

    //types
    Tok_I8,
    Tok_I16,
    Tok_I32,
    Tok_I64,
    Tok_U8,
    Tok_U16,
    Tok_U32,
    Tok_U64,
    Tok_Isz, //Signed integer with pointer size
    Tok_Usz,
    Tok_F16,
    Tok_F32,
    Tok_F64,
    Tok_C8,
    Tok_C32,
    Tok_Bool,
    Tok_Void,

    /*operators*/
    Tok_Eq,
    Tok_NotEq,
    Tok_AddEq,
    Tok_SubEq,
    Tok_MulEq,
    Tok_DivEq,
    Tok_GrtrEq,
    Tok_LesrEq,
    Tok_Or,
    Tok_And,
    Tok_Range,
    Tok_RArrow,
    Tok_ApplyL,
    Tok_ApplyR,
    Tok_Append,
    Tok_New,
    Tok_Not,
    //All other operators are returned by ASCII value

    //literals
    Tok_True,
    Tok_False,
    Tok_IntLit,
    Tok_FltLit,
    Tok_StrLit,
    Tok_CharLit,

    //keywords
    Tok_Return,
    Tok_If,
    Tok_Then,
    Tok_Elif,
    Tok_Else,
    Tok_For,
    Tok_While,
    Tok_Do,
    Tok_In,
    Tok_Continue,
    Tok_Break,
    Tok_Import,
    Tok_Let,
    Tok_Var,
    Tok_Match,
    Tok_With,
    Tok_Type,
    Tok_Trait,
    Tok_Fun,
    Tok_Ext,

    //modifiers
    Tok_Pub,
    Tok_Pri,
    Tok_Pro,
    Tok_Raw,
    Tok_Const,
    Tok_Noinit,
    Tok_Mut,

    //other
    Tok_Where,

    Tok_Newline,
    Tok_Indent,
    Tok_Unindent,
};

#endif
