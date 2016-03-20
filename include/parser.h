#ifndef PARSER_H
#define PARSER_H

#include <vector>
#include <memory> //For unique_ptr
#include "lexer.h"
#include "tokens.h"
#include "compiler.h"

enum ParseErr{
    PE_OK,
    PE_EXPECTED,
    PE_VAL_NOT_FOUND,
    PE_IDENT_NOT_FOUND,
    PE_INVALID_STMT,
};

using namespace llvm;
using namespace ante;

/* Base class for all nodes */
struct Node{
    unique_ptr<Node> next;
    Node *prev;
    unsigned int row, col;

    //print representation of node
    virtual void print(void) = 0;

    //compile node to a given module
    virtual TypedValue* compile(Compiler*) = 0;

    virtual ~Node(){}
};

/*
 *  Define Node* as the intermediate type for parsing functions and include
 *  the actual parser header.
 */
#ifndef YYSTYPE
#define YYSTYPE Node*
#endif

#include "yyparser.h"

/*
 * Class for all nodes that can contain child statement nodes,
 * if statements, function declarations, etc
 */
struct ParentNode : public Node{
    unique_ptr<Node> child;

    /*
        * The body should always be known when a
        * parent node is initialized, so it is required
        * in the constructor (unlike next and prev)
        */
    ParentNode(Node* c) : Node(), child(c){}
    ~ParentNode(){}
};

struct IntLitNode : public Node{
    string val;
    TypeTag type;
    TypedValue* compile(Compiler*);
    void print();
    IntLitNode(string s, TypeTag ty) : Node(), val(s), type(ty){}
    ~IntLitNode(){}
};

struct FltLitNode : public Node{
    string val;
    TypeTag type;
    TypedValue* compile(Compiler*);
    void print(void);
    FltLitNode(string s, TypeTag ty) : Node(), val(s), type(ty){}
    ~FltLitNode(){}
};

struct BoolLitNode : public Node{
    bool val;
    TypedValue* compile(Compiler*);
    void print(void);
    BoolLitNode(char b) : Node(), val(b){}
    ~BoolLitNode(){}
};

struct ArrayNode : public Node{
    vector<Node*> exprs;
    TypedValue* compile(Compiler*);
    void print(void);
    ArrayNode(vector<Node*>& e) : Node(), exprs(e){}
    ~ArrayNode(){}
};

struct TupleNode : public Node{
    vector<Node*> exprs;
    TypedValue* compile(Compiler*);
    vector<Value*> unpack(Compiler*);
    void print(void);
    TupleNode(vector<Node*>& e) : Node(), exprs(e){}
    ~TupleNode(){}
};

struct UnOpNode : public Node{
    int op;
    unique_ptr<Node> rval;
    TypedValue* compile(Compiler*);
    void print(void);
    UnOpNode(int s, Node *rv) : Node(), op(s), rval(rv){}
    ~UnOpNode(){}
};

struct BinOpNode : public Node{
    int op;
    unique_ptr<Node> lval, rval;
    TypedValue* compile(Compiler*);
    void print(void);
    BinOpNode(int s, Node *lv, Node *rv) : Node(), op(s), lval(lv), rval(rv){}
    ~BinOpNode(){}
};

struct TypeNode : public Node{
    TypeTag type;
    string typeName; //used for usertypes
    unique_ptr<TypeNode> extTy; //Used for pointers and non-single anonymous types.

    TypedValue* compile(Compiler*);
    void print(void);
    TypeNode(TypeTag ty, string tName, TypeNode* eTy) : Node(), type(ty), typeName(tName), extTy(eTy){}
    ~TypeNode(){}
};

struct ModNode : public Node{
    int mod;
    TypedValue* compile(Compiler*);
    void print(void);
    ModNode(int m) : Node(), mod(m){}
    ~ModNode(){}
};

struct RetNode : public Node{
    unique_ptr<Node> expr;
    TypedValue* compile(Compiler*);
    void print(void);
    RetNode(Node* e) : Node(), expr(e){}
    ~RetNode(){}
};

struct NamedValNode : public Node{
    string name;
    unique_ptr<Node> typeExpr;
    TypedValue* compile(Compiler*);
    void print(void);
    NamedValNode(string s, Node* t) : Node(), name(s), typeExpr(t){}
    ~NamedValNode(){}
};

struct VarNode : public Node{
    string name;
    TypedValue* compile(Compiler*);
    void print(void);
    VarNode(string s) : Node(), name(s){}
    ~VarNode(){}
};

struct RefVarNode : public Node{
    string name;
    TypedValue* compile(Compiler*);
    void print(void);
    RefVarNode(string s) : Node(), name(s){}
    ~RefVarNode(){}
};

struct FuncCallNode : public Node{
    string name;
    unique_ptr<TupleNode> params;
    TypedValue* compile(Compiler*);
    void print(void);
    FuncCallNode(string s, TupleNode* p) : Node(), name(s), params(p){}
    ~FuncCallNode(){}
};

struct StrLitNode : public Node{
    string val;
    TypedValue* compile(Compiler*);
    void print(void);
    StrLitNode(string s) : Node(), val(s){}
    ~StrLitNode(){}
};

struct LetBindingNode : public Node{
    string name;
    unique_ptr<Node> modifiers, typeExpr, expr;

    TypedValue* compile(Compiler*);
    void print(void);
    LetBindingNode(string s, Node *mods, Node* t, Node* exp) : Node(), name(s), modifiers(mods), typeExpr(t), expr(exp){}
    ~LetBindingNode(){}
};

struct VarDeclNode : public Node{
    string name;
    unique_ptr<Node> modifiers, typeExpr, expr;

    TypedValue* compile(Compiler*);
    void print(void);
    VarDeclNode(string s, Node *mods, Node* t, Node* exp) : Node(), name(s), modifiers(mods), typeExpr(t), expr(exp){}
    ~VarDeclNode(){}
};

struct VarAssignNode : public Node{
    unique_ptr<Node> ref_expr;
    unique_ptr<Node> expr;
    TypedValue* compile(Compiler*);
    void print(void);
    VarAssignNode(Node* v, Node* exp) : Node(), ref_expr(v), expr(exp){}
    ~VarAssignNode(){}
};

struct IfNode : public ParentNode{
    unique_ptr<Node> condition;
    unique_ptr<IfNode> elseN;
    TypedValue* compile(Compiler*);
    void print(void);
    IfNode(Node* n1, Node* body, IfNode* els) : ParentNode(body), condition(n1), elseN(els){}
    ~IfNode(){}
};

struct FuncDeclNode : public ParentNode{
    string name;
    unique_ptr<Node> modifiers, type;
    unique_ptr<NamedValNode> params;
    bool varargs;

    TypedValue* compile(Compiler*);
    void print(void);
    FuncDeclNode(string s, Node *mods, Node *t, Node *p, Node* b, bool va=false) : ParentNode(b), name(s), modifiers(mods), type(t), params((NamedValNode*)p), varargs(va){}
    ~FuncDeclNode(){}
};

struct DataDeclNode : public ParentNode{
    string name;
    size_t fields;

    TypedValue* compile(Compiler*);
    void print(void);
    DataDeclNode(string s, Node* b, size_t f) : ParentNode(b), name(s), fields(f){}
    ~DataDeclNode(){}
};


namespace ante{
    namespace parser{
        Node* getRootNode();
        void printBlock(Node *block);
        void parseErr(ParseErr e, string s, bool showTok);
    }
}

void printErrLine(const char* fileName, unsigned int row, unsigned int col);

#endif
