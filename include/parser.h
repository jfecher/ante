#ifndef PARSER_H
#define PARSER_H

#include <vector>
#include <memory> //For unique_ptr
#include "lexer.h"
#include "tokens.h"
#include "location.hh"

namespace ante {

    enum ParseErr{
        PE_OK,
        PE_EXPECTED,
        PE_VAL_NOT_FOUND,
        PE_IDENT_NOT_FOUND,
        PE_INVALID_STMT,
    };

#ifndef LOC_TY
#  define LOC_TY yy::location
#endif

    /* Needed for compliancy with several versions of bison */
    yy::position mkPos(string *f, unsigned int line, unsigned int col);
    LOC_TY mkLoc(yy::position begin, yy::position end);


    /* forward-decls from compiler.h */
    struct TypedValue;
    struct Compiler;

    struct Node;

    struct NodeIterator {
        Node *cur;

        NodeIterator operator++();
        Node* operator*();
        bool operator==(NodeIterator r);
        bool operator!=(NodeIterator r);
    };

    /* Base class for all nodes */
    struct Node{
        unique_ptr<Node> next;
        Node *prev;
        LOC_TY loc;

        //print representation of node
        virtual void print(void) = 0;

        //compile node to a given module
        virtual TypedValue* compile(Compiler*) = 0;

        NodeIterator begin();
        NodeIterator end();

