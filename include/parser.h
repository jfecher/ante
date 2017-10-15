#ifndef PARSER_H
#define PARSER_H

#include <vector>
#include <memory>
#include "lexer.h"
#include "tokens.h"
#include "location.hh"

#ifndef LOC_TY
#  define LOC_TY yy::location
#endif

namespace ante {

    /* forward-decls from compiler.h */
    struct TypedValue;
    struct Compiler;

    namespace parser {
    
        /* Needed for compliancy with several versions of bison */
        yy::position mkPos(std::string *f, unsigned int line, unsigned int col);
        LOC_TY mkLoc(yy::position begin, yy::position end);


        enum ParseErr{
            PE_OK,
            PE_EXPECTED,
            PE_VAL_NOT_FOUND,
            PE_IDENT_NOT_FOUND,
            PE_INVALID_STMT,
        };

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
            std::unique_ptr<Node> next;
            Node *prev;
            LOC_TY loc;

            //print representation of node
            virtual void print(void) = 0;

            //compile node to a given module
            virtual TypedValue compile(Compiler*) = 0;

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
            std::unique_ptr<Node> child;

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
            //non-owning std::vectors (each decl is later moved into a ante::module)
            std::vector<FuncDeclNode*> funcs;
            std::vector<std::unique_ptr<TraitNode>> traits;
            std::vector<std::unique_ptr<ExtNode>> extensions;
            std::vector<std::unique_ptr<DataDeclNode>> types;
            std::vector<std::unique_ptr<ImportNode>> imports;

            std::vector<std::unique_ptr<Node>> main;

            TypedValue compile(Compiler*);
            void print();
            RootNode(LOC_TY& loc) : Node(loc){}
            ~RootNode(){}
        };


        struct IntLitNode : public Node{
            std::string val;
            TypeTag type;
            TypedValue compile(Compiler*);
            void print();
            IntLitNode(LOC_TY& loc, std::string s, TypeTag ty) : Node(loc), val(s), type(ty){}
            ~IntLitNode(){}
        };

        struct FltLitNode : public Node{
            std::string val;
            TypeTag type;
            TypedValue compile(Compiler*);
            void print(void);
            FltLitNode(LOC_TY& loc, std::string s, TypeTag ty) : Node(loc), val(s), type(ty){}
            ~FltLitNode(){}
        };

        struct BoolLitNode : public Node{
            bool val;
            TypedValue compile(Compiler*);
            void print(void);
            BoolLitNode(LOC_TY& loc, char b) : Node(loc), val((bool) b){}
            ~BoolLitNode(){}
        };

        struct CharLitNode : public Node{
            char val;
            TypedValue compile(Compiler*);
            void print(void);
            CharLitNode(LOC_TY& loc, char c) : Node(loc), val(c){}
            ~CharLitNode(){}
        };

        struct ArrayNode : public Node{
            std::vector<std::unique_ptr<Node>> exprs;
            TypedValue compile(Compiler*);
            void print(void);
            ArrayNode(LOC_TY& loc, std::vector<std::unique_ptr<Node>>& e) : Node(loc), exprs(move(e)){}
            ~ArrayNode(){}
        };

        struct TupleNode : public Node{
            std::vector<std::unique_ptr<Node>> exprs;
            TypedValue compile(Compiler*);

            std::vector<TypedValue> unpack(Compiler*);
            void print(void);
            TupleNode(LOC_TY& loc, std::vector<std::unique_ptr<Node>>& e) : Node(loc), exprs(move(e)){}
            ~TupleNode(){}
        };

        struct UnOpNode : public Node{
            int op;
            std::unique_ptr<Node> rval;
            TypedValue compile(Compiler*);
            void print(void);
            UnOpNode(LOC_TY& loc, int s, Node *rv) : Node(loc), op(s), rval(rv){}
            ~UnOpNode(){}
        };

        struct BinOpNode : public Node{
            int op;
            std::unique_ptr<Node> lval, rval;
            TypedValue compile(Compiler*);
            void print(void);
            BinOpNode(LOC_TY& loc, int s, Node *lv, Node *rv) : Node(loc), op(s), lval(lv), rval(rv){}
            ~BinOpNode(){}
        };

        struct SeqNode : public Node{
            std::vector<std::unique_ptr<Node>> sequence;
            TypedValue compile(Compiler*);
            void print(void);
            SeqNode(LOC_TY& loc) : Node(loc), sequence(){}
            ~SeqNode(){}
        };

        struct BlockNode : public Node{
            std::unique_ptr<Node> block;
            TypedValue compile(Compiler*);
            void print(void);
            BlockNode(LOC_TY& loc, Node *b) : Node(loc), block(b){}
            ~BlockNode(){}
        };

