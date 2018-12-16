#include "compiler.h"
#include "target.h"
#include "error.h"
#include "types.h"

using namespace std;
using namespace ante::parser;

namespace ante {

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


void setTermFGColor(AN_COLOR_TYPE fg){
    cout << fg;
}

/*
 *  Prints a given line (row) of a file, along with an arrow pointing to
 *  the specified column.
 */
void printErrLine(const yy::location& loc, ErrorType t){
    if(!loc.begin.filename) return;
    ifstream f{*loc.begin.filename};

    // highlight the whole first line if the error spans multiple lines
    unsigned int end_col = loc.begin.line == loc.end.line ? loc.begin.column : -1;

    //skip to line in question
    skipToLine(f, loc.begin.line);

    //print line
    string s;
    getline(f, s);

    for(size_t i = 0; i < s.size(); i++){
        if(i == loc.begin.column - 1){
            printErrorTypeColor(t);
        }else if(i == end_col){
            cout << AN_CONSOLE_RESET;
        }
        cout << s[i];
    }

    //draw arrow
    if(!colored_output){
        putchar('\n');
        printErrorTypeColor(t);
        unsigned int i = 1;

        //skip to begin pos and draw arrow until end pos
        for(; i < loc.begin.column; i++) putchar(' ');
        for(; i <= loc.end.column; i++) putchar('^');
    }

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


void showFileInfo(const yy::location &loc, ErrorType t){
    printFileNameAndLineNumber(loc);

    cout << '\t' << flush;
    printErrorTypeColor(t);

    if(t == ErrorType::Error)
        cout << "error: ";
    else if(t == ErrorType::Warning)
        cout << "warning: ";
    else if(t == ErrorType::Note)
        cout << "note: ";

    clearColor();
}


void showError(lazy_printer msg, const yy::location& loc, ErrorType t){
    showFileInfo(loc, t);
    cout << msg << endl;
    printErrLine(loc, t);
    cout << endl << endl;
}


void error(const char* msg, const yy::location& loc, ErrorType t){
    showError(msg, loc, t);
    throw CtError();
}

void error(lazy_printer strs, const yy::location& loc, ErrorType t){
    showError(strs, loc, t);
    throw CtError();
}


/*
 *  Inform the user of an error and return nullptr.
 */
TypedValue Compiler::compErr(lazy_printer msg, const yy::location& loc, ErrorType t){
    showError(msg, loc, t);
    if(t == ErrorType::Error){
        errFlag = true;
        throw new CompilationError(msg, loc);
    }
    return {};
}

TypedValue Compiler::compErr(lazy_printer msg, ErrorType t){
    auto loc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
    return compErr(msg, loc, t);
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

lazy_str anTypeToColoredStr(const AnType *t){
    lazy_str s = anTypeToStr(t);
    if(colored_output)
        s.fmt = AN_TYPE_COLOR;
    return s;
}

} //end of namespace ante
