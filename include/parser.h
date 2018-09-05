#ifndef PARSER_H
#define PARSER_H

#include <vector>
#include <memory>
#include "lexer.h"
#include "tokens.h"
#include "location.hh"
#include "nodevisitor.h"

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

            virtual void accept(NodeVisitor& v) = 0;

            NodeIterator begin();
            NodeIterator end();

            Node(LOC_TY& l) : next(nullptr), prev(nullptr), loc(l){}
            virtual ~Node(){}
        };

        struct ModNode;

        /*
         * Base class for all Nodes that can possibly be modified
         * by a modifier or compiler directive.
         */
        struct ModifiableNode : public Node{
            std::vector<std::unique_ptr<ModNode>> modifiers;

            /*
             * The body should always be known when a
             * parent node is initialized, so it is required
             * in the constructor (unlike next and prev)
             */
            ModifiableNode(LOC_TY& loc) : Node(loc){}
            ~ModifiableNode(){}

            bool hasModifier(int mod) const;
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
            std::vector<std::unique_ptr<Node>> funcs, traits, extensions, types, imports, main;

            void accept(NodeVisitor& v){ v.visit(this); }

            /** Merge all contents of rn into this RootNode */
            void merge(const RootNode *rn);
            RootNode(LOC_TY& loc) : Node(loc){}
            ~RootNode(){}
        };


        struct IntLitNode : public Node{
            std::string val;
            TypeTag type;
            void accept(NodeVisitor& v){ v.visit(this); }
            IntLitNode(LOC_TY& loc, std::string s, TypeTag ty) : Node(loc), val(s), type(ty){}
            ~IntLitNode(){}
        };

        struct FltLitNode : public Node{
            std::string val;
            TypeTag type;
            void accept(NodeVisitor& v){ v.visit(this); }
            FltLitNode(LOC_TY& loc, std::string s, TypeTag ty) : Node(loc), val(s), type(ty){}
            ~FltLitNode(){}
        };

        struct BoolLitNode : public Node{
            bool val;
            void accept(NodeVisitor& v){ v.visit(this); }
            BoolLitNode(LOC_TY& loc, char b) : Node(loc), val((bool) b){}
            ~BoolLitNode(){}
        };

        struct CharLitNode : public Node{
            char val;
            void accept(NodeVisitor& v){ v.visit(this); }
            CharLitNode(LOC_TY& loc, char c) : Node(loc), val(c){}
            ~CharLitNode(){}
        };

        struct ArrayNode : public Node{
            std::vector<std::unique_ptr<Node>> exprs;
            void accept(NodeVisitor& v){ v.visit(this); }
            ArrayNode(LOC_TY& loc, std::vector<std::unique_ptr<Node>>& e) : Node(loc), exprs(move(e)){}
            ~ArrayNode(){}
        };

        struct TupleNode : public Node{
            std::vector<std::unique_ptr<Node>> exprs;
            void accept(NodeVisitor& v){ v.visit(this); }

            std::vector<TypedValue> unpack(Compiler*);
            TupleNode(LOC_TY& loc, std::vector<std::unique_ptr<Node>>& e) : Node(loc), exprs(move(e)){}
            ~TupleNode(){}
        };

        struct UnOpNode : public Node{
            int op;
            std::unique_ptr<Node> rval;
            void accept(NodeVisitor& v){ v.visit(this); }
            UnOpNode(LOC_TY& loc, int s, Node *rv) : Node(loc), op(s), rval(rv){}
            ~UnOpNode(){}
        };

        struct BinOpNode : public Node{
            int op;
            std::unique_ptr<Node> lval, rval;
            void accept(NodeVisitor& v){ v.visit(this); }
            BinOpNode(LOC_TY& loc, int s, Node *lv, Node *rv) : Node(loc), op(s), lval(lv), rval(rv){}
            ~BinOpNode(){}
        };

        struct SeqNode : public Node{
            std::vector<std::unique_ptr<Node>> sequence;
            void accept(NodeVisitor& v){ v.visit(this); }
            SeqNode(LOC_TY& loc) : Node(loc), sequence(){}
            ~SeqNode(){}
        };

        struct BlockNode : public Node{
            std::unique_ptr<Node> block;
            void accept(NodeVisitor& v){ v.visit(this); }
            BlockNode(LOC_TY& loc, Node *b) : Node(loc), block(b){}
            ~BlockNode(){}
        };

        /**
         *  A Node representing a modifier or compiler directive.
         *
         *  This ModNode is a compiler directive if and only if
         *  its mod tag equals ModNode::CD_ID.  If a ModNode
         *  is not a compiler directive, its expr field is always null.
         */
        struct ModNode : public Node{
            int mod;
            std::unique_ptr<Node> directive, expr;

            //this ModNode is a compiler directive iff its mod == preproc_id
            //otherwise, it is a normal modifier, and expr is null
            static const int CD_ID = 1;

            void accept(NodeVisitor& v){ v.visit(this); }

            bool isCompilerDirective() const {
                return mod == CD_ID;
            }

            /** Constructor for normal modifiers */
            ModNode(LOC_TY& loc, int m, Node *e) : Node(loc), mod(m), expr(e){}

            /** Constructor for compiler directives */
            ModNode(LOC_TY& loc, Node *d, Node *e) : Node(loc), mod(CD_ID), directive(d), expr(e){}
            ~ModNode(){}
        };

        struct TypeNode : public ModifiableNode{
            TypeTag type;
            std::string typeName; //used for usertypes
            std::unique_ptr<TypeNode> extTy; //Used for pointers and non-single anonymous types.
            std::vector<std::unique_ptr<TypeNode>> params; //type parameters for generic types

            void accept(NodeVisitor& v){ v.visit(this); }
            TypeNode(LOC_TY& loc, TypeTag ty, std::string tName, TypeNode* eTy)
                : ModifiableNode(loc), type(ty), typeName(tName), extTy(eTy), params(){}
            ~TypeNode(){}
        };

        struct TypeCastNode : public Node{
            std::unique_ptr<TypeNode> typeExpr;
            std::unique_ptr<Node> rval;
            void accept(NodeVisitor& v){ v.visit(this); }
            TypeCastNode(LOC_TY& loc, TypeNode *ty, Node *rv) : Node(loc), typeExpr(ty), rval(rv){}
            ~TypeCastNode(){}
        };

        struct RetNode : public Node{
            std::unique_ptr<Node> expr;
            void accept(NodeVisitor& v){ v.visit(this); }
            RetNode(LOC_TY& loc, Node* e) : Node(loc), expr(e){}
            ~RetNode(){}
        };

        struct NamedValNode : public Node{
            std::string name;
            std::unique_ptr<Node> typeExpr;
            void accept(NodeVisitor& v){ v.visit(this); }
            NamedValNode(LOC_TY& loc, std::string s, Node* t) : Node(loc), name(s), typeExpr(t){}
            ~NamedValNode(){ if(typeExpr.get() == (void*)1) typeExpr.release(); }
        };

        struct VarNode : public Node{
            std::string name;
            void accept(NodeVisitor& v){ v.visit(this); }
            VarNode(LOC_TY& loc, std::string s) : Node(loc), name(s){}
            ~VarNode(){}
        };

        struct GlobalNode : public Node{
            std::vector<std::unique_ptr<VarNode>> vars;
            void accept(NodeVisitor& v){ v.visit(this); }
            GlobalNode(LOC_TY& loc, std::vector<std::unique_ptr<VarNode>> &&vn) : Node(loc), vars(move(vn)){}
            ~GlobalNode(){}
        };

        struct StrLitNode : public Node{
            std::string val;
            void accept(NodeVisitor& v){ v.visit(this); }
            StrLitNode(LOC_TY& loc, std::string s) : Node(loc), val(s){}
            ~StrLitNode(){}
        };

        struct VarAssignNode : public ModifiableNode{
            Node* ref_expr;
            std::unique_ptr<Node> expr;
            bool freeLval;
            void accept(NodeVisitor& v){ v.visit(this); }
            VarAssignNode(LOC_TY& loc, Node* v, Node* exp, bool b)
                : ModifiableNode(loc), ref_expr(v), expr(exp), freeLval(b){}
            ~VarAssignNode(){ if(freeLval) delete ref_expr; }
        };

        struct ExtNode : public ModifiableNode{
            std::unique_ptr<TypeNode> typeExpr;
            std::unique_ptr<TypeNode> traits;
            std::unique_ptr<Node> methods;

            void accept(NodeVisitor& v){ v.visit(this); }
            ExtNode(LOC_TY& loc, TypeNode *ty, Node *m, TypeNode *tr)
                : ModifiableNode(loc), typeExpr(ty), traits(tr), methods(m){}
            ~ExtNode(){}
        };

        struct ImportNode : public Node{
            std::unique_ptr<Node> expr;
            void accept(NodeVisitor& v){ v.visit(this); }
            ImportNode(LOC_TY& loc, Node* e) : Node(loc), expr(e){}
            ~ImportNode(){}
        };

        struct JumpNode : public Node{
            std::unique_ptr<Node> expr;
            int jumpType;
            void accept(NodeVisitor& v){ v.visit(this); }
            JumpNode(LOC_TY& loc, int jt, Node* e) : Node(loc), expr(e), jumpType(jt){}
            ~JumpNode(){}
        };

        struct WhileNode : public Node{
            std::unique_ptr<Node> condition, child;
            void accept(NodeVisitor& v){ v.visit(this); }
            WhileNode(LOC_TY& loc, Node *cond, Node *body)
                : Node(loc), condition(cond), child(body){}
            ~WhileNode(){}
        };

        struct ForNode : public Node{
            std::string var;
            std::unique_ptr<Node> range, child;
            void accept(NodeVisitor& v){ v.visit(this); }
            ForNode(LOC_TY& loc, std::string v, Node *r, Node *body) :
                Node(loc), var(v), range(r), child(body){}
            ~ForNode(){}
        };

        struct MatchBranchNode : public Node{
            std::unique_ptr<Node> pattern, branch;
            void accept(NodeVisitor& v){ v.visit(this); }
            MatchBranchNode(LOC_TY& loc, Node *p, Node *b) : Node(loc), pattern(p), branch(b){}
            ~MatchBranchNode(){}
        };

        struct MatchNode : public Node{
            std::unique_ptr<Node> expr;
            std::vector<std::unique_ptr<MatchBranchNode>> branches;

            void accept(NodeVisitor& v){ v.visit(this); }
            MatchNode(LOC_TY& loc, Node *e, std::vector<std::unique_ptr<MatchBranchNode>> &b)
                : Node(loc), expr(e), branches(move(b)){}
            ~MatchNode(){}
        };

        struct IfNode : public Node{
            std::unique_ptr<Node> condition, thenN, elseN;
            void accept(NodeVisitor& v){ v.visit(this); }
            IfNode(LOC_TY& loc, Node* c, Node* then, Node* els)
                : Node(loc), condition(c), thenN(then), elseN(els){}
            ~IfNode(){}
        };

        struct FuncDeclNode : public ModifiableNode{
            std::string name;
            std::shared_ptr<Node> child;
            std::shared_ptr<TypeNode> type;
            std::shared_ptr<NamedValNode> params;
            bool varargs;

            void accept(NodeVisitor& v){ v.visit(this); }

            FuncDeclNode(LOC_TY& loc, std::string s, TypeNode *t, NamedValNode *p, Node* b, bool va=false) :
                ModifiableNode(loc), name(s), child(b), type(t), params(p), varargs(va){}
            ~FuncDeclNode(){ if(next.get()) next.release(); }
        };

        struct DataDeclNode : public ModifiableNode{
            std::unique_ptr<Node> child;
            std::string name;
            size_t fields;
            std::vector<std::unique_ptr<TypeNode>> generics;
            bool isAlias;

            void declare(Compiler*);
            void accept(NodeVisitor& v){ v.visit(this); }
            DataDeclNode(LOC_TY& loc, std::string s, Node* b, size_t f, bool a)
                : ModifiableNode(loc), child(b), name(s), fields(f), isAlias(a){}
            DataDeclNode(LOC_TY& loc, std::string s, Node* b, size_t f, std::vector<std::unique_ptr<TypeNode>> &g, bool a)
                : ModifiableNode(loc), child(b), name(s), fields(f), generics(move(g)), isAlias(a){}
            ~DataDeclNode(){}
        };

        struct TraitNode : public ModifiableNode{
            std::unique_ptr<Node> child;
            std::string name;

            void accept(NodeVisitor& v){ v.visit(this); }
            TraitNode(LOC_TY& loc, std::string s, Node* b)
                : ModifiableNode(loc), child(b), name(s){}
            ~TraitNode(){}
        };

        RootNode* getRootNode();
        void printBlock(Node *block, size_t indent_level);
        void parseErr(ParseErr e, std::string s, bool showTok);
    } // end of ante::parser

    void printErrLine(const char* fileName, unsigned int row, unsigned int col);

}

#endif