        struct ModNode : public Node{
            int mod;
            TypedValue compile(Compiler*);
            void print(void);
            ModNode(LOC_TY& loc, int m) : Node(loc), mod(m){}
            ~ModNode(){}
        };

        struct TypeNode : public Node{
            TypeTag type;
            std::string typeName; //used for usertypes
            std::unique_ptr<TypeNode> extTy; //Used for pointers and non-single anonymous types.
            std::vector<std::unique_ptr<TypeNode>> params; //type parameters for generic types
            std::vector<TokenType> modifiers;

            TypedValue compile(Compiler*);
            void print(void);
            TypeNode* addModifiers(ModNode *m);
            TypeNode* addModifier(int m);
            void copyModifiersFrom(const TypeNode *tn);
            bool hasModifier(int m) const;
            TypeNode(LOC_TY& loc, TypeTag ty, std::string tName, TypeNode* eTy) : Node(loc), type(ty), typeName(tName), extTy(eTy), params(), modifiers(){}
            ~TypeNode(){}
        };

        struct TypeCastNode : public Node{
            std::unique_ptr<TypeNode> typeExpr;
            std::unique_ptr<Node> rval;
            TypedValue compile(Compiler*);
            void print(void);
            TypeCastNode(LOC_TY& loc, TypeNode *ty, Node *rv) : Node(loc), typeExpr(ty), rval(rv){}
            ~TypeCastNode(){}
        };

        struct PreProcNode : public Node{
            std::shared_ptr<Node> expr;
            TypedValue compile(Compiler*);
            void print(void);
            PreProcNode(LOC_TY& loc, Node* e) : Node(loc), expr(e){}
            PreProcNode(LOC_TY& loc, std::shared_ptr<Node> e) : Node(loc), expr(e){}
            ~PreProcNode(){}
        };

        struct RetNode : public Node{
            std::unique_ptr<Node> expr;
            TypedValue compile(Compiler*);
            void print(void);
            RetNode(LOC_TY& loc, Node* e) : Node(loc), expr(e){}
            ~RetNode(){}
        };

        struct NamedValNode : public Node{
            std::string name;
            std::unique_ptr<Node> typeExpr;
            TypedValue compile(Compiler*);
            void print(void);
            NamedValNode(LOC_TY& loc, std::string s, Node* t) : Node(loc), name(s), typeExpr(t){}
            ~NamedValNode(){ if(typeExpr.get() == (void*)1) typeExpr.release(); }
        };

        struct VarNode : public Node{
            std::string name;
            TypedValue compile(Compiler*);
            void print(void);
            VarNode(LOC_TY& loc, std::string s) : Node(loc), name(s){}
            ~VarNode(){}
        };

        struct GlobalNode : public Node{
            std::vector<std::unique_ptr<VarNode>> vars;
            TypedValue compile(Compiler*);
            void print(void);
            GlobalNode(LOC_TY& loc, std::vector<std::unique_ptr<VarNode>> &&vn) : Node(loc), vars(move(vn)){}
            ~GlobalNode(){}
        };

        struct StrLitNode : public Node{
            std::string val;
            TypedValue compile(Compiler*);
            void print(void);
            StrLitNode(LOC_TY& loc, std::string s) : Node(loc), val(s){}
            ~StrLitNode(){}
        };

        struct LetBindingNode : public Node{
            std::string name;
            std::unique_ptr<Node> modifiers, typeExpr, expr;

            TypedValue compile(Compiler*);
            void print(void);
            LetBindingNode(LOC_TY& loc, std::string s, Node *mods, Node* t, Node* exp) : Node(loc), name(s), modifiers(mods), typeExpr(t), expr(exp){}
            ~LetBindingNode(){}
        };

        struct VarDeclNode : public Node{
            std::string name;
            std::unique_ptr<Node> modifiers, typeExpr, expr;

            TypedValue compile(Compiler*);
            void print(void);
            VarDeclNode(LOC_TY& loc, std::string s, Node *mods, Node* t, Node* exp) : Node(loc), name(s), modifiers(mods), typeExpr(t), expr(exp){}
            ~VarDeclNode(){}
        };

        struct VarAssignNode : public Node{
            Node* ref_expr;
            std::unique_ptr<Node> expr;
            bool freeLval;
            TypedValue compile(Compiler*);
            void print(void);
            VarAssignNode(LOC_TY& loc, Node* v, Node* exp, bool b) : Node(loc), ref_expr(v), expr(exp), freeLval(b){}
            ~VarAssignNode(){ if(freeLval) delete ref_expr; }
        };

