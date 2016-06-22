#include "parser.h"
using namespace ante::parser;


inline void maybePrintArr(Node *n){
    if(n){
        cout << ", ";
        n->print();
    }
}

void IntLitNode::print(){
    cout << val;
}

void FltLitNode::print(){
    cout << val;
}

void BoolLitNode::print(){
    if(val)
        cout << "true";
    else
        cout << "false";
}

void StrLitNode::print(){
    cout << '"' << val << '"';
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

void TypeNode::print(){
    cout << typeNodeToStr(this);
}

void TypeCastNode::print(){
    putchar('(');
    typeExpr->print();
    putchar(' ');
    rval->print();
    putchar(')');
}

void UnOpNode::print(){
    putchar('(');
    Lexer::printTok(op);
    putchar(' ');
    rval->print();
    putchar(')');
}

void BinOpNode::print(){
    if(op == '('){
        lval->print();
        rval->print();
    }else{
        putchar('(');
        if(lval) lval->print();
        putchar(' ');
        Lexer::printTok(op);
        putchar(' ');
        if(rval) rval->print();
        putchar(')');
    }
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
}

void VarNode::print(){
    cout << name;
}


void LetBindingNode::print(){
    cout << "let ";
    if(typeExpr.get()){
        typeExpr->print();
    }
    cout << ' ' << name << " = ";
    
    expr->print(); //expr is not null-checked since it is required to be non-null
    putchar('\n');
}

void VarDeclNode::print(){
    cout << "varDecl ";
    if(typeExpr) typeExpr->print();
    cout << ' ' << name << " = ";
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
    puts("");
    methods->print();
    puts("end ext");
}

void WhileNode::print(){
    cout << "while ";
    condition->print();
    puts(" do");
    body->print();
    puts("end while");
}

void FuncDeclNode::print(){
    cout << "fun ";
    cout << name;
    if(params){
        cout << ": ";
        params->print();
    }
    if(type){
        cout << " => ";
        type->print();
    }
    if(body.get()){
        puts("\nfnbody:");
        body->print();
        puts("\nendfn");
    }
}

void DataDeclNode::print(){
    cout << "data " << name << endl;
    body->print();
}
