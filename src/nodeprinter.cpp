#include "parser.h"
using namespace ante::parser;


inline void maybePrintArr(Node *n){
    if(n){
        cout << ", ";
        n->print();
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
    if(exprs.size() > 0)
        exprs[0]->print();
    putchar(')');
}

void ModNode::print(){
    Lexer::printTok(mod);
}


/* 
 * defined in types.cpp.
 *
 * could also just #include "compiler.h",
 * but that's a bit overkill for one function
 */
string typeNodeToStr(TypeNode*);

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

void BinOpNode::print(){
    if(op == '('){
        lval->print();
        rval->print();
    }else if(op == ';'){
        lval->print();
        puts(";");
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
    cout << "\n}";
}

void RetNode::print(){
    cout << "return ";
    if(expr) expr->print();
}

void ImportNode::print(){
    cout << "import ";
    expr->print();
}


//unlike IfNodes, an ExprIfNode's
//condition, thenN, and elseN are all
//guarenteed to be initialized
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
    if(typeExpr.get())
        typeExpr->print();
    else
        cout << "..."; //varargs

    putchar(' ');
    cout << name;

    maybePrintArr(next.get());
}

void VarNode::print(){
    cout << name;
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
    for(auto *b : branches)
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
    cout << "type " << name << " = ";
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
