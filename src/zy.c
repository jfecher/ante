#include "interpreter.h"

char *tokenDictionary[] = {
    "Greater",
    "Identifier",
    "Print",
    "Num",
    "String",
    "Int",
    "FuncCall",

    "Invalid",
    "Assign",
    "Multiply",
    "Divide",
    "Plus",
    "Minus",
    "PlusEquals",
    "MinusEquals",
    "EqualsEquals",
    "GreaterEquals",
    "Equals",
    "LesserEquals",
    "Lesser",
    "Modulus",
    "BraceOpen",
    "BraceClose",
    "ParenOpen",
    "ParenClose",
    "BracketOpen",
    "BracketClose",
    "Underscore",
    "Comma",
    "Colon",
    "Bar",
    "Boolean",
    "BooleanOr",
    "BooleanAnd",
    "BooleanTrue",
    "BooleanFalse",
    "IntegerLiteral",
    "DoubleLiteral",
    "StringLiteral",
    "MultiplyEquals",
    "DivideEquals",
    "Return",
    "If",
    "Else",
    "For",
    "ForEach",
    "While",
    "Continue",
    "Break",
    "Import",
    "Newline",
    "TypeDef",
    "Indent",
    "Unindent",
    "EndOfInput",
    "StrConcat",
    "MalformedString",
    "Exponent",
    "FuncDef",
    "In"
};

int main(int argc, const char *argv[])
{
    if(argc >= 3){
        src = fopen(argv[argc-1], "r");

        if(src == NULL){
            printf("File not found.");
            return 2; //File not found
        }

        if(strcmp(argv[1], "--lex") == 0 || strcmp(argv[1], "-l") == 0){
            lexAndPrint();
        }else if(strcmp(argv[1], "--exec") == 0 || strcmp(argv[1], "-e") == 0){
            interpret(src, 0);
        }else{
            printf("Unrecognized option '%s'\n", argv[1]);
        }

        fclose(src);
    }else if(argc == 2){ //if there is only one argument, assume it is a file to execute
        src = fopen(argv[1], "r");

        if(src == NULL){
            printf("File '%s' not found.\n", argv[1]);
            return 2; //File not found
        }

        interpret(src, 0);
        fclose(src);
    }else if(argc == 1){
        interpret(src, 1);
    }
    return 0;
}
