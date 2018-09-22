/*
 *      ptree.cpp
 *  Provide a C API to be used in parser.c generated from
 *  syntax.y which creates and links nodes to a parse tree.
 */
#include "compiler.h"
#include "yyparser.h"
#include <stack>

using namespace std;
using namespace ante::parser;

namespace ante {

    namespace parser {

        //stack of relative roots, eg. a FuncDeclNode's first statement would be set as the
        //relative root, where the last would be returned by the parser.  Relative roots are
        //returned through getRoot() which also pops the stack.
        stack<Node*> roots;

        //The single true-root of the compiled file.  One RootNode per file parsed.
        RootNode *root;

        RootNode* getRootNode(){
            return root;
        }

        bool ModifiableNode::hasModifier(int mod) const {
            for(auto& m : this->modifiers){
                if(m->mod == mod)
                    return true;
            }
            return false;
        }

        Node* setElse(Node *ifn, Node *elseN){
            if(auto *n = dynamic_cast<IfNode*>(ifn)){
                if(n->elseN)
                    setElse(n->elseN.get(), elseN);
                else
                    n->elseN.reset(elseN);
            }else{
                auto *seq = dynamic_cast<SeqNode*>(ifn);

                if(seq && (n = dynamic_cast<IfNode*>(seq->sequence.back().get()))){
                    while(auto *tmp = dynamic_cast<IfNode*>(n->elseN.get()))
                        n = tmp;

                    n->elseN.reset(elseN);
                    return ifn;
                }else{
                    ante::error("Missing matching if clause for else clause", ifn->loc);
                }
            }
            return ifn;
        }

        yy::position mkPos(string *f, unsigned int line, unsigned int col) {
            yy::position pos;
            pos.filename = f;
            pos.line = line;
            pos.column = col;
            return pos;
        }

        LOC_TY mkLoc(yy::position begin, yy::position end) {
            LOC_TY loc;
            loc.begin = begin;
            loc.end = end;
            return loc;
        }

        //initializes the root node
        void createRoot(LOC_TY& loc){
            root = new RootNode(loc);
        }

        void createRoot(){
            auto loc = mkLoc(mkPos(yylexer->fileName, 0, 0),
                             mkPos(yylexer->fileName, 0, 0));
            createRoot(loc);
        }

        Node* append_main(Node *n){
            root->main.emplace_back(n);
            return n;
        }

        Node* append_fn(Node *n){
            root->funcs.emplace_back(n);
            return n;
        }

        Node* append_type(Node *n){
            root->types.emplace_back(n);
            return n;
        }

        Node* append_extension(Node *n){
            root->extensions.emplace_back(n);
            return n;
        }

        Node* append_trait(Node *n){
            root->traits.emplace_back(n);
            return n;
        }

        Node* append_import(Node *n){
            root->imports.emplace_back(n);
            return n;
        }

        void copyModsToContainedNodes(ModNode *m, ModifiableNode *n){
            if(!m->isCompilerDirective()){
                if(ExtNode *en = dynamic_cast<ExtNode*>(n)){
                    for(auto *f : *en->methods){
                        if(ModifiableNode *fmn = dynamic_cast<ModifiableNode*>(f)){
                            ModNode *cpy = new ModNode(m->loc, m->mod, nullptr);
                            fmn->modifiers.emplace_back(cpy);
                        }
                    }
                }
            }
        }

        Node* append_modifier(Node *modifier, Node *modifiableNode){
            ModNode *m = (ModNode*)modifier;
            if(ModifiableNode *van = dynamic_cast<ModifiableNode*>(modifiableNode)){
                van->modifiers.emplace_back(m);
                copyModsToContainedNodes(m, van);

                return van;
            }else if(BinOpNode *assign = dynamic_cast<BinOpNode*>(modifiableNode)){
                if(assign->op == '='){
                    auto *vas = new VarAssignNode(assign->loc, assign->lval.release(), assign->rval.release(), false);
                    delete assign;
                    vas->modifiers.emplace_back(m);
                    return vas;
                }
            }
            m->expr.reset(modifiableNode);
            return m;
        }

