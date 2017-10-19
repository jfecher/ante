#ifndef PTREE_H
#define PTREE_H

#include "parser.h"

#ifndef LOC_TY
#define LOC_TY yy::location
#endif

//defined in lexer.cpp
extern char* lextxt;

namespace ante {
    namespace parser {

        Node* setRoot(Node* root);
        Node* getRoot();
        Node* setNext(Node* cur, Node* nxt);
        Node* setElse(Node *ifn, Node *elseN);
        Node* addMatch(Node *matchExpr, Node *newMatch);
        Node* applyMods(Node *mods, Node *decls);

        void createRoot(LOC_TY& loc);

        Node* append_main(Node *n);
        Node* append_fn(Node *n);
        Node* append_type(Node *n);
        Node* append_extension(Node *n);
        Node* append_trait(Node *n);
        Node* append_import(Node *n);

        Node* mkIntLitNode(LOC_TY loc, char* s);
        Node* mkFltLitNode(LOC_TY loc, char* s);
        Node* mkStrLitNode(LOC_TY loc, char* s);
        Node* mkCharLitNode(LOC_TY loc, char* s);
        Node* mkBoolLitNode(LOC_TY loc, char b);
        Node* mkArrayNode(LOC_TY loc, Node *expr);
        Node* mkTupleNode(LOC_TY loc, Node *expr);
        Node* mkModNode(LOC_TY loc, ante::TokenType mod);

        //A compiler directive is represented as a ModNode
        //internally, hence the omission of Node from the name below
        Node* mkCompilerDirective(LOC_TY loc, Node *mod);

        Node* mkGlobalNode(LOC_TY loc, Node* s);
        Node* mkTypeNode(LOC_TY loc, TypeTag type, char* typeName, Node *extTy = nullptr);
        Node* mkTypeCastNode(LOC_TY loc, Node *l, Node *r);
        Node* mkUnOpNode(LOC_TY loc, int op, Node *r);
        Node* mkBinOpNode(LOC_TY loc, int op, Node* l, Node* r);
        Node* mkSeqNode(LOC_TY loc, Node *l, Node *r);
        Node* mkBlockNode(LOC_TY loc, Node* b);
        Node* mkNamedValNode(LOC_TY loc, Node* nodes, Node* tExpr, Node* prev);
        Node* mkVarNode(LOC_TY loc, char* s);
        Node* mkRetNode(LOC_TY loc, Node* expr);
        Node* mkImportNode(LOC_TY loc, Node* expr);
        Node* mkLetBindingNode(LOC_TY loc, char* s, Node* mods, Node* tExpr, Node* expr);
        Node* mkVarDeclNode(LOC_TY loc, char* s, Node* mods, Node* tExpr, Node* expr);
        Node* mkVarAssignNode(LOC_TY loc, Node* var, Node* expr, bool shouldFreeLval = true);
        Node* mkExtNode(LOC_TY loc, Node* typeExpr, Node* methods, Node* traits=0);
        Node* mkMatchNode(LOC_TY loc, Node* expr, Node* branch);
        Node* mkMatchBranchNode(LOC_TY loc, Node* pattern, Node* branch);
        Node* mkJumpNode(LOC_TY loc, int jumpType, Node* expr);

        Node* mkIfNode(LOC_TY loc, Node* con, Node* body, Node* els);
        Node* mkWhileNode(LOC_TY loc, Node* con, Node* body);
        Node* mkForNode(LOC_TY loc, Node* var, Node* range, Node* body);
        Node* mkFuncDeclNode(LOC_TY loc, Node* s, Node* mods, Node* tExpr, Node* p, Node* body);
        Node* mkDataDeclNode(LOC_TY loc, char* s, Node *p, Node* b);
        Node* mkTraitNode(LOC_TY loc, char* s, Node* fns);

    }
}

#endif
