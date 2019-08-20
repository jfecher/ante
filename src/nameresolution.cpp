#include "nameresolution.h"
#include "compiler.h"
#include "target.h"
#include "types.h"
#include "uniontag.h"
#include "nodecl.h"
#include "scopeguard.h"
#include "types.h"
#include "moduletree.h"
#include "typeinference.h"
#include "trait.h"
#include "util.h"

using namespace std;

//yy::locations stored in all Nodes contain a string* to
//a filename which must not be freed until all nodes are
//deleted, including the FuncDeclNodes within ante::Modules
//that all have a static lifetime
list<string> fileNames;


namespace ante {
    using namespace parser;

    TypeArgs convertToTypeArgs(vector<unique_ptr<TypeNode>> const& types, Module *module){
        TypeArgs ret;
        ret.reserve(types.size());
        for(auto &t : types){
            ret.push_back(static_cast<AnTypeVarType*>(toAnType(t.get(), module)));
        }
        return ret;
    }

    TypeArgs convertToNewTypeArgs(vector<unique_ptr<TypeNode>> const& types, Module *module,
            unordered_map<string, AnTypeVarType*> &mapping){

        TypeArgs ret;
        ret.reserve(types.size());
        for(auto &t : types){
            ret.push_back(copyWithNewTypeVars(static_cast<AnTypeVarType*>(toAnType(t.get(), module)), mapping));
        }
        return ret;
    }

    /** Check if a name was declared previously in the given table.
     * Throw an appropriate error if it was. */
    template<typename T>
    void checkForPreviousDecl(NameResolutionVisitor *v, string const& name,
            T const& tbl, LOC_TY &loc, string kind = "", LOC_TY *importLoc = nullptr){

        auto prevDecl = tbl.find(name);
        if(prevDecl != tbl.end()){
            showError(kind + ' ' + name + " was already declared", loc);
            error(name + " was previously declared here", (*prevDecl).getValue()->getLoc(), ErrorType::Note);
            if(importLoc)
                error("Second" + name +  " was imported here", *importLoc, ErrorType::Note);
            throw CtError();
        }
    }


    void NameResolutionVisitor::declare(string const& name, VarNode *decl){
        if(name != "_"){
            checkForPreviousDecl(this, name, varTable.top().back(), decl->loc, "Variable");
            auto var = new Variable(name, decl);
            decl->decl = var;
            varTable.top().back().try_emplace(name, var);
        }else{
            auto var = new Variable(name, decl);
            decl->decl = var;
        }
    }


    void NameResolutionVisitor::declare(string const& name, NamedValNode *decl){
        if(name != "_" && name != ""){
            checkForPreviousDecl(this, name, varTable.top().back(), decl->loc, "Parameter");
            auto var = new Variable(name, decl);
            decl->decl = var;
            varTable.top().back().try_emplace(name, var);
        }else{
            auto var = new Variable(name, decl);
            decl->decl = var;
        }
    }


    void NameResolutionVisitor::declare(TraitDecl *decl, LOC_TY &loc){
        auto prevDecl = compUnit->traitDecls.find(decl->name);
        if(prevDecl != compUnit->traitDecls.end()){
            error("Trait " + decl->name + " has already been declared", loc);
        }
        compUnit->traitDecls.try_emplace(decl->name, decl);
    }


    void NameResolutionVisitor::define(string const& name, AnDataType *dt, LOC_TY &loc){
        TypeDecl *existingTy = lookupType(name);
        if(existingTy){
            auto pt = try_cast<AnProductType>(existingTy->type);
            if(pt->isTypeFamily()) return;

            showError(name + " was already declared", loc);
            error(name + " was previously declared here", existingTy->loc, ErrorType::Note);
        }

        TypeDecl decl{static_cast<AnType*>(dt), loc};
        compUnit->userTypes.try_emplace(name, decl);
    }

    TypeDecl* NameResolutionVisitor::lookupType(string const& name) const {
        return compUnit->lookupTypeDecl(name);
    }

    Variable* NameResolutionVisitor::lookupVar(std::string const& name) const {
        if(!varTable.empty()){
            auto &context = varTable.top();
            for(auto it = context.rbegin(); it != context.rend(); it++){
                auto var = it->find(name);
                if(var != it->end())
                    return var->getValue();
            }
        }
        //local var not found, search for a global
        auto it = globals.find(name);
        if(it != globals.end()){
            Variable *v = it->getValue().get();
            if(v->tval.type->hasModifier(Tok_Global))
                return v;
        }
        return nullptr;
    }


    size_t NameResolutionVisitor::getScope() const {
        return varTable.size();
    }


    void NameResolutionVisitor::newScope(){
        varTable.top().emplace_back();
    }


    void NameResolutionVisitor::exitScope(){
        varTable.top().pop_back();
    }


    void NameResolutionVisitor::enterFunction(){
        varTable.emplace();
        newScope();
    }


    void NameResolutionVisitor::exitFunction(){
        varTable.pop();
    }