        LOC_TY copyLoc(const LOC_TY &loc){
            return mkLoc(mkPos(loc.begin.filename, loc.begin.line, loc.begin.column),
                        mkPos(loc.end.filename,   loc.end.line,   loc.end.column));
        }

        NodeIterator Node::begin(){
            return {this};
        }

        NodeIterator Node::end(){
            return {nullptr};
        }

        NodeIterator NodeIterator::operator++(){
            cur = cur->next.get();
            return *this;
        }

        Node* NodeIterator::operator*(){
            return cur;
        }

        bool NodeIterator::operator==(NodeIterator r){
            return cur == r.cur;
        }

        bool NodeIterator::operator!=(NodeIterator r){
            return cur != r.cur;
        }


        Node* mkGlobalNode(LOC_TY loc, Node* s){
            vector<unique_ptr<VarNode>> vars;
            while(s){
                vars.emplace_back((VarNode*)s);
                s = s->next.release();
            }

            return new GlobalNode(loc, move(vars));
        }

        /*
        *  Saves the root of a new block and returns it.
        */
        Node* setRoot(Node* node){
            roots.push(node);
            return node;
        }

        /*
        *  Pops and returns the root of the current block
        */
        Node* getRoot(){
            Node *ret = roots.top();
            roots.pop();
            return ret;
        }

        Node* setNext(Node* cur, Node* nxt){
            cur->next.reset(nxt);
            return nxt;
        }

        Node* addMatch(Node *matchExpr, Node *newMatch){
            ((MatchNode*)matchExpr)->branches.emplace_back(
                (MatchBranchNode*)newMatch);
            return matchExpr;
        }

        TypeNode* mkAnonTypeNode(TypeTag);

        Node* mkIntLitNode(LOC_TY loc, char* s){
            string str = s;
            TypeTag type = TT_I32;

            //check for type suffix
            int len = str.length();
            if(len > 3 && (str[len -3] == 'u' || str[len - 3] == 'i')){
                char sign = str[len - 3];
                switch(str[len - 2]){
                    case '1':
                        type = sign == 'i'? TT_I16 : TT_U16;
                        str = str.substr(0, len-3);
                        break;
                    case '3':
                        type = sign == 'i'? TT_I32 : TT_U32;
                        str = str.substr(0, len-3);
                        break;
                    case '6':
                        type = sign == 'i'? TT_I64 : TT_U64;
                        str = str.substr(0, len-3);
                        break;
                    case 's':
                        type = sign == 'i'? TT_Isz : TT_Usz;
                        str = str.substr(0, len-3);
                        break;
                    default:
                        break;
                }
            }else if(len > 2 && (str[len-2] == 'u' || str[len-2] == 'i')){
                char sign = str[len - 2];
                type = sign == 'i'? TT_I8 : TT_U8;
                str = str.substr(0, len-2);
            }else if(len > 1){
                char sign = str[len - 1];
                if(sign == 'u' || sign == 'i'){
                    str = str.substr(0, len-1);
                    type = sign == 'i'? TT_Isz : TT_Usz;
                }
            }

            return new IntLitNode(loc, str, type);
        }

        Node* mkFltLitNode(LOC_TY loc, char* s){
            string str = s;
            int len = str.length();
            TypeTag type = TT_F64;

            if(len > 3 && str[len - 3] == 'f'){
                char fltSize = str[len - 2];
                if(fltSize == '1'){ //16 bit IEEE half
                    type = TT_F16;
                    str = str.substr(0, len-3);
                }else if(fltSize == '3'){ //32 bit IEEE single
                    type = TT_F32;
                    str = str.substr(0, len-3);
                }else if(fltSize == '6'){ //64 bit IEEE double
                    type = TT_F64;
                    str = str.substr(0, len-3);
                }
            }

            return new FltLitNode(loc, str, type);
        }

        Node* mkStrLitNode(LOC_TY loc, char* s){
            return new StrLitNode(loc, s);
        }

        Node* mkCharLitNode(LOC_TY loc, char* s){
            return new CharLitNode(loc, s[0]);
        }

