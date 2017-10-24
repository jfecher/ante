#include "repl.h"
#include "target.h"
#include <vector>
#include <string>

#ifdef unix
#  include <unistd.h>
#  include <termios.h>
#  include <sys/ioctl.h>
#endif

using namespace std;
using namespace ante;
using namespace ante::parser;

extern char* lextxt;

namespace ante {

    unsigned int sl_pos = 0;
    unsigned int sl_history_pos = 0;
    vector<string> sl_history;

#ifdef unix
    winsize termSize;

#elif defined(WIN32)
#  include <windows.h>
#  define getchar getchar_windows

	HANDLE h_in, h_out;
	DWORD cc, normal_mode, getch_mode;

	TCHAR getchar_windows() {
		TCHAR c = 0;
		SetConsoleMode(h_in, getch_mode);
		ReadConsole(h_in, &c, 1, &cc, NULL);
		SetConsoleMode(h_in, normal_mode);
		return c;
	}

	void clearline_windows() {
		DWORD numCharsWritten;
		CONSOLE_SCREEN_BUFFER_INFO csbi;

		// Get the number of character cells in the current buffer.
		if (!GetConsoleScreenBufferInfo(h_out, &csbi)){
			cerr << "Cannot get screen buffer info" << endl;
			return;
		}

		COORD homeCoords = { 0, csbi.dwCursorPosition.Y };
		DWORD cellsToWrite = csbi.dwSize.X;

		// Fill the entire screen with blanks.
		if (!FillConsoleOutputCharacter(h_out, (TCHAR) ' ', cellsToWrite, homeCoords, &numCharsWritten)) {
			cerr << "Error when attempting to clear screen" << endl;
			return;
		}

		// Get the current text attribute.
		if (!GetConsoleScreenBufferInfo(h_out, &csbi)) {
			cerr << "Error when getting screen buffer info" << endl;
			return;
		}

		// Set the buffer's attributes accordingly.
		if (!FillConsoleOutputAttribute(h_out, csbi.wAttributes, cellsToWrite, homeCoords, &numCharsWritten)) {
			cerr << "Error when attempting to fill attributes" << endl;
			return;
		}

		SetConsoleCursorPosition(h_out, homeCoords);
	}
#endif

    void savePos(){
#ifdef unix
        printf("\033[s");
#elif defined(_WIN32)

#endif
    }

    void loadPos(){
#ifdef unix
        printf("\033[u");
#elif defined(_WIN32)

#endif
    }

    void clearScreen(){
#ifdef unix
        printf("\033[J");
#elif defined(_WIN32)
        clearline_windows();
        cout << ": ";
#endif
    }

    void newline(){
#ifdef unix
        printf("\033[2K\r: ");
#elif defined(_WIN32)

#endif
    }

    void updateTermSize(){
#ifdef unix
        winsize w;
        ioctl(0, TIOCGWINSZ, &w);
        if(w.ws_col != termSize.ws_col){
            termSize = w;
            clearScreen();
        }
#elif defined(_WIN32)

#endif
    }

    void appendHistory(string &line){
        if(!line.empty() and (sl_history.empty() or line != sl_history.back()))
            sl_history.push_back(line);
    }

    void handleEscSeq(string &line){
        if(getchar() == '['){
            char escSeq = getchar();
            if(escSeq == 68 and sl_pos > 0){ //move left
                sl_pos--;
            }else if(escSeq == 67 and sl_pos < line.length()){ //right
                sl_pos++;
            }else if(escSeq == 65){ //up
                if(sl_history_pos == sl_history.size()){
                    appendHistory(line);
                }

                if(sl_history_pos > 0){
                    sl_history_pos--;
                    line = sl_history[sl_history_pos];
                }
            }else if(escSeq == 66){ //down
                if(sl_history_pos < sl_history.size() - 1){
                    sl_history_pos++;
                    line = sl_history[sl_history_pos];
                }else if(sl_history_pos == sl_history.size() - 1){
                    line = "";
                    sl_history_pos++;
                }
            }
        }
    }

