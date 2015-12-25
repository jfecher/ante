#ifndef TOKENS_H
#define TOKENS_H

#define IS_LITERAL(t) ((t) < 258)

//TOK_TYPE_STR assumes t is not a literal token
#define TOK_TYPE_STR(t) (tokDictionary[(t)-258])

enum TokenType{
    Tok_Ident = 258,
    Tok_UserType,

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
    Tok_Range,   //inclusive range
    Tok_RangeBX, //beginning-exclusive range
    Tok_RangeEX, //end exclusive range
    Tok_RangeX,  //exclusive range
    //All other operators are returned by ASCII value

    //literals
    Tok_True,
    Tok_False,
	Tok_IntLit,
	Tok_FltLit,
	Tok_StrLit,

    //keywords
    Tok_Return,
	Tok_If,
    Tok_Elif,
	Tok_Else,
	Tok_For,
	Tok_While,
    Tok_Do,
    Tok_In,
	Tok_Continue,
	Tok_Break,
    Tok_Import,
    Tok_Match,
    Tok_Data,
    Tok_Enum,

    //modifiers
    Tok_Pub,
    Tok_Pri,
    Tok_Pro,
    Tok_Const,
    Tok_Ext,
    Tok_Dyn,
    Tok_Pathogen,

    //other
    Tok_Where,
    Tok_Infect,
    Tok_Cleanse,
    Tok_Ct,

    Tok_Newline,
    Tok_Indent,
    Tok_Unindent,
};

//defined in src/lexer.cpp
extern const char* tokDictionary[];

//#define TOK(t, r, c) (Token){t, NULL, r, c}
//#define TOKL(t, r) (Token){t, l, r, c}

#define IS_TERMINATING_OP(o) ((o)==']'||(o)=='}'||(o)==')')

#endif