        Node* mkBoolLitNode(LOC_TY loc, char b){
            return new BoolLitNode(loc, b);
        }

        Node* mkArrayNode(LOC_TY loc, Node *expr){
            vector<unique_ptr<Node>> exprs;
            while(expr){
                exprs.emplace_back(expr);
                auto *nxt = expr->next.get();
                expr->next.release();
                expr = nxt;
            }
            return new ArrayNode(loc, exprs);
        }

        Node* mkTupleNode(LOC_TY loc, Node *expr){
            vector<unique_ptr<Node>> exprs;
            while(expr){
                exprs.emplace_back(expr);
                auto *nxt = expr->next.get();
                expr->next.release();
                expr = nxt;
            }
            return new TupleNode(loc, exprs);
        }

        Node* mkModNode(LOC_TY loc, ante::TokenType mod){
            return new ModNode(loc, mod, nullptr);
        }

        Node* mkModExprNode(LOC_TY loc, ante::TokenType mod, Node *expr){
            return new ModNode(loc, mod, expr);
        }

        Node* mkCompilerDirective(LOC_TY loc, Node *directive){
            return new ModNode(loc, directive, nullptr);
        }

        Node* mkCompilerDirectiveExpr(LOC_TY loc, Node *directive, Node *expr){
            return new ModNode(loc, directive, expr);
        }

        Node* mkTypeNode(LOC_TY loc, TypeTag type, char* typeName, Node* extTy){
            if(type == TT_Array){
                //2nd type ext is size of the array when making Array types, ensure it is an intlit
                auto *size = dynamic_cast<IntLitNode*>(extTy->next.get());

                if(!size){
                    ante::error("Size of array must be an integer literal", extTy->next->loc);
                    exit(1);
                }
            }
            return new TypeNode(loc, type, typeName, static_cast<TypeNode*>(extTy));
        }

        Node* mkTypeCastNode(LOC_TY loc, Node *l, Node *r){
            return new TypeCastNode(loc, static_cast<TypeNode*>(l), r);
        }

        Node* mkUnOpNode(LOC_TY loc, int op, Node* r){
            return new UnOpNode(loc, op, r);
        }

        Node* mkBinOpNode(LOC_TY loc, int op, Node* l, Node* r){
            return new BinOpNode(loc, op, l, r);
        }

        Node* mkSeqNode(LOC_TY loc, Node *l, Node *r){
            if(SeqNode *seq = dynamic_cast<SeqNode*>(l)){
                seq->sequence.emplace_back(r);
                return seq;
            }else{
                SeqNode *s = new SeqNode(loc);
                s->sequence.emplace_back(l);
                s->sequence.emplace_back(r);
                return s;
            }
        }

        Node* mkBlockNode(LOC_TY loc, Node *b){
            return new BlockNode(loc, b);
        }

        Node* mkRetNode(LOC_TY loc, Node* expr){
            return new RetNode(loc, expr);
        }


        //returns true if type->extTy should be defined
        bool typeHasExtData(TypeTag t){
            return t == TT_Tuple or t == TT_Array or t == TT_Ptr or t == TT_Data or t == TT_Function
                or t == TT_TaggedUnion or t == TT_MetaFunction;

        }

        TypeNode* copy(const unique_ptr<TypeNode> &n);

        //helper function to deep-copy TypeNodes.  Used in mkNamedValNode
        TypeNode* copy(const TypeNode *n){
            if(!n or n == (void*)1) return 0;

            auto loc = copyLoc(n->loc);
            TypeNode *cpy = new TypeNode(loc, n->type, n->typeName, nullptr);

            //arrays can have an IntLit in their extTy so handle them specially
            if(n->type == TT_Array){
                cpy->extTy.reset(copy(n->extTy));

                auto *len = (IntLitNode*)n->extTy->next.get();
                if(len){
                    auto loc_cpy = copyLoc(len->loc);
                    auto *len_cpy = new IntLitNode(loc_cpy, len->val, len->type);
                    cpy->extTy->next.reset(len_cpy);
                }
            }else if(n->extTy.get()){
                TypeNode *nxt = n->extTy.get();

                TypeNode *ext = copy(nxt);
                cpy->extTy.reset(ext);

                while((nxt = static_cast<TypeNode*>(nxt->next.get()))){
                    ext->next.reset(copy(nxt));
                    ext = static_cast<TypeNode*>(ext->next.get());
                }
            }

            cpy->typeName = n->typeName;

            //if n has type params, copy them too
            if(!n->params.empty()){
                for(auto& tn : n->params){
                    cpy->params.emplace_back(copy(tn));
                }
            }
            return cpy;
        }

