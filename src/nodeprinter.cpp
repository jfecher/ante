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
void parser::printBlock(Node *block){
    while(block){
        PrintingVisitor::print(block);
        block = block->next.get();
        cout << endl;
    }
}

void PrintingVisitor::visit(RootNode *n){
    puts("Types:");
    for(auto& f : n->types){ f->accept(*this); puts("\n"); }

    puts("\n\nFunctions:");
    for(auto& f : n->funcs){ f->accept(*this); puts("\n"); }

    puts("\n\nTraits:");
    for(auto& f : n->traits){ f->accept(*this); puts("\n"); }

    puts("\n\nExtensions:");
    for(auto& f : n->extensions){ f->accept(*this); puts("\n"); }

    puts("\n\nMain:");
    for(auto& f : n->main){ f->accept(*this); puts(";"); }
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
    n->exprs[0]->accept(*this);
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
        n->expr->accept(*this);
        puts("]");
    }else{
        Lexer::printTok(n->mod);
        putchar(' ');
    }
}

void PrintingVisitor::visit(TypeNode *n){
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
    for(auto &n : n->sequence){
        n->accept(*this);
        puts(";");
    }
}

void PrintingVisitor::visit(BinOpNode *n){
    if(n->op == '('){
        n->lval->accept(*this);
        n->rval->accept(*this);
    }else{
        putchar('(');
        n->lval->accept(*this);
        putchar(' ');
        Lexer::printTok(n->op);
        putchar(' ');
        n->rval->accept(*this);
        putchar(')');
    }
}

void PrintingVisitor::visit(BlockNode *n){
    puts("{");
    n->block->accept(*this);
    cout << "\n}" << flush;
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
    puts(" then");
    n->thenN->accept(*this);
    if(n->elseN){
        puts("\nelse");
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
    cout << n->name << flush;

    maybePrintArr(n->next.get());
}

void PrintingVisitor::visit(VarNode *n){
    cout << n->name << flush;
    maybePrintArr(n->next.get());
}


void PrintingVisitor::visit(LetBindingNode *n){
    cout << "let ";
    if(n->typeExpr.get()){
        n->typeExpr->accept(*this);
        putchar(' ');
    }
    cout << n->name << " = ";

    n->expr->accept(*this); //expr is not null-checked since it is required to be non-null
}

void PrintingVisitor::visit(VarDeclNode *n){
    cout << "varDecl ";
    if(n->typeExpr){
        n->typeExpr->accept(*this);
        putchar(' ');
    }
    cout << n->name << " = ";
    if(n->expr) n->expr->accept(*this);
    else cout << "(undef)";
}

void PrintingVisitor::visit(GlobalNode *n){
    cout << "global ";
    n->vars[0]->accept(*this);
    puts("");
}

void PrintingVisitor::visit(VarAssignNode *n){
    cout << "varAssign ";
    if(n->ref_expr) n->ref_expr->accept(*this);
    cout << " = ";
    if(n->expr) n->expr->accept(*this);
    else cout << "(undef)";
}

void PrintingVisitor::visit(ExtNode *n){
    cout << "ext ";
    n->typeExpr->accept(*this);
    cout << "\n";
    printBlock(n->methods.get());
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
    puts(" do ");
    n->child->accept(*this);
}

void PrintingVisitor::visit(ForNode *n){
    cout << "for " << n->var << " in ";
    n->range->accept(*this);
    puts(" do ");
    n->child->accept(*this);
}

void PrintingVisitor::visit(MatchNode *n){
    cout << "match ";
    n->expr->accept(*this);
    puts(" with");
    for(auto& b : n->branches)
        b->accept(*this);
    puts("end match");
}

void PrintingVisitor::visit(MatchBranchNode *n){
    cout << "| ";
    n->pattern->accept(*this);
    cout << " -> ";
    n->branch->accept(*this);
    putchar('\n');
}

void PrintingVisitor::visit(FuncDeclNode *n){
    bool isExtern = false;
    if(n->modifiers.get()){
        printSpaceDelimitedList(n->modifiers.get());
    }

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
    if(n->type){
        cout << " -> ";
        n->type->accept(*this);
    }
    if(n->child.get()){
        cout << " = ";
        n->child->accept(*this);
    }else if(isExtern){
        cout << ";";
    }
}

void PrintingVisitor::visit(DataDeclNode *n){
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

    if(((TypeNode*)nvn->typeExpr.get())->type == TT_TaggedUnion){
        cout << endl;
        while(nvn && ((TypeNode*)nvn->typeExpr.get())->type == TT_TaggedUnion){
            auto *ty = (TypeNode*)nvn->typeExpr.get();

            cout << "| " << nvn->name << " " << (ty->extTy.get() ? typeNodeToStr(ty->extTy.get()) : "") << endl;
            nvn = (NamedValNode*)nvn->next.get();
        }
    }else{
        n->child->accept(*this);
    }
}

void PrintingVisitor::visit(TraitNode *n){
    cout << "trait " << n->name << endl;
    printBlock(n->child.get());
    cout << "end of trait " << n->name << endl;
}
