#include "interpreter.h"

char *tokenDictionary[] = {
    "Tok_Greater",
    "Tok_Identifier",
    "Tok_Print",
    "Tok_Function",
    "Tok_Num",
    "Tok_String",
    "Tok_Int",

    "Tok_Invalid",
    "Tok_Begin",
	"Tok_Assign",
	"Tok_Multiply",
	"Tok_Divide",
	"Tok_Plus",
	"Tok_Minus",
	"Tok_PlusEquals",
	"Tok_MinusEquals",
    "Tok_EqualsEquals",
	"Tok_GreaterEquals",
	"Tok_Equals",
	"Tok_LesserEquals",
	"Tok_Lesser",
    "Tok_Modulus",
	"Tok_BraceOpen",
	"Tok_BraceClose",
	"Tok_ParenOpen",
	"Tok_ParenClose",
	"Tok_BracketOpen",
	"Tok_BracketClose",
	"Tok_Underscore",
	"Tok_Comma",
	"Tok_Colon",
    "Tok_ListInitializer",
    "Tok_BooleanOr",
    "Tok_Boolean",
    "Tok_BooleanTrue",
    "Tok_BooleanAnd",
    "Tok_BooleanFalse",
	"Tok_IntegerLiteral",
	"Tok_DoubleLiteral",
	"Tok_StringLiteral",
	"Tok_MultiplyEquals",
    "Tok_DivideEquals",
	"Tok_Return",
	"Tok_If",
	"Tok_Else",
	"Tok_For",
	"Tok_ForEach",
	"Tok_While",
	"Tok_Continue",
	"Tok_Break",
    "Tok_Import",
    "Tok_Newline",
    "Tok_TypeDef",
    "Tok_Indent",
    "Tok_Unindent",
	"Tok_EndOfInput",
    "Tok_StrConcat",
    "Tok_MalformedString",
    "Tok_Exponent"
};

int main(int argc, const char *argv[]) //Main entry point
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
        puts("0");
    }else if(argc == 2){
        src = fopen(argv[1], "r");

        if(src == NULL){
            printf("File %s not found.\n", argv[1]);
            return 2; //File not found
        }

        interpret(src, 0);
        fclose(src);
    }else if(argc == 1){
        interpret(src, 1);
    }
    return 0;
}
