#ifndef LEXER_H
#define LEXER_H

#include <string>
#include <fstream>
#include <set>
#include <map>
using namespace std;

namespace zyl{

    #define IS_NUMERIC(c) (c >= 48 && c <= 57)
    #define IS_ALPHA_NUMERIC(c) (IS_NUMERIC(c) || (c >= 65 && c <= 98) || (c >= 97 && c <= 122) || c == 95)
    #define IS_WHITESPACE(c) (c == ' ' || c == '\t' || c == '\n' || c == 13)
    #define IS_COMMENT(c) (c == '~' || c == '`')
    #define SCOPE_STEP 4

    enum BinaryOps{
        Eq,
        NotEq,
        Less,
        Grtr,
        LessEq,
        GrtrEq,
        Add,
        Sub,
        Mul,
        Div,
        Rem,
        Comma,
        Cat,
        Period,
    };
    
    enum UnaryOps{
        Not,
        Deref,
        Address,
        ParenOpen,
        ParenClose,
        BracketOpen,
        BracketClose,
        BraceOpen,
        BraceClose,
    };

    enum DataTypes{
        I8,
        I16,
        I32,
        I64,
        U8,
        U16,
        U32,
        U64,
        F32,
        F64,
        Str,
        Bool,
        Void
    };


    enum Literals{
        True,
        False,
        StrLit,
        IntLit,
        FloatLit,
    };

    enum Modifiers{
        Pub,
        Pri,
        Pro,
        Const,
        Dyn, 
    };

    enum TokenType{
        Identifier,
        FuncCall,
        FuncDef,
        Newline,
        Indent,
        Unindent,
        
        //keywords
        If,
        Elif,
        Else,
        Import,
        Match,
        For,
        Foreach,
        In,
        Do,
        While,
        Continue,
        Break,
        Where,

        Assign,
        Colon,
        Struct,
        Class,
        Enum,

        DataType,
        Literal,
        Modifier,
        OpUnary,
        OpBinary,
        EndOfInput
    };

    #define BOPTOK(d) (Token){TokenType::OpBinary, (int)(d)}
    #define UOPTOK(d) (Token){TokenType::OpUnary,  (int)(d)}
    #define MODTOK(d) (Token){TokenType::Modifier, (int)(d)}
    #define LITTOK(d) (Token){TokenType::Literal,  (int)(d)}

    struct Token{
        TokenType type;
        int data;
        string lexeme;
    };

    class Lexer{
        public:
            Lexer(ifstream in);
            Token getNextToken(void);
        private:
            unsigned char curScope;
            unsigned char scope;
            char c, n;
            ifstream f;
            string src;

            void incPos(void);
            void skipTo(char c);
            Token skipComment(void);
            Token genNumLit(void);
            Token genStrLit(void);
            Token genWhitespaceToken(void);
            Token genAlphaNumericToken(void);
    };
};

#endif
