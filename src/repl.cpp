#include "repl.h"
#include "target.h"

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

#ifdef unix
    winsize termSize;
#endif

    string getInputColorized(){
        string line = "";

        cout << ": " << flush;
        char inp = getchar();

        while(inp and inp != '\n'){
            if(inp == '\b' or inp == 127){
                if(!line.empty())
                    line = line.substr(0, line.length() - 1);
            }else if(inp == '\033'){
                getchar();
                inp = getchar();
                continue;
            }else{
                line += inp;
            }

            auto *l = new Lexer(nullptr, line, 1, 1, true);

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
        cout << "Ante REPL v0.0.4\nType 'exit' to exit.\n";
        setupTerm();

        auto cmd = getInputColorized();

        while(cmd != "exit"){
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