        Node(LOC_TY& l) : next(nullptr), prev(nullptr), loc(l){}
        virtual ~Node(){}
    };

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
        ParentNode(LOC_TY& loc, Node* c) : Node(loc), child(c){}
        ~ParentNode(){}
    };


    struct FuncDeclNode;
    struct TraitNode;
    struct ExtNode;
    struct DataDeclNode;
    struct ImportNode;

    /*
    * Specialized Node to act as root
    * - Separates top-level definitions from code that is compiled
    *   into the 'main' or "init_${module}" function
    */
    struct RootNode : public Node{
        //non-owning vectors (each decl is later moved into a ante::module)
        vector<FuncDeclNode*> funcs;
        vector<TraitNode*> traits;
        vector<ExtNode*> extensions;
        vector<DataDeclNode*> types;
        vector<unique_ptr<ImportNode>> imports;

        vector<unique_ptr<Node>> main;
        
        TypedValue* compile(Compiler*);
        void print();
        RootNode(LOC_TY& loc) : Node(loc){}
        ~RootNode(){}
    };


    struct IntLitNode : public Node{
        string val;
        TypeTag type;
        TypedValue* compile(Compiler*);
        void print();
        IntLitNode(LOC_TY& loc, string s, TypeTag ty) : Node(loc), val(s), type(ty){}
        ~IntLitNode(){}
    };

    struct FltLitNode : public Node{
        string val;
        TypeTag type;
        TypedValue* compile(Compiler*);
        void print(void);
        FltLitNode(LOC_TY& loc, string s, TypeTag ty) : Node(loc), val(s), type(ty){}
        ~FltLitNode(){}
    };

    struct BoolLitNode : public Node{
        bool val;
        TypedValue* compile(Compiler*);
        void print(void);
        BoolLitNode(LOC_TY& loc, char b) : Node(loc), val((bool) b){}
        ~BoolLitNode(){}
    };

    struct CharLitNode : public Node{
        char val;
        TypedValue* compile(Compiler*);
        void print(void);
        CharLitNode(LOC_TY& loc, char c) : Node(loc), val(c){}
        ~CharLitNode(){}
    };

    struct ArrayNode : public Node{
        vector<unique_ptr<Node>> exprs;
        TypedValue* compile(Compiler*);
        void print(void);
        ArrayNode(LOC_TY& loc, vector<unique_ptr<Node>>& e) : Node(loc), exprs(move(e)){}
        ~ArrayNode(){}
    };

    struct TupleNode : public Node{
        vector<unique_ptr<Node>> exprs;
        TypedValue* compile(Compiler*);

        vector<TypedValue*> unpack(Compiler*);
        void print(void);
        TupleNode(LOC_TY& loc, vector<unique_ptr<Node>>& e) : Node(loc), exprs(move(e)){}
        ~TupleNode(){}
    };

    struct UnOpNode : public Node{
        int op;
        unique_ptr<Node> rval;
        TypedValue* compile(Compiler*);
        void print(void);
        UnOpNode(LOC_TY& loc, int s, Node *rv) : Node(loc), op(s), rval(rv){}
        ~UnOpNode(){}
    };

    struct BinOpNode : public Node{
        int op;
        unique_ptr<Node> lval, rval;
        TypedValue* compile(Compiler*);
        void print(void);
        BinOpNode(LOC_TY& loc, int s, Node *lv, Node *rv) : Node(loc), op(s), lval(lv), rval(rv){}
        ~BinOpNode(){}
    };

    struct SeqNode : public Node{
        vector<unique_ptr<Node>> sequence;
        TypedValue* compile(Compiler*);
        void print(void);
        SeqNode(LOC_TY& loc) : Node(loc), sequence(){}
        ~SeqNode(){}
    };

    struct BlockNode : public Node{
        unique_ptr<Node> block;
        TypedValue* compile(Compiler*);
        void print(void);
        BlockNode(LOC_TY& loc, Node *b) : Node(loc), block(b){}
        ~BlockNode(){}
    };

    struct ModNode : public Node{
        int mod;
        TypedValue* compile(Compiler*);
        void print(void);
        ModNode(LOC_TY& loc, int m) : Node(loc), mod(m){}
        ~ModNode(){}
    };

    struct TypeNode : public Node{
        TypeTag type;
        string typeName; //used for usertypes
        unique_ptr<TypeNode> extTy; //Used for pointers and non-single anonymous types.
        vector<unique_ptr<TypeNode>> params; //type parameters for generic types
        vector<int> modifiers;

        unsigned int getSizeInBits(Compiler*, string* tn = 0);
        TypedValue* compile(Compiler*);
        void print(void);
        TypeNode* addModifiers(ModNode *m);
        TypeNode* addModifier(int m);
        void copyModifiersFrom(const TypeNode *tn);
        bool hasModifier(int m) const;
        TypeNode(LOC_TY& loc, TypeTag ty, string tName, TypeNode* eTy) : Node(loc), type(ty), typeName(tName), extTy(eTy), params(), modifiers(){}
        ~TypeNode(){}
    };

    struct TypeCastNode : public Node{
        unique_ptr<TypeNode> typeExpr;
        unique_ptr<Node> rval;
        TypedValue* compile(Compiler*);
        void print(void);
        TypeCastNode(LOC_TY& loc, TypeNode *ty, Node *rv) : Node(loc), typeExpr(ty), rval(rv){}
        ~TypeCastNode(){}
    };

    struct PreProcNode : public Node{
        shared_ptr<Node> expr;
        TypedValue* compile(Compiler*);
        void print(void);
        PreProcNode(LOC_TY& loc, Node* e) : Node(loc), expr(e){}
        PreProcNode(LOC_TY& loc, shared_ptr<Node> e) : Node(loc), expr(e){}
        ~PreProcNode(){}
    };

    struct RetNode : public Node{
        unique_ptr<Node> expr;
        TypedValue* compile(Compiler*);
        void print(void);
        RetNode(LOC_TY& loc, Node* e) : Node(loc), expr(e){}
        ~RetNode(){}
    };

    string typeNodeToStr(const TypeNode*);

    struct NamedValNode : public Node{
        string name;
        unique_ptr<Node> typeExpr;
        TypedValue* compile(Compiler*);
        void print(void);
        NamedValNode(LOC_TY& loc, string s, Node* t) : Node(loc), name(s), typeExpr(t){}
        ~NamedValNode(){}
    };

    struct VarNode : public Node{
        string name;
        TypedValue* compile(Compiler*);
        void print(void);
        VarNode(LOC_TY& loc, string s) : Node(loc), name(s){}
        ~VarNode(){}
    };

    struct GlobalNode : public Node{
        vector<unique_ptr<VarNode>> vars;
        TypedValue* compile(Compiler*);
        void print(void);
        GlobalNode(LOC_TY& loc, vector<unique_ptr<VarNode>> &vn) : Node(loc), vars(move(vn)){}
        ~GlobalNode(){}
    };

    struct StrLitNode : public Node{
        string val;
        TypedValue* compile(Compiler*);
        void print(void);
        StrLitNode(LOC_TY& loc, string s) : Node(loc), val(s){}
        ~StrLitNode(){}
    };

    struct LetBindingNode : public Node{
        string name;
        unique_ptr<Node> modifiers, typeExpr, expr;

        TypedValue* compile(Compiler*);
        void print(void);
        LetBindingNode(LOC_TY& loc, string s, Node *mods, Node* t, Node* exp) : Node(loc), name(s), modifiers(mods), typeExpr(t), expr(exp){}
        ~LetBindingNode(){}
    };

    struct VarDeclNode : public Node{
        string name;
        unique_ptr<Node> modifiers, typeExpr, expr;

        TypedValue* compile(Compiler*);
        void print(void);
        VarDeclNode(LOC_TY& loc, string s, Node *mods, Node* t, Node* exp) : Node(loc), name(s), modifiers(mods), typeExpr(t), expr(exp){}
        ~VarDeclNode(){}
    };

    struct VarAssignNode : public Node{
        Node* ref_expr;
        unique_ptr<Node> expr;
        bool freeLval;
        TypedValue* compile(Compiler*);
        void print(void);
        VarAssignNode(LOC_TY& loc, Node* v, Node* exp, bool b) : Node(loc), ref_expr(v), expr(exp), freeLval(b){}
        ~VarAssignNode(){ if(freeLval) delete ref_expr; }
    };

    struct ExtNode : public Node{
        unique_ptr<TypeNode> typeExpr;
        unique_ptr<TypeNode> traits;
        unique_ptr<Node> methods;

        TypedValue* compile(Compiler*);
        void print(void);
        ExtNode(LOC_TY& loc, TypeNode *ty, Node *m, TypeNode *tr) : Node(loc), typeExpr(ty), traits(tr), methods(m){}
        ~ExtNode(){}
    };

    struct ImportNode : public Node{
        unique_ptr<Node> expr;
        TypedValue* compile(Compiler*);
        void print();
        ImportNode(LOC_TY& loc, Node* e) : Node(loc), expr(e){}
        ~ImportNode(){}
    };

    struct JumpNode : public Node{
        unique_ptr<Node> expr;
        int jumpType;
        TypedValue* compile(Compiler*);
        void print();
        JumpNode(LOC_TY& loc, int jt, Node* e) : Node(loc), expr(e), jumpType(jt){}
        ~JumpNode(){}
    };

    struct WhileNode : public ParentNode{
        unique_ptr<Node> condition;
        TypedValue* compile(Compiler*);
        void print(void);
        WhileNode(LOC_TY& loc, Node *cond, Node *body) : ParentNode(loc, body), condition(cond){}
        ~WhileNode(){}
    };

    struct ForNode : public ParentNode{
        string var;
        unique_ptr<Node> range;
        TypedValue* compile(Compiler*);
        void print(void);
        ForNode(LOC_TY& loc, string v, Node *r, Node *body) : ParentNode(loc, body), var(v), range(r){}
        ~ForNode(){}
    };

    struct MatchBranchNode : public Node{
        unique_ptr<Node> pattern, branch;
        TypedValue* compile(Compiler*);
        void print(void);
        MatchBranchNode(LOC_TY& loc, Node *p, Node *b) : Node(loc), pattern(p), branch(b){}
        ~MatchBranchNode(){}
    };

    struct MatchNode : public Node{
        unique_ptr<Node> expr;
        vector<unique_ptr<MatchBranchNode>> branches;

        TypedValue* compile(Compiler*);
        void print(void);
        MatchNode(LOC_TY& loc, Node *e, vector<unique_ptr<MatchBranchNode>> &b) : Node(loc), expr(e), branches(move(b)){}
        ~MatchNode(){}
    };

    struct IfNode : public Node{
        unique_ptr<Node> condition, thenN, elseN;
        TypedValue* compile(Compiler*);
        void print(void);
        IfNode(LOC_TY& loc, Node* c, Node* then, Node* els) : Node(loc), condition(c), thenN(then), elseN(els){}
        ~IfNode(){}
    };

    struct FuncDeclNode : public Node{
        string name, basename;
        shared_ptr<Node> child;
        unique_ptr<Node> modifiers, type;
        unique_ptr<NamedValNode> params;
        bool varargs;

        TypedValue* compile(Compiler*);
        void print(void);
        FuncDeclNode(LOC_TY& loc, string s, string bn, Node *mods, Node *t, Node *p, Node* b, bool va=false) : Node(loc), name(s), basename(bn), child(b), modifiers(mods), type(t), params((NamedValNode*)p), varargs(va){}
        FuncDeclNode(FuncDeclNode* fdn);
        ~FuncDeclNode(){ if(next.get()) next.release(); }
    };

    struct DataDeclNode : public ParentNode{
        string name;
        size_t fields;
        vector<unique_ptr<TypeNode>> generics;

        TypedValue* compile(Compiler*);
        void print(void);
        DataDeclNode(LOC_TY& loc, string s, Node* b, size_t f) : ParentNode(loc, b), name(s), fields(f){}
        DataDeclNode(LOC_TY& loc, string s, Node* b, size_t f, vector<unique_ptr<TypeNode>> &g) : ParentNode(loc, b), name(s), fields(f), generics(move(g)){}
        ~DataDeclNode(){}
    };

    struct TraitNode : public ParentNode{
        string name;

        TypedValue* compile(Compiler*);
        void print(void);
        TraitNode(LOC_TY& loc, string s, Node* b) : ParentNode(loc, b), name(s){}
        ~TraitNode(){}
    };


    namespace parser{
        RootNode* getRootNode();
        void printBlock(Node *block);
        void parseErr(ParseErr e, string s, bool showTok);
    }

    void printErrLine(const char* fileName, unsigned int row, unsigned int col);

}

#endif
