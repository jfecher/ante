#include "parser.h"
#include "types.h"

using namespace ante::parser;
using namespace ante;
using namespace std;

inline void maybePrintArr(Node *n){
    if(n){
        cout << ", ";
        n->print();
    }
}

inline void printSpaceDelimitedList(Node *n){
    Node *nxt = n;
    while(nxt){
        nxt->print();
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
        block->print();
        block = block->next.get();
        cout << endl;
    }
}

void RootNode::print(){
    puts("Types:");
    for(auto& f : types){ f->print(); puts("\n"); }

    puts("\n\nFunctions:");
    for(auto& f : funcs){ f->print(); puts("\n"); }

    puts("\n\nTraits:");
    for(auto& f : traits){ f->print(); puts("\n"); }

    puts("\n\nExtensions:");
    for(auto& f : extensions){ f->print(); puts("\n"); }

    puts("\n\nMain:");
    for(auto& f : main){ f->print(); puts(";"); }
}

void IntLitNode::print(){
    cout << val;
    maybePrintArr(next.get());
}

void FltLitNode::print(){
    cout << val;
    maybePrintArr(next.get());
}

void BoolLitNode::print(){
    if(val)
        cout << "true";
    else
        cout << "false";
    maybePrintArr(next.get());
}

void StrLitNode::print(){
    cout << '"' << val << '"';
    maybePrintArr(next.get());
}

void CharLitNode::print(){
    cout << '\'' << val << '\'';
    maybePrintArr(next.get());
}

void ArrayNode::print(){
    putchar('[');
    /*for(size_t i = 0; i < exprs.size(); i++){
        exprs[i]->print();
        if(i != exprs.size() - 1){
            cout << ", ";
        }
    }*/
    exprs[0]->print();
    putchar(']');
}

void TupleNode::print(){
    putchar('(');
    for(auto &n : exprs){
        n->print();
        if(n != exprs.back())
            cout << ", ";
    }
    putchar(')');
}

void ModNode::print(){
    Lexer::printTok(mod);
}

void PreProcNode::print(){
    cout << "![";
    expr->print();
    puts("]");
}


void TypeNode::print(){
    cout << typeNodeToStr(this);
}

void TypeCastNode::print(){
    putchar('(');
    typeExpr->print();
    putchar(' ');
    rval->print();
    putchar(')');
    maybePrintArr(next.get());
}

void UnOpNode::print(){
    putchar('(');
    Lexer::printTok(op);
    putchar(' ');
    rval->print();
    putchar(')');
    maybePrintArr(next.get());
}

void SeqNode::print(){
    for(auto &n : sequence){
        n->print();
        puts(";");
    }
}

void BinOpNode::print(){
    if(op == '('){
        lval->print();
        rval->print();
    }else{
        putchar('(');
        lval->print();
        putchar(' ');
        Lexer::printTok(op);
        putchar(' ');
        rval->print();
        putchar(')');
    }
}

void BlockNode::print(){
    puts("{");
    block->print();
    cout << "\n}" << flush;
}

void RetNode::print(){
    cout << "return ";
    if(expr) expr->print();
}

void ImportNode::print(){
    cout << "import ";
    expr->print();
}


void IfNode::print(){
    cout << "if ";
    condition->print();
    puts(" then");
    thenN->print();
    if(elseN){
        puts("\nelse");
        elseN->print();
    }
}

void NamedValNode::print(){
    if(typeExpr.get() == (void*)1)
        cout << "self";
    else if(typeExpr.get())
        typeExpr->print();
    else
        cout << "..."; //varargs

    putchar(' ');
    cout << name << flush;

    maybePrintArr(next.get());
}

void VarNode::print(){
    cout << name << flush;
    maybePrintArr(next.get());
}


void LetBindingNode::print(){
    cout << "let ";
    if(typeExpr.get()){
        typeExpr->print();
        putchar(' ');
    }
    cout << name << " = ";

    expr->print(); //expr is not null-checked since it is required to be non-null
}

void VarDeclNode::print(){
    cout << "varDecl ";
    if(typeExpr){
        typeExpr->print();
        putchar(' ');
    }
    cout << name << " = ";
    if(expr) expr->print();
    else cout << "(undef)";
}

void GlobalNode::print(){
    cout << "global ";
    vars[0]->print();
    puts("");
}

void VarAssignNode::print(){
    cout << "varAssign ";
    if(ref_expr) ref_expr->print();
    cout << " = ";
    if(expr) expr->print();
    else cout << "(undef)";
}

void ExtNode::print(){
    cout << "ext ";
    typeExpr->print();
    cout << "\n";
    printBlock(methods.get());
    cout << "end ext";
}

void JumpNode::print(){
    if(jumpType == Tok_Continue)
        cout << "continue ";
    else
        cout << "break ";

    expr->print();
}

void WhileNode::print(){
    cout << "while ";
    condition->print();
    puts(" do ");
    child->print();
}

void ForNode::print(){
    cout << "for " << var << " in ";
    range->print();
    puts(" do ");
    child->print();
}

void MatchNode::print(){
    cout << "match ";
    expr->print();
    puts(" with");
    for(auto& b : branches)
        b->print();
    puts("end match");
}

void MatchBranchNode::print(){
    cout << "| ";
    pattern->print();
    cout << " -> ";
    branch->print();
    putchar('\n');
}

void FuncDeclNode::print(){
    if(modifiers.get()){
        printSpaceDelimitedList(modifiers.get());
    }

    cout << "fun ";
    cout << name;
    if(params){
        cout << ": ";
        params->print();
    }
    if(type){
        cout << " -> ";
        type->print();
    }
    if(child.get()){
        cout << " = ";
        child->print();
    }
}

void DataDeclNode::print(){
    cout << "type " << name;
    if(!generics.empty()){
        cout << "<";
        for(size_t i = 0; i < generics.size(); i++){
            cout << typeNodeToStr(generics[i].get());
            if(i != generics.size()-1){
                cout << ", ";
            }
        }
        cout << ">";
    }
    cout << " = ";

    auto *nvn = (NamedValNode*)child.get();

    if(((TypeNode*)nvn->typeExpr.get())->type == TT_TaggedUnion){
        cout << endl;
        while(nvn && ((TypeNode*)nvn->typeExpr.get())->type == TT_TaggedUnion){
            auto *ty = (TypeNode*)nvn->typeExpr.get();

            cout << "| " << nvn->name << " " << (ty->extTy.get() ? typeNodeToStr(ty->extTy.get()) : "") << endl;
            nvn = (NamedValNode*)nvn->next.get();
        }
    }else{
        child->print();
    }
}

void TraitNode::print(){
    cout << "trait " << name << endl;
    printBlock(child.get());
    cout << "end of trait " << name << endl;
}
