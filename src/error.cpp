#include "compiler.h"
#include "target.h"
#include "error.h"

/* 
 * Skips input in a given istream until it encounters the given coordinates,
 * with each newline signalling the end of a row.
 *
 * precondition: coordinates must be valid
 */
void skipToLine(istream& ifs, unsigned int row){
    unsigned int line = 1;
    if(line != row){
        while(true){
            char c = ifs.get();
            if(c == '\n'){
                line++;
                if(line >= row){
                    break;
                }
            }else if(c == EOF){
                break;
            }
        }
    }
}

#ifdef _WIN32
void wrapInColor(string s, win_console_color color){
    cout << color << s << AN_CONSOLE_RESET;
}

#else
template<typename T>
void wrapInColor(string s, const char* color){
    if(colored_output){
        cout << color << s << AN_CONSOLE_RESET;
    }else{
        cout << s;
    }
}
#endif

void printErrorTypeColor(ErrorType t){
    if(colored_output){
        if(t == ErrorType::Error)
            cout << AN_ERR_COLOR;
        else if(t == ErrorType::Warning)
            cout << AN_WARN_COLOR;
        else
            cout << AN_NOTE_COLOR;
    }
}

void clearColor(){
    if(colored_output)
        cout << AN_CONSOLE_RESET;
}

/*
 *  Prints a given line (row) of a file, along with an arrow pointing to
 *  the specified column.
 */
void printErrLine(const yy::location& loc, ErrorType t){
    if(!loc.begin.filename) return;
    ifstream f{*loc.begin.filename};

    auto line_start = loc.begin.line;

    //skip to line in question
    skipToLine(f, line_start);

    //print line
    string s;
    getline(f, s);
   
    auto col_start = loc.begin.column;

    cout << s;

    //draw arrow
    putchar('\n');
    printErrorTypeColor(t);

    unsigned int i = 1;

    //skip to begin pos
    for(; i < col_start; i++) putchar(' ');

    //draw arrow until end pos
    for(; i <= loc.end.column; i++) putchar('^');

    clearColor();
}

void printFileNameAndLineNumber(const yy::location& loc){
    if(colored_output) cout << AN_CONSOLE_ITALICS;

	if (loc.begin.filename) cout << *loc.begin.filename;
	else cout << "(unknown file)";

    clearColor();
    cout << ": ";

    if(colored_output) cout << AN_CONSOLE_BOLD;
    cout << loc.begin.line << ",";

    if(loc.begin.column == loc.end.column) cout << loc.begin.column;
    else cout << loc.begin.column << '-' << loc.end.column;

    clearColor();
}

void ante::error(const char* msg, const yy::location& loc, ErrorType t){
    printFileNameAndLineNumber(loc);

    cout << '\t' << flush;
    printErrorTypeColor(t);

    if(t == Error)
        cout << "error: ";
    else if(t == Warning)
        cout << "warning: ";
    else if(t == Note)
        cout << "note: ";

    clearColor();
    cout << msg << endl;

    printErrLine(loc, t);
    cout << endl << endl;
}

namespace ante {
    void error(ante::lazy_printer strs, const yy::location& loc, ErrorType t){
        printFileNameAndLineNumber(loc);
    
        cout << '\t' << flush;
        printErrorTypeColor(t);

        if(t == Error)
            cout << "error: ";
        else if(t == Warning)
            cout << "warning: ";
        else if(t == Note)
            cout << "note: ";

        clearColor();
        cout << strs << endl;
        
        printErrLine(loc, t);
        cout << endl << endl;
    }
}


/*
 *  Inform the user of an error and return nullptr.
 */
TypedValue* Compiler::compErr(ante::lazy_printer msg, const yy::location& loc, ErrorType t){
    error(msg, loc, t);
    errFlag = t == ErrorType::Error;
    throw new CompilationError(msg, loc);
}


lazy_str typeNodeToColoredStr(const TypeNode *tn){
    lazy_str s = typeNodeToStr(tn);
    if(colored_output)
        s.fmt = AN_TYPE_COLOR;
    return s;
}

lazy_str typeNodeToColoredStr(const unique_ptr<TypeNode>& tn){
    lazy_str s = typeNodeToStr(tn.get());
    if(colored_output)
        s.fmt = AN_TYPE_COLOR;
    return s;
}
