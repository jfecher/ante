#include "compiler.h"
#include "target.h"

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
    cout << color << s << AN_CONSOLE_RESET;
}
#endif

/*
 *  Prints a given line (row) of a file, along with an arrow pointing to
 *  the specified column.
 */
void printErrLine(const yy::location& loc){
    if(!loc.begin.filename) return;
    ifstream f{*loc.begin.filename};

    //Premature newline error, show previous line as error instead
    auto line_start = loc.begin.column == 0 ? loc.begin.line - 1 : loc.begin.line;

    //skip to line in question
    skipToLine(f, line_start);

    //print line
    string s;
    getline(f, s);
    
    auto col_start = loc.begin.column == 0 ? s.length() + 1 : loc.begin.column;

    cout << s;

    //draw arrow
    putchar('\n');
    cout << AN_ERR_COLOR;
    unsigned int i = 1;

    //skip to begin pos
    for(; i < col_start; i++) putchar(' ');

    //draw arrow until end pos
    for(; i <= loc.end.column; i++) putchar('^');

    cout << AN_CONSOLE_RESET; //reset color
}

void printFileNameAndLineNumber(const yy::location& loc){
	if (loc.begin.filename)
		cout << AN_CONSOLE_ITALICS << *loc.begin.filename << AN_CONSOLE_RESET << ": ";
	else
		cout << AN_CONSOLE_ITALICS << "(unknown file)" << AN_CONSOLE_RESET << ": ";

    cout << AN_CONSOLE_BOLD << loc.begin.line << ",";
    if(loc.begin.column == loc.end.column)
        cout << loc.begin.column << AN_CONSOLE_RESET;
    else
        cout << loc.begin.column << '-' << loc.end.column << AN_CONSOLE_RESET;
}

void ante::error(const char* msg, const yy::location& loc){
    printFileNameAndLineNumber(loc);

    cout << '\t' << AN_ERR_COLOR << "error: " << AN_CONSOLE_RESET << msg << endl;
    printErrLine(loc);
    cout << endl << endl;
}

namespace ante {
    void error(ante::lazy_printer strs, const yy::location& loc){
        printFileNameAndLineNumber(loc);

        cout << '\t' << AN_ERR_COLOR << "error: " << AN_CONSOLE_RESET << strs << endl;

        printErrLine(loc);
        cout << endl << endl;
    }
}


/*
 *  Inform the user of an error and return nullptr.
 *  (perhaps this should throw an exception?)
 */
/*TypedValue* Compiler::compErr(const string msg, const yy::location& loc){
    error(msg.c_str(), loc);
    errFlag = true;
    return nullptr;
}*/

TypedValue* Compiler::compErr(ante::lazy_printer msg, const yy::location& loc){
    error(msg, loc);
    errFlag = true;
    return nullptr;
}


lazy_str typeNodeToColoredStr(const TypeNode *tn){
    lazy_str s = typeNodeToStr(tn);
    s.fmt = AN_TYPE_COLOR;
    return s;
}

lazy_str typeNodeToColoredStr(const unique_ptr<TypeNode>& tn){
    lazy_str s = typeNodeToStr(tn.get());
    s.fmt = AN_TYPE_COLOR;
    return s;
}