    FuncDecl* NameResolutionVisitor::getFunction(string const& name) const{
        auto it = compUnit->fnDecls.find(name);
        if(it != compUnit->fnDecls.end())
            return it->getValue();

        for(const Module *m : compUnit->imports){
            auto it = m->fnDecls.find(name);
            if(it != m->fnDecls.end())
                return it->getValue();
        }
        return nullptr;
    }

    /** Declare function but do not define it */
    void NameResolutionVisitor::declare(FuncDeclNode *n){
        checkForPreviousDecl(this, n->name, this->compUnit->fnDecls, n->loc, "Function");

        auto *fd = new FuncDecl(n, n->name, this->compUnit);
        compUnit->fnDecls[n->name] = fd;
        n->decl = fd;
    }

    inline bool fileExists(const string &fName){
        if(FILE *f = fopen(fName.c_str(), "r")){
            fclose(f);
            return true;
        }
        return false;
    }

    /** add ".an" if string does not end with it already */
    std::string addAnSuffix(std::string const& s){
        if(s.empty() || (s.length() >= 3 && s.substr(s.length()-3) == ".an")){
            return s;
        }else{
            return s + ".an";
        }
    }

    /**
    * Returns the first path to a given filename
    * matched within the relative root directories.
    * If no file is found then the empty string is returned.
    */
    string findFile(string const& fName){
        for(auto &root : {AN_EXEC_STR, AN_LIB_DIR}){
            string f = root + addAnSuffix(fName);
            if(fileExists(f)){
                return f;
            }
        }
        return "";
    }

    /** Return true if the given file has already been imported into the current module. */
    bool alreadyImported(NameResolutionVisitor &v, std::string const& name){
        return std::any_of(v.compUnit->imports.begin(), v.compUnit->imports.end(), [&](Module *mod){
            return mod->name == name;
        });
    }

    template<typename T>
    bool hasFunction(vector<T> const& fns, string const& name){
        return ante::any(fns, [&](T const& declFn){
            return declFn->name == name;
        });
    }

    template<typename T>
    typename vector<T>::const_iterator getFunction(vector<T> const& fns, string const& name){
        return ante::find_if(fns, [&](T const& declFn){
            return declFn->name == name;
        });
    }

    void checkParamCountMatches(FuncDeclNode *decl, FuncDeclNode *def){
        size_t expectedParams = ante::count(*decl->params);
        size_t numParams = ante::count(*def->params);
        if(expectedParams != numParams){
            string part1 = to_string(expectedParams) + (expectedParams == 1 ? " parameter" : " parameters");
            string part2 = to_string(numParams) + (numParams == 1 ? " was" : " were");
            showError(def->name + " was declared to take " + part1 + " but " + part2 + " given here", def->loc);
        }
    }

    void checkParamCountMatches(TypeFamily const& decl, DataDeclNode *def){
        size_t expectedParams = decl.typeArgs.size();
        size_t numParams = def->generics.size();
        if(expectedParams != numParams){
            string part1 = to_string(expectedParams) + " type parameter" + (expectedParams == 1 ? "" : "s");
            string part2 = to_string(numParams) + (numParams == 1 ? " was" : " were");
            showError(def->name + " was declared to take " + part1 + " but " + part2 + " given here", def->loc);
        }
    }

    bool checkFnInTraitDecl(vector<shared_ptr<FuncDecl>> const& traitDeclFns,
            vector<FuncDecl*> const& traitImplFns, FuncDeclNode *fdn, TraitImpl *trait){

        auto decl = getFunction(traitDeclFns, fdn->name);
        if(decl != traitDeclFns.cend()){
            checkParamCountMatches((*decl)->getFDN(), fdn);
            return true;
        }

        auto original = getFunction(traitImplFns, fdn->name);
        if(original != traitImplFns.cend()){
            showError("Duplicate function " + fdn->name + " in trait impl", fdn->loc);
            showError(fdn->name + " previously defined here", (*original)->getFDN()->loc, ErrorType::Note);
        }else{
            showError("No function named " + fdn->name + " in trait " + trait->name, fdn->loc);
        }
        return false;
    }

    void checkForUnimplementedFunctions(vector<shared_ptr<FuncDecl>> const& traitDeclFns, TraitImpl *trait){
        if(!traitDeclFns.empty()){
            for(auto &fn : traitDeclFns){
                showError("impl " + traitToColoredStr(trait) + " missing implementation of " + fn->getName(), trait->impl->loc);
                showError(fn->getName() + " declared here", fn->getFDN()->loc, ErrorType::Note);
            }
            throw CtError();
        }
    }

    vector<TypeFamily>::const_iterator getType(vector<TypeFamily> const& decls, string const& name){
        return ante::find_if(decls, [&](TypeFamily const& decl){
            return decl.name == name;
        });
    }

