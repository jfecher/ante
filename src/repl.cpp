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
            if(inp == '\n'){
                if(line.back() == '\\'){
                    line.back() = '\n';
                }else{
                    //append the line to the history before the newline is added
                    appendHistory(line);
                    sl_history_pos = sl_history.size();
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

            auto *l = new Lexer(nullptr, line, 1, 1, true);

            loadPos();
            LOC_TY loc;
            printf("\033[2K\r: ");
            while(l->next(&loc));

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
