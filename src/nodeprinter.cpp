#define NOMINMAX

#include "parser.h"
#include "types.h"

using namespace ante::parser;
using namespace ante;
using namespace std;

inline void maybePrintArr(Node *n){
    if(n){
        cout << ", ";
        PrintingVisitor::print(n);
    }
}

inline void printSpaceDelimitedList(Node *n){
    Node *nxt = n;
    while(nxt){
        PrintingVisitor::print(nxt);
        nxt = nxt->next.get();
        if(nxt) putchar(' ');
    }
}

/*
 *  Prints a list of nodes, can print
 *  entire parse tree if passed the root.
 */
void parser::printBlock(Node *block, size_t scope){
    while(block){
        for(size_t i = 0; i < scope; i++)
            putchar(' ');
        PrintingVisitor::print(block);
        block = block->next.get();
        cout << endl;
    }
}

void PrintingVisitor::visit(RootNode *n){
    for(auto& f : n->types){ f->accept(*this); puts("\n"); }

    for(auto& f : n->funcs){
        f->accept(*this); 
        cout << "  :  " << anTypeToColoredStr(f->getType());
        puts("\n");
    }

    for(auto& f : n->traits){ f->accept(*this); puts("\n"); }

    for(auto& f : n->extensions){ f->accept(*this); puts("\n"); }

    for(auto& f : n->main){
        f->accept(*this);
        puts(";");
    }
}

void printModifiers(PrintingVisitor &v, ModifiableNode *n){
    for(auto it = n->modifiers.rbegin(); it != n->modifiers.rend(); ++it){
        (*it)->accept(v);
        putchar(' ');
    }
}

void PrintingVisitor::visit(IntLitNode *n){
    cout << n->val;
    maybePrintArr(n->next.get());
}

void PrintingVisitor::visit(FltLitNode *n){
    cout << n->val;
    maybePrintArr(n->next.get());
}

void PrintingVisitor::visit(BoolLitNode *n){
    if(n->val)
        cout << "true";
    else
        cout << "false";
    maybePrintArr(n->next.get());
}

void PrintingVisitor::visit(StrLitNode *n){
    cout << '"' << n->val << '"';
    maybePrintArr(n->next.get());
}

void PrintingVisitor::visit(CharLitNode *n){
    cout << '\'' << n->val << '\'';
    maybePrintArr(n->next.get());
}

void PrintingVisitor::visit(ArrayNode *n){
    putchar('[');
    for(auto &e : n->exprs){
        e->accept(*this);
        if(&e != &n->exprs.back())
            cout << ", ";
    }
    putchar(']');
}

void PrintingVisitor::visit(TupleNode *n){
    putchar('(');
    for(auto &elem : n->exprs){
        elem->accept(*this);
        if(elem != n->exprs.back())
            cout << ", ";
    }
    putchar(')');
}

void PrintingVisitor::visit(ModNode *n){
    if(n->isCompilerDirective()){
        cout << "![";
        n->directive->accept(*this);
        putchar(']');
    }else{
        Lexer::printTok(n->mod);
    }
    if(n->expr){
        putchar(' ');
        n->expr->accept(*this);
    }
}

void PrintingVisitor::visit(TypeNode *n){
    printModifiers(*this, n);
    cout << typeNodeToStr(n);
}

void PrintingVisitor::visit(TypeCastNode *n){
    putchar('(');
    n->typeExpr->accept(*this);
    putchar(' ');
    n->rval->accept(*this);
    putchar(')');
    maybePrintArr(n->next.get());
}

void PrintingVisitor::visit(UnOpNode *n){
    putchar('(');
    Lexer::printTok(n->op);
    putchar(' ');
    n->rval->accept(*this);
    putchar(')');
    maybePrintArr(n->next.get());
}

void PrintingVisitor::visit(SeqNode *n){
    for(auto &c : n->sequence){
        c->accept(*this);

        if(&c != &n->sequence.back()){
            puts(";");

            cout << "  :  " << anTypeToStr(c->getType());

            for(size_t i = 0; i < indent_level; i++)
                putchar(' ');
        }
    }
}

void PrintingVisitor::visit(BinOpNode *n){
    if(n->op == '('){
        n->lval->accept(*this);

        n->rval->accept(*this);
    }else{
        n->lval->accept(*this);

        putchar(' ');
        Lexer::printTok(n->op);
        putchar(' ');
        n->rval->accept(*this);
    }
}

void PrintingVisitor::visit(BlockNode *n){
    puts("{");
    indent_level += 2;
    for(size_t i = 0; i < indent_level; i++)
        putchar(' ');
    n->block->accept(*this);
    indent_level -= 2;

    cout << endl;
    for(size_t i = 0; i < indent_level; i++)
        putchar(' ');
    cout << "}" << flush;
}