    bool checkTyInTraitDecl(vector<TypeFamily> const& traitDeclTys,
            vector<TypeFamily> const& traitImplTys, DataDeclNode *ddn, TraitImpl *trait){

        auto decl = getType(traitDeclTys, ddn->name);
        if(decl != traitDeclTys.end()){
            checkParamCountMatches(*decl, ddn);
            return true;
        }

        auto duplicate = getType(traitImplTys, ddn->name);
        if(duplicate != traitImplTys.end()){
            showError("Duplicate type " + ddn->name + " in trait impl", ddn->loc);
            //showError(fdn->name + " previously defined here", (*duplicate)->getFDN()->loc, ErrorType::Note);
        }else{
            showError("No type named " + ddn->name + " in trait " + trait->name, ddn->loc);
        }
        return false;
    }

    void checkForUnimplementedTypes(vector<TypeFamily> const& traitDeclTys, TraitImpl *trait){
        if(!traitDeclTys.empty()){
            for(auto &ty : traitDeclTys){
                showError("impl " + traitToColoredStr(trait) + " missing implementation of type " + ty.name, trait->impl->loc);
            }
            throw CtError();
        }
    }

    TraitImpl* toTrait(TypeNode *tn, Module *m){
        auto decl = m->lookupTraitDecl(tn->typeName);
        if(!decl) return nullptr;

        auto typeArgs = ante::applyToAll(tn->params, [m](unique_ptr<TypeNode> const& tn) -> AnType*{
            return toAnType(tn.get(), m);
        });
        return new TraitImpl(decl, typeArgs);
    }

    void handleTraitImpl(NameResolutionVisitor &v, ExtNode *n){
        TraitImpl *trait = toTrait(n->trait.get(), v.compUnit);
        if(!trait)
            error(lazy_str(n->trait->typeName, AN_TYPE_COLOR) + " is not a trait", n->trait->loc);

        if(trait->implemented()){
            showError(traitToColoredStr(trait) + " has already been implemented", n->loc);
            error("Previously implemented here", trait->impl->loc, ErrorType::Note);
        }

        auto traitDeclFns = trait->decl->funcs;
        auto traitImplFns = vecOf<FuncDecl*>(traitDeclFns.size());

        auto traitDeclTys = trait->decl->typeFamilies;
        auto traitImplTys = vecOf<TypeFamily>(traitDeclTys.size());

        for(Node &m : *n->methods){
            if(FuncDeclNode *fdn = dynamic_cast<FuncDeclNode*>(&m)){
                auto *fd = new FuncDecl(fdn, fdn->name, v.compUnit);
                fdn->decl = fd;
                if(checkFnInTraitDecl(traitDeclFns, traitImplFns, fdn, trait)){
                    ante::remove_if(traitDeclFns, [&](shared_ptr<FuncDecl> const& declFn){
                        return declFn->name == fd->name;
                    });
                    traitImplFns.push_back(fd);
                }
            }else if(DataDeclNode *ddn = dynamic_cast<DataDeclNode*>(&m)){
                if(checkTyInTraitDecl(traitDeclTys, traitImplTys, ddn, trait)){
                    ante::remove_if(traitDeclTys, [&](TypeFamily &decl){
                        return decl.name == ddn->name;
                    });
                    traitImplTys.emplace_back(ddn->name, convertToTypeArgs(ddn->generics, v.compUnit));
                }
            }
        }

        trait->impl = n;
        n->traitType = trait;
        checkForUnimplementedFunctions(traitDeclFns, trait);
        checkForUnimplementedTypes(traitDeclTys, trait);
    }

    void NameResolutionVisitor::declare(ExtNode *n){
        if(n->typeExpr){ // module Mod
            string name = typeNodeToStr(n->typeExpr.get());
            Module &submodule = compUnit->addChild(name);

            NameResolutionVisitor submoduleVisitor{name};
            submoduleVisitor.compUnit = &submodule;

            if(!alreadyImported(submoduleVisitor, "Prelude"))
                TRY_TO(submoduleVisitor.importFile(AN_PRELUDE_FILE, n->loc));

            for(Node &m : *n->methods)
                TRY_TO(submoduleVisitor.declare((FuncDeclNode*)&m));
        }else{ // impl Trait
            handleTraitImpl(*this, n);
        }
    }


    void NameResolutionVisitor::visit(RootNode *n){
        if(compUnit->ast){
            ASSERT_UNREACHABLE()
        }
        compUnit->ast.reset(n);

        if(compUnit->name != "Prelude"){
            TRY_TO(importFile(AN_PRELUDE_FILE, n->loc));
        }
        for(auto &m : n->imports)
            TRY_TO(m->accept(*this));
        for(auto &m : n->types)
            TRY_TO(m->accept(*this));
        for(auto &m : n->traits)
            TRY_TO(m->accept(*this));

        // unwrap any surrounding modifiers then declare
        for(auto &m : n->extensions){
            auto mn = m.get();
            while(dynamic_cast<ModNode*>(mn))
                mn = static_cast<ModNode*>(mn)->expr.get();

            TRY_TO(declare(static_cast<ExtNode*>(mn)));
        }
        for(auto &m : n->funcs){
            auto mn = m.get();
            while(dynamic_cast<ModNode*>(mn))
                mn = static_cast<ModNode*>(mn)->expr.get();

            TRY_TO(declare(static_cast<FuncDeclNode*>(mn)));
        }
        for(auto &m : n->extensions)
            TRY_TO(m->accept(*this));
        for(auto &m : n->funcs)
            TRY_TO(m->accept(*this));
        for(auto &m : n->main)
            TRY_TO(m->accept(*this));
    }