        TypeNode* copy(const unique_ptr<TypeNode> &n){
            return copy(n.get());
        }


        /*
        *  This may create several NamedVal nodes depending on the
        *  number of VarNodes contained within varNodes.
        *  This is used for the shortcut when declaring multiple
        *  variables of the same type, e.g. i32 a b c
        */
        Node* mkNamedValNode(LOC_TY loc, Node* varNodes, Node* tExpr, Node* prev){
            //Note: there will always be at least one varNode
            const TypeNode* ty = (TypeNode*)tExpr;
            VarNode* vn = (VarNode*)varNodes;
            Node *first = new NamedValNode(loc, vn->name, tExpr);
            Node *nxt = first;

            if(!prev) setRoot(first);
            else setNext(prev, first);

            while((vn = (VarNode*)vn->next.get())){
                TypeNode *tyNode = copy(ty);
                LOC_TY loccpy = copyLoc(vn->loc);

                nxt->next.reset(new NamedValNode(loccpy, vn->name, tyNode));
                nxt = nxt->next.get();
            }
            delete varNodes;
            return nxt;
        }

        Node* mkVarNode(LOC_TY loc, char* s){
            return new VarNode(loc, s);
        }

        Node* mkImportNode(LOC_TY loc, Node* expr){
            return new ImportNode(loc, expr);
        }

        Node* mkVarAssignNode(LOC_TY loc, Node* var, Node* expr, bool freeLval){
            return new VarAssignNode(loc, var, expr, freeLval);
        }

        Node* mkExtNode(LOC_TY loc, Node* ty, Node* methods, Node* traits){
            return new ExtNode(loc, (TypeNode*)ty, methods, (TypeNode*)traits);
        }

        Node* mkIfNode(LOC_TY loc, Node* con, Node* then, Node* els){
            return new IfNode(loc, con, then, els);
        }

        Node* mkJumpNode(LOC_TY loc, int jumpType, Node* expr){
            return new JumpNode(loc, jumpType, expr);
        }

        Node* mkWhileNode(LOC_TY loc, Node* con, Node* body){
            return new WhileNode(loc, con, body);
        }

        Node* mkForNode(LOC_TY loc, Node* var, Node* range, Node* body){
            return new ForNode(loc, (char*)var, range, body);
        }

        Node* mkFuncDeclNode(LOC_TY loc, Node* s, Node* tExpr, Node* p, Node* b){
            auto ret = new FuncDeclNode(loc, (char*)s, (TypeNode*)tExpr, (NamedValNode*)p, b);

            //s is copied from lextxt, and may or may not be equal
            if(s) free(s);
            return ret;
        }

        Node* mkDataDeclNode(LOC_TY loc, char* s, Node *p, Node* b, bool isAlias){
            vector<unique_ptr<TypeNode>> params;
            while(p){
                params.emplace_back((TypeNode*)p);
                p = p->next.release();
            }
            return new DataDeclNode(loc, s, b, getTupleSize(b), params, isAlias);
        }


        Node* mkMatchNode(LOC_TY loc, Node* expr, Node* branch){
            vector<unique_ptr<MatchBranchNode>> branches;
            branch->next.release();
            branches.emplace_back((MatchBranchNode*)branch);
            return new MatchNode(loc, expr, branches);
        }

        Node* mkMatchBranchNode(LOC_TY loc, Node* pattern, Node* branch){
            return new MatchBranchNode(loc, pattern, branch);
        }

        Node* mkTraitNode(LOC_TY loc, char* s, Node* fns){
            return new TraitNode(loc, s, fns);
        }
    } //end of namespace ante::parser
} //end of namespace ante