void PrintingVisitor::visit(RetNode *n){
    cout << "return ";
    if(n->expr) n->expr->accept(*this);
}

void PrintingVisitor::visit(ImportNode *n){
    cout << "import ";
    n->expr->accept(*this);
}


void PrintingVisitor::visit(IfNode *n){
    cout << "if ";
    n->condition->accept(*this);
    cout << " then ";

    n->thenN->accept(*this);
    if(n->elseN){
        cout << endl;
        for(size_t i = 0; i < indent_level; i++)
            putchar(' ');
        cout << "else ";

        n->elseN->accept(*this);
    }
}

void PrintingVisitor::visit(NamedValNode *n){
    if(n->typeExpr.get() == (void*)1)
        cout << "self";
    else if(n->typeExpr.get())
        n->typeExpr->accept(*this);
    else
        cout << "..."; //varargs

    putchar(' ');
    cout << n->name << ": " << anTypeToColoredStr(n->getType()) << flush;

    maybePrintArr(n->next.get());
}

void PrintingVisitor::visit(VarNode *n){
    cout << '(' << n->name << ": " << anTypeToColoredStr(n->getType()) << ')' << flush;
    maybePrintArr(n->next.get());
}


void PrintingVisitor::visit(VarAssignNode *n){
    printModifiers(*this, n);
    if(n->ref_expr) n->ref_expr->accept(*this);
    cout << " := ";
    if(n->expr) n->expr->accept(*this);
    else cout << "(undef)";
}

void PrintingVisitor::visit(ExtNode *n){
    printModifiers(*this, n);
    cout << "ext ";
    n->typeExpr->accept(*this);
    cout << "\n";
    printBlock(n->methods.get(), this->indent_level);
    cout << "end ext";
}

void PrintingVisitor::visit(JumpNode *n){
    if(n->jumpType == Tok_Continue)
        cout << "continue ";
    else
        cout << "break ";

    n->expr->accept(*this);
}

void PrintingVisitor::visit(WhileNode *n){
    cout << "while ";
    n->condition->accept(*this);
    cout << " do ";
    n->child->accept(*this);
}

void PrintingVisitor::visit(ForNode *n){
    cout << "for ";
    n->pattern->accept(*this);
    cout << " in ";
    n->range->accept(*this);
    cout << " do ";
    n->child->accept(*this);
}

void PrintingVisitor::visit(MatchNode *n){
    cout << "match ";
    n->expr->accept(*this);
    puts(" with");
    for(auto& b : n->branches){
        for(size_t i = 0; i < indent_level; i++)
            putchar(' ');
        b->accept(*this);
        cout << endl;
    }
    for(size_t i = 0; i < indent_level; i++)
        putchar(' ');
    cout << "end match";
}

void PrintingVisitor::visit(MatchBranchNode *n){
    cout << "| ";
    n->pattern->accept(*this);
    cout << " -> ";
    n->branch->accept(*this);
}

void PrintingVisitor::visit(FuncDeclNode *n){
    printModifiers(*this, n);
    bool isExtern = false;
    cout << "fun ";

    if(!n->name.empty() && n->name[n->name.size()-1] == ';'){
        isExtern = true;
        cout << n->name.substr(0, n->name.size()-1);
    }else{
        cout << n->name;
    }

    if(n->params){
        cout << ": ";
        n->params->accept(*this);
    }
    if(n->returnType){
        cout << " -> ";
        n->returnType->accept(*this);
    }
    if(n->child.get()){
        cout << " = ";
        n->child->accept(*this);
    }else if(isExtern){
        cout << ";";
    }
}

void PrintingVisitor::visit(DataDeclNode *n){
    printModifiers(*this, n);
    cout << "type " << n->name;
    if(!n->generics.empty()){
        cout << "<";
        for(size_t i = 0; i < n->generics.size(); i++){
            cout << typeNodeToStr(n->generics[i].get());
            if(i != n->generics.size()-1){
                cout << ", ";
            }
        }
        cout << ">";
    }
    cout << " = ";

    auto *nvn = (NamedValNode*)n->child.get();

    if(((TypeNode*)nvn->typeExpr.get())->typeTag == TT_TaggedUnion){
        cout << endl;
        while(nvn && ((TypeNode*)nvn->typeExpr.get())->typeTag == TT_TaggedUnion){
            auto *ty = (TypeNode*)nvn->typeExpr.get();

            cout << "| " << nvn->name << " " << (ty->extTy.get() ? typeNodeToStr(ty->extTy.get()) : "") << endl;
            nvn = (NamedValNode*)nvn->next.get();
        }
    }else{
        n->child->accept(*this);
    }
}

void PrintingVisitor::visit(TraitNode *n){
    printModifiers(*this, n);
    cout << "trait " << n->name << endl;
    printBlock(n->child.get(), this->indent_level);
    cout << "end of trait " << n->name << endl;
}