    void NameResolutionVisitor::visit(IntLitNode *n){}

    void NameResolutionVisitor::visit(FltLitNode *n){}

    void NameResolutionVisitor::visit(BoolLitNode *n){}

    void NameResolutionVisitor::visit(StrLitNode *n){}

    void NameResolutionVisitor::visit(CharLitNode *n){}

    void NameResolutionVisitor::visit(ArrayNode *n){
        for(auto &e : n->exprs)
            e->accept(*this);
    }

    void NameResolutionVisitor::visit(TupleNode *n){
        for(auto &e : n->exprs)
            e->accept(*this);
    }

    void NameResolutionVisitor::visit(ModNode *n){
        if(n->expr)
            n->expr->accept(*this);
    }

    void NameResolutionVisitor::visit(TypeNode *n){
        n->setType(toAnType(n, compUnit));
    }

    void NameResolutionVisitor::visit(TypeCastNode *n){
        n->rval->accept(*this);

        /*  Check for validity of cast
        if(!val){
            error("Invalid type cast " + anTypeToColoredStr(rtval.type) +
                    " -> " + anTypeToColoredStr(ty), n->loc);
        }*/
    }

    void NameResolutionVisitor::visit(UnOpNode *n){
        n->rval->accept(*this);
    }

    void NameResolutionVisitor::visit(SeqNode *n){
        for(auto &e : n->sequence){
            TRY_TO(e->accept(*this));
        }
    }

