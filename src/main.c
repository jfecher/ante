#include "interpreter.h"

char *tokenDictionary[] = {
    "Greater",
    "Identifier",
    "Print",
    "Function",
    "Num",
    "String",

    "Invalid",         //0
    "Begin",           //1
	//"Identifier",      //2
	"Assign",          //3
	"Multiply",        //4
	"Divide",          //5
	"Plus",            //6
	"Minus",           //7
	"PlusEquals",      //8
	"MinusEquals",     //9
    "EqualsEquals",    //10
	//"Greater",         //11
	"GreaterEquals",   //12
	"Equals",          //13
	"LesserEquals",    //14
	"Lesser",          //15
	"BraceOpen",       //16
	"BraceClose",      //17
	"ParenOpen",       //18
	"ParenClose",      //19
	"BracketOpen",     //20
	"BracketClose",    //21
	"Underscore",      //22
	"Comma",           //23
	"Colon",           //24
    "ListInitializer", //25
    "Char",            //26
    "Boolean",         //27
    "BooleanOr",       //28
    "BooleanAnd",      //29
    "BooleanTrue",     //30
    "BooleanFalse",    //31
	"IntegerLiteral",  //32
	"DoubleLiteral",   //33
	"StringLiteral",   //34
    "CharLiteral",     //35
	"MultiplyEquals",  //36
    "DivideEquals",    //37
	//"Int",             //38
	//"Double",          //39
	//"String",          //40
	//"Function",        //41
	//"Print",           //42
	"Return",          //43
	"If",              //44
	"Else",            //45
	"For",             //46
	"While",           //47
	"Continue",        //48
	"Break",           //49
    "Import",          //50
    "Newline",         //51
    "TypeDef",         //52
    "Indent",          //53
    "Unindent",        //54
	"EndOfInput",      //55
    "Concat",          //56
    "Malformed String" //57
};

int main(int argc, const char *argv[]) //Main entry point
{
    FILE *input = NULL;
 
    if(argc >= 3){
        input = fopen(argv[argc-1], "r");

        if(input == NULL){
            printf("File not found.");
            return 2; //File not found
        }

        if(strcmp(argv[1], "--lex") == 0 || strcmp(argv[1], "-l") == 0){
            puts("Now lexing...");
            src = input;

            initialize_lexer(0);
            Token *toks = lexer_next(0);
            int i = 0;
            while(toks[i].type != Tok_EndOfInput){
                for(; toks[i].type != Tok_EndOfInput; i++)
                {
                    switch(toks[i].type){
                        case Tok_Newline: case Tok_Indent: case Tok_Unindent:
                            printf("     \t%s\n", tokenDictionary[toks[i].type]);
                            break;
                        default:
                            printf("%s \t%s\n", toks[i].lexeme, tokenDictionary[toks[i].type]);
                            break;
                    }
                    free(toks[i].lexeme);
                }
            }
            free(toks);
        }else if(strcmp(argv[1], "--exec") == 0 || strcmp(argv[1], "--interpret") == 0 || strcmp(argv[1], "-i") == 0){
            interpret(input, 0);
        }else if(strcmp(argv[1], "--show") == 0 || strcmp(argv[1], "-s") == 0){
            while(!feof(input)) printf("%c", fgetc(input)); 
        }else{
            printf("Unrecognized option \"%s\"\n", argv[1]);
        }

        fclose(input);
    }else if(argc == 1){
        interpret(input, 1);
    }
    return 0;
}