        struct ExtNode : public Node{
            std::unique_ptr<TypeNode> typeExpr;
            std::unique_ptr<TypeNode> traits;
            std::unique_ptr<Node> methods;

            TypedValue compile(Compiler*);
            void print(void);
            ExtNode(LOC_TY& loc, TypeNode *ty, Node *m, TypeNode *tr) : Node(loc), typeExpr(ty), traits(tr), methods(m){}
            ~ExtNode(){}
        };

        struct ImportNode : public Node{
            std::unique_ptr<Node> expr;
            TypedValue compile(Compiler*);
            void print();
            ImportNode(LOC_TY& loc, Node* e) : Node(loc), expr(e){}
            ~ImportNode(){}
        };

        struct JumpNode : public Node{
            std::unique_ptr<Node> expr;
            int jumpType;
            TypedValue compile(Compiler*);
            void print();
            JumpNode(LOC_TY& loc, int jt, Node* e) : Node(loc), expr(e), jumpType(jt){}
            ~JumpNode(){}
        };

        struct WhileNode : public ParentNode{
            std::unique_ptr<Node> condition;
            TypedValue compile(Compiler*);
            void print(void);
            WhileNode(LOC_TY& loc, Node *cond, Node *body) : ParentNode(loc, body), condition(cond){}
            ~WhileNode(){}
        };

        struct ForNode : public ParentNode{
            std::string var;
            std::unique_ptr<Node> range;
            TypedValue compile(Compiler*);
            void print(void);
            ForNode(LOC_TY& loc, std::string v, Node *r, Node *body) : ParentNode(loc, body), var(v), range(r){}
            ~ForNode(){}
        };

        struct MatchBranchNode : public Node{
            std::unique_ptr<Node> pattern, branch;
            TypedValue compile(Compiler*);
            void print(void);
            MatchBranchNode(LOC_TY& loc, Node *p, Node *b) : Node(loc), pattern(p), branch(b){}
            ~MatchBranchNode(){}
        };

        struct MatchNode : public Node{
            std::unique_ptr<Node> expr;
            std::vector<std::unique_ptr<MatchBranchNode>> branches;

            TypedValue compile(Compiler*);
            void print(void);
            MatchNode(LOC_TY& loc, Node *e, std::vector<std::unique_ptr<MatchBranchNode>> &b) : Node(loc), expr(e), branches(move(b)){}
            ~MatchNode(){}
        };

        struct IfNode : public Node{
            std::unique_ptr<Node> condition, thenN, elseN;
            TypedValue compile(Compiler*);
            void print(void);
            IfNode(LOC_TY& loc, Node* c, Node* then, Node* els) : Node(loc), condition(c), thenN(then), elseN(els){}
            ~IfNode(){}
        };

        struct FuncDeclNode : public Node{
            std::string name;
            std::shared_ptr<Node> child;
            std::shared_ptr<Node> modifiers, type;
            std::shared_ptr<NamedValNode> params;
            bool varargs;

            TypedValue compile(Compiler*);
            void print(void);
            FuncDeclNode(LOC_TY& loc, std::string s, Node *mods, Node *t, Node *p, Node* b, bool va=false) : Node(loc), name(s), child(b), modifiers(mods), type(t), params((NamedValNode*)p), varargs(va){}
            ~FuncDeclNode(){ if(next.get()) next.release(); }
        };

        struct DataDeclNode : public ParentNode{
            std::string name;
            size_t fields;
            std::vector<std::unique_ptr<TypeNode>> generics;

            void declare(Compiler*);
            TypedValue compile(Compiler*);
            void print(void);
            DataDeclNode(LOC_TY& loc, std::string s, Node* b, size_t f) : ParentNode(loc, b), name(s), fields(f){}
            DataDeclNode(LOC_TY& loc, std::string s, Node* b, size_t f, std::vector<std::unique_ptr<TypeNode>> &g) : ParentNode(loc, b), name(s), fields(f), generics(move(g)){}
            ~DataDeclNode(){}
        };

        struct TraitNode : public ParentNode{
            std::string name;

            TypedValue compile(Compiler*);
            void print(void);
            TraitNode(LOC_TY& loc, std::string s, Node* b) : ParentNode(loc, b), name(s){}
            ~TraitNode(){}
        };


        RootNode* getRootNode();
        void printBlock(Node *block);
        void parseErr(ParseErr e, std::string s, bool showTok);
    } // end of ante::parser

    void printErrLine(const char* fileName, unsigned int row, unsigned int col);

}

#endif