    /**
    * Converts a given filename (with its file
    * extension already removed) to a module name.
    *
    * - Replaces directory separators with '.'
    * - Capitalizes first letters of words
    * - Ignores non alphanumeric characters
    */
    string toModuleName(string const& s){
        string mod = "";
        bool capitalize = true;

        for(auto &c : s){
            if(capitalize && ((c >= 'a' && c <= 'z') or (c >= 'A' && c <= 'Z'))){
                if(c >= 'a' && c <= 'z'){
                    mod += c + 'A' - 'a';
                }else{
                    mod += c;
                }
                capitalize = false;
            }else{
#ifdef _WIN32
                if(c == '\\'){
#else
                if(c == '/'){
#endif
                    if(&c != s.c_str()){
                        capitalize = true;
                        mod += '.';
                    }
                }else if(c == '_'){
                    capitalize = true;
                }else if(IS_ALPHANUM(c)){
                    mod += c;
                }
            }
        }
        return mod;
    }


    llvm::Optional<string> getIdentifier(Node *n){
        BinOpNode *bop = dynamic_cast<BinOpNode*>(n);
        if(bop && bop->op == '.'){
            auto l = getIdentifier(bop->lval.get());
            auto r = getIdentifier(bop->rval.get());
            if(!l || !r) return llvm::Optional<string>();
            return *l + "." + *r;
        }else if(VarNode *vn = dynamic_cast<VarNode*>(n)){
            return vn->name;
        }else if(TypeNode *tn = dynamic_cast<TypeNode*>(n)){
            return typeNodeToStr(tn);
        }else{
            return llvm::Optional<string>();
        }
    }


    Declaration* NameResolutionVisitor::findCandidate(Node *n) const {
        auto name = getIdentifier(n);
        auto vn = dynamic_cast<VarNode*>(n);
        if(!name){
            return new NoDecl(n);
        }else if(vn && !vn->decl->isFuncDecl()){
            return vn->decl;
        }
        
        auto bop = dynamic_cast<BinOpNode*>(n);
        if(bop && bop->decl){
            return bop->decl;
        }else{
            return getFunction(*name);
        }
    }

    bool findFieldInTypeList(llvm::StringMap<TypeDecl> const& m, Node *lval, VarNode *rval) {
        for(auto &p : m){
            if(auto *pt = try_cast<AnProductType>(p.second.type)){
                for(size_t i = 0; i < pt->fieldNames.size(); i++){
                    auto &field = pt->fieldNames[i];
                    if(field == rval->name){
                        auto ty = static_cast<AnProductType*>(copyWithNewTypeVars(pt));
                        lval->setType(ty);
                        auto *fakeDecl = new Variable(field, rval);
                        rval->decl = fakeDecl;
                        rval->setType(ty->fields[i]);
                        return true;
                    }
                }
            }
        }
        return false;
    }

    void NameResolutionVisitor::searchForField(BinOpNode *op){
        VarNode *vn = dynamic_cast<VarNode*>(op->rval.get());
        if(!vn){
            error("RHS of . operator must be an identifier", op->rval->loc);
        }

        if(findFieldInTypeList(compUnit->userTypes, op->lval.get(), vn))
            return;

        for(Module *m : compUnit->imports){
            if(findFieldInTypeList(m->userTypes, op->lval.get(), vn))
                return;
        }

        error("No field named " + vn->name + " found for any type", vn->loc);
    }


    bool isImplicitImportExpr(BinOpNode *bop){
        return bop && bop->op == '.' && dynamic_cast<TypeNode*>(bop->lval.get());
    }


    Module *findModule(NameResolutionVisitor *v, string const& name){
        for(Module *m : v->compUnit->imports){
            auto it = m->findChild(name);
            if(it != m->childrenEnd())
                return &it->second;
        }

        Module &root = Module::getRoot();
        auto it = root.findChild(name);
        return it != root.childrenEnd() ? &it->second : nullptr;
    }


    std::pair<Module*, Node*> handleImplicitModuleImport(NameResolutionVisitor *v, BinOpNode *n){
        BinOpNode *cur = n;
        TypeNode *tn;
        Node *rhs = n->rval.get();

        Module *m = nullptr;

        while(isImplicitImportExpr(cur)){
            tn = static_cast<TypeNode*>(cur->lval.get());

            if(m == nullptr){
                m = findModule(v, tn->typeName);
            }else{
                auto it = m->findChild(tn->typeName);
                if(it == m->childrenEnd()){
                    error("Cannot find module " + lazy_str(tn->typeName, AN_TYPE_COLOR), tn->loc);
                }
                m = &it->second;
            }

            rhs = cur->rval.get();
            cur = dynamic_cast<BinOpNode*>(rhs);
        }

        if(!m){
            error("Cannot find module " + lazy_str(((TypeNode*)(n->lval.get()))->typeName, AN_TYPE_COLOR), n->lval->loc);
        }
        return {m, rhs};
    }


    void NameResolutionVisitor::visit(BinOpNode *n){
        if(isImplicitImportExpr(n)){
            auto modAndNode = handleImplicitModuleImport(this, n);
            Module *mod = modAndNode.first;

            if(VarNode *vn = dynamic_cast<VarNode*>(modAndNode.second)){
                auto fn = mod->fnDecls.find(vn->name);
                if(fn == mod->fnDecls.end()){
                    error("No function named '" + vn->name + "' has not been declared in "
                            + lazy_str(mod->name, AN_TYPE_COLOR), n->loc);
                }
                vn->decl = fn->second;
                n->decl = fn->second;
                return;
            }
        }

        n->lval->accept(*this);

        if(n->op == '.'){
            searchForField(n);
            return;
        }

        n->rval->accept(*this);

        if(n->op != '('){
            FuncDecl *candidate = getFunction(Lexer::getTokStr(n->op));
            if(candidate)
                n->decl = candidate;
            else //v TODO: memory leak here
                n->decl = new NoDecl(n);
        }else{
            n->decl = findCandidate(n->lval.get());
        }
    }

    void NameResolutionVisitor::visit(BlockNode *n){
        newScope();
        n->block->accept(*this);
        exitScope();
    }

    void NameResolutionVisitor::visit(RetNode *n){
        n->expr->accept(*this);
    }


    template<class StringIt>
    NameResolutionVisitor visitImport(string const& filename, StringIt path){
        //The lexer stores the fileName in the loc field of all Nodes. The fileName is copied
        //to let Node's outlive the context they were made in, ensuring they work with imports.
        fileNames.emplace_back(filename);
        setLexer(new Lexer(&fileNames.back()));
        yy::parser p{};
        int flag = p.parse();
        if(flag != PE_OK){ //parsing error, cannot procede
            //print out remaining errors
            int tok;
            yy::location loc;
            while((tok = yylexer->next(&loc)) != Tok_Newline && tok != 0);
            while(p.parse() != PE_OK && yylexer->peek() != 0);

            cerr << "Syntax error, aborting.\n";
            exit(flag);
        }
        string modName = "";
        for(string s : path) modName = s;

        //Add this module to the cache first to ensure it is not compiled twice
        NameResolutionVisitor newVisitor{modName};
        newVisitor.compUnit = &Module::getRoot().addPath(path);
        RootNode *root = parser::getRootNode();
        root->accept(newVisitor);

        if (errorCount()) return newVisitor;
        TypeInferenceVisitor::infer(root, newVisitor.compUnit);
        return newVisitor;
    }


    /**
     * Issue an error if any of the names in the given module
     * conflict with any in the modules already imported.
     */
    void checkForConflict(Module *import, Module *other, LOC_TY &loc){
        for(const auto& ty : import->userTypes){
            if(other->userTypes.find(ty.getKey()) != other->userTypes.end()){
                error(lazy_str(ty.getKey().str(), AN_TYPE_COLOR) +  " in module "
                        + lazy_str(import->name, AN_TYPE_COLOR) + " conflicts with "
                        + lazy_str(ty.getKey().str(), AN_TYPE_COLOR)
                        + " in module " + lazy_str(other->name, AN_TYPE_COLOR), loc);
            }
        }

        for(const auto& tr : import->traitDecls){
            if(other->traitDecls.find(tr.getKey()) != other->traitDecls.end()){
                error(lazy_str(tr.getKey().str(), AN_TYPE_COLOR) +  " in module "
                        + lazy_str(import->name, AN_TYPE_COLOR) + " conflicts with "
                        + lazy_str(tr.getKey().str(), AN_TYPE_COLOR)
                        + " in module " + lazy_str(other->name, AN_TYPE_COLOR), loc);
            }
        }

        for(const auto& fn : import->fnDecls){
            if(other->fnDecls.find(fn.getKey()) != other->fnDecls.end()){
                error(lazy_str(fn.getKey().str(), AN_TYPE_COLOR) +  " in module "
                        + lazy_str(import->name, AN_TYPE_COLOR) + " conflicts with "
                        + lazy_str(fn.getKey().str(), AN_TYPE_COLOR)
                        + " in module " + lazy_str(other->name, AN_TYPE_COLOR), loc);
            }
        }
    }


    void NameResolutionVisitor::importFile(string const& fName, LOC_TY &loc){
        //f = fName with full directory
        string fullPath = findFile(fName);
        if(fullPath.empty()){
            error("No file named '" + string(fName) + "' was found.", loc);
        }
        auto modPath = ModulePath(fName);
        Module &root = Module::getRoot();
        auto it = root.findPath(modPath);
        if(it != root.childrenEnd()){
            //module already compiled
            Module *import = &it->getValue();
            for(auto *mod : compUnit->imports){
                if(mod->name == import->name){
                    error("Module " + lazy_str(import->name, AN_TYPE_COLOR) + " has already been imported", loc, ErrorType::Warning);
                    return;
                }
                checkForConflict(import, mod, loc);
            }

            compUnit->imports.push_back(import);
        }else{
            //module not found
            NameResolutionVisitor newVisitor = visitImport(fullPath, modPath);
            compUnit->imports.push_back(newVisitor.compUnit);
        }
    }

    /**
    * Return a copy of the given string with the first character in lowercase.
    */
    std::string lowercaseFirstLetter(std::string const& s){
        if(s.empty()) return "";
        return char(tolower(s[0])) + s.substr(1);
    }

    /**
    * Convert an import expression to a filepath string.
    * Converts most tokens as given, but lowercases the first letter of types
    * as these modules are expected to meet the convention of capital module
    * name referring to a lowercase filename.  If this is not desired, string
    * literals can be used instead.
    */
    std::string moduleExprToStr(Node *expr){
        if(BinOpNode *bn = dynamic_cast<BinOpNode*>(expr)){
            if(bn->op != '.') return "";

            return moduleExprToStr(bn->lval.get()) + "/" + moduleExprToStr(bn->rval.get());
        }else if(TypeNode *tn = dynamic_cast<TypeNode*>(expr)){
            if(tn->typeTag != TT_Data || !tn->params.empty()) return "";

            return lowercaseFirstLetter(tn->typeName);
        }else if(VarNode *va = dynamic_cast<VarNode*>(expr)){
            return va->name;
        }else if(StrLitNode *sln = dynamic_cast<StrLitNode*>(expr)){
            return sln->val;
        }else{
            error("Syntax error in import expression", expr->loc);
            return "";
        }
    }

    /**
    * Converts an import expression to a filepath string.
    * See moduleExprToStr for details.
    */
    std::string importExprToStr(Node *expr){
        if(StrLitNode *sln = dynamic_cast<StrLitNode*>(expr)){
            return sln->val;
        }else{
            return addAnSuffix(moduleExprToStr(expr));
        }
    }

    void NameResolutionVisitor::visit(ImportNode *n){
        //TODO: handle name resolution for custom overloads of import
        std::string path = importExprToStr(n->expr.get());
        if(path.empty()){
            error("No viable overload for import for malformed expression", n->loc);
        }

        importFile(path.c_str(), n->loc);
    }


    void NameResolutionVisitor::visit(IfNode *n){
        n->condition->accept(*this);
        newScope();
        n->thenN->accept(*this);
        exitScope();
        if(n->elseN){
            newScope();
            n->elseN->accept(*this);
            exitScope();
        }
    }

    void NameResolutionVisitor::visit(NamedValNode *n){
        if(n->typeExpr)
            n->typeExpr->accept(*this);
        declare(n->name, n);
    }

    void NameResolutionVisitor::visit(VarNode *n){
        if(autoDeclare){
            declare(n->name, n);
            return;
        }

        auto maybeVar = lookupVar(n->name);
        if(maybeVar){
            n->decl = maybeVar;
        }else if(FuncDecl *fn = getFunction(n->name)){
            n->decl = fn;
        }else{
            error("Variable or function '" + n->name + "' has not been declared.", n->loc);
        }
    }


    void NameResolutionVisitor::visit(VarAssignNode *n){
        if(n->modifiers.empty()){
            //assignment
            n->ref_expr->accept(*this);
        }else{
            //declaration
            for(auto &mod : n->modifiers)
                mod->accept(*this);

            TMP_SET(autoDeclare, true);
            n->ref_expr->accept(*this);
            static_cast<VarNode*>(n->ref_expr)->decl->definition = n->expr.get();
        }
        n->expr->accept(*this);
    }

    void NameResolutionVisitor::visit(ExtNode *n){
        if(n->typeExpr){
            string name = typeNodeToStr(n->typeExpr.get());
            NameResolutionVisitor submodule{name};
            submodule.compUnit = &compUnit->findChild(name)->second;

            assert(submodule.compUnit && ("Could not find submodule " + name).c_str());

            for(Node &m : *n->methods)
                TRY_TO(m.accept(submodule));
        } else {
            if (!alreadyImported(*this, "Prelude"))
                TRY_TO(importFile(AN_PRELUDE_FILE, n->loc));
            for (Node &m : *n->methods)
                TRY_TO(m.accept(*this));
        }
    }

    void NameResolutionVisitor::visit(JumpNode *n){
        n->expr->accept(*this);
    }

    void NameResolutionVisitor::visit(WhileNode *n){
        n->condition->accept(*this);
        newScope();
        n->child->accept(*this);
        exitScope();
    }

    void NameResolutionVisitor::visit(ForNode *n){
        n->range->accept(*this);
        newScope();
        {
            TMP_SET(autoDeclare, true);
            n->pattern->accept(*this);
        }
        n->child->accept(*this);
        exitScope();
    }

    void NameResolutionVisitor::visit(MatchNode *n){
        n->expr->accept(*this);
        for(auto &b : n->branches){
            newScope();
            b->accept(*this);
            exitScope();
        }
    }

    void NameResolutionVisitor::visit(MatchBranchNode *n){
        {
            TMP_SET(autoDeclare, true);
            n->pattern->accept(*this);
        }
        n->branch->accept(*this);
    }

    void NameResolutionVisitor::visit(FuncDeclNode *n){
        if(!n->decl){
            declare(n);
        }

        enterFunction();
        for(Node &p : *n->params){
            p.accept(*this);
        }

        if(n->child)
            n->child->accept(*this);
        exitFunction();
    }

    /*
    *  Checks to see if a type is valid to be used.
    *  To be valid the type must:
    *      - Not be recursive (contain no references to
    *        itself that are not behind a pointer)
    *      - Contain no typevars that are not declared
    *        within the rootTy's params
    *      - Contain only data types that have been declared
    */
    void NameResolutionVisitor::validateType(const AnType *tn, const DataDeclNode *rootTy){
        if(!tn) return;

        if(tn->typeTag == TT_Data){
            auto *dataTy = try_cast<AnProductType>(tn);

            if(dataTy->name == rootTy->name){
                if(dataTy->name == rootTy->name){
                    error("Recursive types are disallowed, wrap the type in a pointer instead", rootTy->loc);
                }

                error("Type "+dataTy->name+" has not been declared", rootTy->loc);
            }

            for(auto *t : dataTy->fields)
                validateType(t, rootTy);

        }else if(tn->typeTag == TT_TaggedUnion){
            auto *dataTy = try_cast<AnSumType>(tn);

            if(dataTy->name == rootTy->name){
                if(dataTy->name == rootTy->name){
                    error("Recursive types are disallowed, wrap the type in a pointer instead", rootTy->loc);
                }

                error("Type "+dataTy->name+" has not been declared", rootTy->loc);
            }

            for(auto *t : dataTy->tags)
                validateType(t, rootTy);

        }else if(tn->typeTag == TT_Tuple){
            auto *agg = try_cast<AnAggregateType>(tn);
            for(auto *ext : agg->extTys){
                validateType(ext, rootTy);
            }
        }else if(tn->typeTag == TT_Array){
            auto *arr = try_cast<AnArrayType>(tn);
            validateType(arr->extTy, rootTy);
        }else if(tn->typeTag == TT_Ptr or tn->typeTag == TT_Function or tn->typeTag == TT_MetaFunction){
            return;

        }else if(tn->typeTag == TT_TypeVar){
            auto *tvt = try_cast<AnTypeVarType>(tn);

            for(auto &p : rootTy->generics){
                if(p->typeName == tvt->name) return;
            }

            error("Lookup for " + tvt->name + " not found", rootTy->loc);
        }
    }


    void NameResolutionVisitor::visitUnionDecl(parser::DataDeclNode *decl){
        auto generics = convertToTypeArgs(decl->generics, compUnit);
        AnSumType *data = AnSumType::create(decl->name, {}, generics);
        define(decl->name, data, decl->loc);

        for(Node& child : *decl->child){
            auto nvn = static_cast<NamedValNode*>(&child);
            TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
            AnType *tagTy = tyn->extTy ? toAnType(tyn->extTy.get(), compUnit) : AnType::getUnit();

            // fake var to make sure the field decl is not null
            auto var = new Variable(nvn->name, decl);
            nvn->decl = var;

            vector<AnType*> exts = { AnType::getU8() }; //All variants are comprised of at least their tag value
            if(tagTy->typeTag == TT_Tuple){
                auto &extTys = try_cast<AnAggregateType>(tagTy)->extTys;
                exts.insert(exts.end(), extTys.begin(), extTys.end());
            }else{
                exts.push_back(tagTy);
            }

            //Store the tag as a UnionTag and a AnDataType
            //AnDataType *tagdt = AnDataType::create(nvn->name, exts, false, generics);
            AnProductType *tagdt = AnProductType::create(nvn->name, exts, generics);

            tagdt->parentUnionType = data;
            tagdt->isGeneric = isGeneric(exts);
            data->tags.emplace_back(tagdt);

            validateType(tagTy, decl);
            define(nvn->name, tagdt, nvn->loc);
        }
    }


    void NameResolutionVisitor::visitTypeFamily(DataDeclNode *n){
        auto *family = AnProductType::createTypeFamily(n->name, convertToTypeArgs(n->generics, compUnit));
        define(n->name, family, n->loc);
    }


    void NameResolutionVisitor::visit(DataDeclNode *n){
        auto *nvn = (NamedValNode*)n->child.get();
        if(!nvn) return visitTypeFamily(n);

        if(((TypeNode*) nvn->typeExpr.get())->typeTag == TT_TaggedUnion){
            visitUnionDecl(n);
            return;
        }

        AnProductType *data = AnProductType::create(n->name, {}, convertToTypeArgs(n->generics, compUnit));

        define(n->name, data, n->loc);
        data->fields.reserve(n->fields);
        data->fieldNames.reserve(n->fields);
        data->isAlias = n->isAlias;

        while(nvn){
            TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
            auto ty = toAnType(tyn, compUnit);

            auto var = new Variable(nvn->name, n);
            nvn->decl = var;

            validateType(ty, n);

            data->fields.push_back(ty);
            data->fieldNames.push_back(nvn->name);

            nvn = (NamedValNode*)nvn->next.get();
        }
    }

    void mutateWithNewTypeVarNodes(TypeNode *ty, unordered_map<string, AnTypeVarType*> &map){
        auto mn = dynamic_cast<ModNode*>(ty);
        if(mn){
            mutateWithNewTypeVarNodes(static_cast<TypeNode*>(mn->expr.get()), map);
            return;
        }

        if(ty->extTy){
            for(Node &node : *ty->extTy){
                if(TypeNode *ext = dynamic_cast<TypeNode*>(&node)){
                    mutateWithNewTypeVarNodes((TypeNode*)ext, map);
                }
            }
        }else if(ty->typeTag == TT_TypeVar){
            auto it = map.find(ty->typeName);
            if(it != map.end()){
                ty->typeName = it->second->name;
            }else{
                auto newTypeVar = nextTypeVar();
                map[ty->typeName] = newTypeVar;
                ty->typeName = newTypeVar->name;
            }
        }
    }

    void mutateWithNewTypeVarNodes(FuncDeclNode *fdn, unordered_map<string, AnTypeVarType*> &map){
        if(fdn->returnType) mutateWithNewTypeVarNodes(fdn->returnType.get(), map);
        for(Node& node : *fdn->params){
            auto nvn = static_cast<NamedValNode*>(&node);
            if(nvn->typeExpr){
                mutateWithNewTypeVarNodes(static_cast<TypeNode*>(nvn->typeExpr.get()), map);
            }
        }
    }

    void NameResolutionVisitor::visit(TraitNode *n){
        unordered_map<string, AnTypeVarType*> map;
        auto typeArgs = convertToNewTypeArgs(n->generics, compUnit, map);
        auto decl = new TraitDecl(n->name, typeArgs);

        // trait type is created here but the internal trait
        // tr will still be mutated with additional methods after
        declare(decl, n->loc);

        enterFunction();
        for(Node &child : *n->child){
            if(FuncDeclNode *fdn = dynamic_cast<FuncDeclNode*>(&child)){
                mutateWithNewTypeVarNodes(fdn, map);
                child.accept(*this);
                auto *fd = static_cast<FuncDecl*>(fdn->decl);
                compUnit->fnDecls[fdn->name] = fd;
                decl->funcs.emplace_back(fd);
            }else if(DataDeclNode *type = dynamic_cast<DataDeclNode*>(&child)){
                child.accept(*this);
                decl->typeFamilies.emplace_back(type->name, convertToNewTypeArgs(type->generics, compUnit, map));
            }
        }
        exitFunction();
    }
}