    string getInputColorized(){
        string line = "";

        savePos();
        cout << ": " << flush;
        char inp = getchar();

        while(inp){
            updateTermSize();

            //stop on newlines unless there is a \ before.
            if(inp == '\n' or inp == '\r'){
                if(line.back() == '\\'){
                    if(inp == '\r'){
                        line.back() = '\r';
                        line += '\n';
                    }else{
                        line.back() = '\n';
                    }
                }else{
                    //append the line to the history before the newline is added
                    appendHistory(line);
                    sl_history_pos = sl_history.size();
                    if(inp == '\r') line += '\r';
                    line += '\n';
                    break;
                }
            }else if(inp == '\t'){
                line += "    "; //replace tabs with 4 spaces
            }else if(inp == '\b' or inp == 127){
                if(!line.empty())
                    line = line.substr(0, line.length() - 1);
            }else if(inp == '\033'){
                handleEscSeq(line);
            }else{
                line += inp;
            }

#if defined(unix) || defined(_WIN32)
            loadPos();
            newline();

            LOC_TY loc;
			auto *l = new Lexer(nullptr, line, 1, 1, true);
			while (l->next(&loc)){ /* do nothing*/ };
#endif

            inp = getchar();
        }

        puts("");
        return line;
    }

    void setupTerm(){
#ifdef unix
        termios newt;
        tcgetattr(STDIN_FILENO, &newt);
        newt.c_lflag &= ~(ICANON | ECHO);
        tcsetattr(STDIN_FILENO, TCSANOW, &newt);
        ioctl(0, TIOCGWINSZ, &termSize);
#elif defined WIN32
		h_in = GetStdHandle(STD_INPUT_HANDLE);
		h_out = GetStdHandle(STD_OUTPUT_HANDLE);
		if (!h_in or !h_out) {
			fputs("Error when attempting to access windows terminal\n", stderr);
			exit(1);
		}
		GetConsoleMode(h_in, &normal_mode);
		getch_mode = normal_mode & ~(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
#endif
    }


    void startRepl(Compiler *c){
        cout << "Ante REPL v0.0.6\nType 'exit' to exit.\n";
        setupTerm();

        auto cmd = getInputColorized();

        while(cmd != "exit\n"){
            int flag;
            //Catch any lexing errors
            try{
                //lex and parse the new string
                setLexer(new Lexer(nullptr, cmd, /*line*/1, /*col*/1));
                yy::parser p{};
                flag = p.parse();
            }catch(CtError *e){
                delete e;
                continue;
            }

            if(flag == PE_OK){
                RootNode *expr = parser::getRootNode();

                //Compile each expression and hold onto the last value
                TypedValue val = c->ast ? mergeAndCompile(c, expr)
                                        : (c->ast.reset(expr), expr->compile(c));

                //print val if it's not an error
                if(!!val and val.type->typeTag != TT_Void)
                    val.dump();
            }

            cmd = getInputColorized();
        }
    }

    TypedValue mergeAndCompile(Compiler *c, RootNode *rn){
        scanImports(c, rn);
        move(rn->imports.begin(),
            next(rn->imports.begin(), rn->imports.size()),
            back_inserter(c->ast->imports));

        for(auto &t : rn->types){
            safeCompile(c, t);
            c->ast->types.emplace_back(move(t));
        }

        for(auto &t : rn->traits){
            safeCompile(c, t);
            c->ast->traits.emplace_back(move(t));
        }

        for(auto &t : rn->extensions){
            safeCompile(c, t);
            c->ast->extensions.emplace_back(move(t));
        }

        for(auto &t : rn->funcs){
            safeCompile(c, t);
            c->ast->funcs.emplace_back(move(t));
        }

        TypedValue ret;
        for(auto &e : rn->main){
            ret = safeCompile(c, e);
            c->ast->main.emplace_back(move(e));
        }
        return ret;
    }
}
