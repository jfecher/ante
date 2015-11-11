#ifndef TOKENS_H
#define TOKENS_H

enum TokenType{
    Tok_EndOfInput,
    Tok_Ident,

    //types
    Tok_I8,
    Tok_I16,
    Tok_I32,
    Tok_I64,
    Tok_U8,
    Tok_U16,
    Tok_U32,
    Tok_U64,
    Tok_F32,
    Tok_F64,
    Tok_Bool,
    Tok_Void,

	Tok_Assign,
	Tok_Eq,
    Tok_NotEq,
	Tok_AddEq,
	Tok_SubEq,
    Tok_MulEq,
    Tok_DivEq,
	Tok_GrtrEq,
	Tok_LesrEq,
    Tok_Add,
    Tok_Sub,
    Tok_Mul,
    Tok_Div,
    Tok_Or,
    Tok_And,
    Tok_True,
    Tok_False,
	Tok_IntLit,
	Tok_FltLit,
	Tok_StrLit,
    Tok_StrCat,

    Tok_ParenOpen,
    Tok_ParenClose,
    Tok_BraceOpen,
    Tok_BraceClose,
    Tok_BracketOpen,
    Tok_BracketClose,

    //keywords
    Tok_Return,
	Tok_If,
    Tok_Elif,
	Tok_Else,
	Tok_For,
	Tok_ForEach,
	Tok_While,
    Tok_Do,
    Tok_In,
	Tok_Continue,
	Tok_Break,
    Tok_Import,
    Tok_Where,
    Tok_Enum,
    Tok_Struct,
    Tok_Class,

    Tok_Newline,
    Tok_Indent,
    Tok_Unindent,
};

extern const char* tokDictionary[];

typedef struct{
    TokenType type;
    const char* lexeme;
    unsigned int row;
    unsigned int col;
} Token;

#define TOK(t, r, c) (Token){t, NULL, r, c}
#define TOKL(t, r) (Token){t, l, r, c}

#endif
