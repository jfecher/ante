#include "nameresolution.h"
#include "compiler.h"
#include "target.h"
#include "types.h"
#include "uniontag.h"
#include "nodecl.h"
#include "scopeguard.h"
#include "types.h"

using namespace std;

//Global containing every module/file compiled
//to avoid recompilation
llvm::StringMap<unique_ptr<ante::Module>> allCompiledModules;

//each mergedCompUnits is static in lifetime
list<unique_ptr<ante::Module>> allMergedCompUnits;

//yy::locations stored in all Nodes contain a string* to
//a filename which must not be freed until all nodes are
//deleted, including the FuncDeclNodes within ante::Modules
//that all have a static lifetime
list<unique_ptr<string>> fileNames;


namespace ante {
    using namespace parser;

    template<typename F>
    void tryTo(F f){
        try{
            f();
        }catch(CompilationError *e){
            delete e;
        }
    }

    void NameResolutionVisitor::error(lazy_printer msg, LOC_TY loc, ErrorType t){
        ante::showError(msg, loc, t);
        if(t == ErrorType::Error){
            errFlag = true;
            throw new CompilationError(msg, loc);
        }
    }


    TypeArgs convertToTypeArgs(vector<unique_ptr<TypeNode>> const& types){
        TypeArgs ret;
        ret.reserve(types.size());
        for(auto &t : types){
            ret.push_back(static_cast<AnTypeVarType*>(toAnType(t.get())));
        }
        return ret;
    }

    /** Check if a name was declared previously in the given table.
     * Throw an appropriate error if it was. */
    template<typename T>
    void checkForPreviousDecl(NameResolutionVisitor *v, string const& name,
            T const& tbl, LOC_TY &loc, string kind = "", LOC_TY *importLoc = nullptr){

        auto prevDecl = tbl.find(name);

        //Lambda to just call the apropriate error function
        auto err = [&](string prepend, string msg, LOC_TY loc, ErrorType errTy){
            if(v)   v->error(prepend + name + msg, loc, errTy);
            else ante::error(prepend + name + msg, loc, errTy);
        };

        if(prevDecl != tbl.end()){
            try{
                err(kind + ' ', " was already declared", loc, ErrorType::Error);
            }catch(...){
                err("", " was previously declared here", (*prevDecl).getValue()->getLoc(), ErrorType::Note);
                if(importLoc)
                    err("Second", " was imported here", *importLoc, ErrorType::Note);
                throw;
            }
        }
    }


    void NameResolutionVisitor::declare(string const& name, VarNode *decl){
        checkForPreviousDecl(this, name, varTable.top().back(), decl->loc, "Variable");
        auto var = new Variable(name, decl);
        decl->decl = var;
        varTable.top().back().try_emplace(name, var);
    }


    void NameResolutionVisitor::declare(string const& name, NamedValNode *decl){
        checkForPreviousDecl(this, name, varTable.top().back(), decl->loc, "Parameter");
        auto var = new Variable(name, decl);
        decl->decl = var;
        varTable.top().back().try_emplace(name, var);
    }


    void NameResolutionVisitor::declareProductType(DataDeclNode *n){
        if(AnProductType::get(n->name)){
            error(n->name + " was already declared", n->loc);
        }

        AnProductType::create(n->name, {}, convertToTypeArgs(n->generics));
    }

    void NameResolutionVisitor::declareSumType(DataDeclNode *n){
        if(AnProductType::get(n->name)){
            error(n->name + " was already declared", n->loc);
        }

        AnSumType::create(n->name, {}, convertToTypeArgs(n->generics));
    }


    void NameResolutionVisitor::define(string const& name, AnDataType *dt){
        if(typeTable.size() == 1){
            //TODO: Check for redeclaration
            compUnit->userTypes.try_emplace(name, dt);
            mergedCompUnits->userTypes.try_emplace(name, dt);
        }else{
            typeTable.top().back().try_emplace(name, dt);
        }
    }


    std::optional<Variable*> NameResolutionVisitor::lookupVar(std::string const& name) const {
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
        return std::nullopt;
    }


    size_t NameResolutionVisitor::getScope() const {
        return varTable.size();
    }


    void NameResolutionVisitor::newScope(){
        varTable.top().emplace_back();
        typeTable.top().emplace_back();
    }


    void NameResolutionVisitor::exitScope(){
        varTable.top().pop_back();
        typeTable.top().pop_back();
    }


    void NameResolutionVisitor::enterFunction(){
        varTable.emplace();
        typeTable.emplace();
        newScope();
    }


    void NameResolutionVisitor::exitFunction(){
        varTable.pop();
        typeTable.pop();
    }


    FuncDecl* NameResolutionVisitor::getFunction(string const& name) const{
        return mergedCompUnits->fnDecls[name];
    }

    /** Declare function but do not define it */
    void NameResolutionVisitor::declare(FuncDeclNode *n){
        checkForPreviousDecl(this, n->name, this->compUnit->fnDecls, n->loc, "Function");

        auto *fd = new FuncDecl(n, n->name, this->mergedCompUnits);
        mergedCompUnits->fnDecls[n->name] = fd;
        compUnit->fnDecls[n->name] = fd;
        n->decl = fd;
    }

    void NameResolutionVisitor::declare(ExtNode *n){
        // Only declare methods if this is a module, not an impl
        if(n->typeExpr){
            NameResolutionVisitor submodule;
            submodule.compUnit->name = this->compUnit->name + "." + typeNodeToStr(n->typeExpr.get());

            for(auto *m : *n->methods)
                tryTo([&](){ m->accept(submodule); });
        }
    }


    void NameResolutionVisitor::visit(RootNode *n){
        if(compUnit->name != ".Stdlib.Prelude"){
            tryTo([&](){ importFile("stdlib/prelude.an", n->loc); });
        }

        for(auto &m : n->imports)
            tryTo([&](){ m->accept(*this); });
        for(auto &m : n->types)
            tryTo([&](){ m->accept(*this); });
        for(auto &m : n->traits)
            tryTo([&](){ m->accept(*this); });

        // unwrap any surrounding modifiers then declare
        for(auto &m : n->extensions){
            auto mn = m.get();
            while(dynamic_cast<ModNode*>(mn))
                mn = static_cast<ModNode*>(mn)->expr.get();

            tryTo([&](){ declare(static_cast<ExtNode*>(mn)); });
        }

        for(auto &m : n->funcs){
            auto mn = m.get();
            while(dynamic_cast<ModNode*>(mn))
                mn = static_cast<ModNode*>(mn)->expr.get();

            tryTo([&](){ declare(static_cast<FuncDeclNode*>(mn)); });
        }

        for(auto &m : n->extensions)
            tryTo([&](){ m->accept(*this); });
        for(auto &m : n->funcs)
            tryTo([&](){ m->accept(*this); });
        for(auto &m : n->main)
            tryTo([&](){ m->accept(*this); });
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
        n->setType(toAnType(n));
    }

    void NameResolutionVisitor::visit(TypeCastNode *n){
        n->rval->accept(*this);

        /*  Check for validity of cast
        if(!val){
            c->compErr("Invalid type cast " + anTypeToColoredStr(rtval.type) +
                    " -> " + anTypeToColoredStr(ty), n->loc);
        }*/
    }

    void NameResolutionVisitor::visit(UnOpNode *n){
        n->rval->accept(*this);
    }

    void NameResolutionVisitor::visit(SeqNode *n){
        for(auto &e : n->sequence){
            tryTo([&](){ e->accept(*this); });
        }
    }


    optional<string> getIdentifier(Node *n){
        if(BinOpNode *bop = dynamic_cast<BinOpNode*>(n); bop && bop->op == '.'){
            auto l = getIdentifier(bop->lval.get());
            auto r = getIdentifier(bop->rval.get());
            if(!l || !r) return std::nullopt;
            return *l + "." + *r;
        }else if(VarNode *vn = dynamic_cast<VarNode*>(n)){
            return vn->name;
        }else if(TypeNode *tn = dynamic_cast<TypeNode*>(n)){
            return typeNodeToStr(tn);
        }else{
            return std::nullopt;
        }
    }


    Declaration* NameResolutionVisitor::findCandidate(Node *n) const {
        auto name = getIdentifier(n);
        if(!name){
            return new NoDecl(n);
        }else if(auto vn = dynamic_cast<VarNode*>(n); vn && !vn->decl->isFuncDecl()){
            return vn->decl;
        }else{
            return getFunction(*name);
        }
    }


    void NameResolutionVisitor::searchForField(Node *n){
        VarNode *vn = dynamic_cast<VarNode*>(n);
        if(!vn){
            error("RHS of . operator must be an identifier", n->loc);
        }

        for(auto &p : mergedCompUnits->userTypes){
            if(auto *pt = try_cast<AnProductType>(p.second)){
                for(size_t i = 0; i < pt->fieldNames.size(); i++){
                    auto &field = pt->fieldNames[i];
                    if(field == vn->name){
                        auto *fakeDecl = new Variable(field, vn);
                        fakeDecl->tval.type = pt->fields[i];
                        vn->decl = fakeDecl;
                        return;
                    }
                }
            }
        }

        error("No field named " + vn->name + " found for any type", vn->loc);
    }


    void NameResolutionVisitor::visit(BinOpNode *n){
        if(n->op == '.' && dynamic_cast<TypeNode*>(n->lval.get())){
            //TODO: auto-module import
            return;
        }

        n->lval->accept(*this);

        if(n->op == '.' && !dynamic_cast<TypeNode*>(n->lval.get())){
            searchForField(n->rval.get());
            return;
        }

        n->rval->accept(*this);

        if(n->op != '('){
            FuncDecl *candidate = getFunction(to_string(n->op));
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

    /**
    * @brief Merges two modules
    *
    * @param mod module to merge into this
    */
    void ante::Module::import(ante::Module *mod, LOC_TY &loc){
        for(auto& pair : mod->fnDecls){
            checkForPreviousDecl(nullptr, pair.first().str(), fnDecls, pair.second->getLoc(), "Imported function", &loc);
            fnDecls[pair.first()] = pair.second;
        }

        for(auto& pair : mod->userTypes){
            auto prevDecl = userTypes.find(name);
            if(prevDecl != userTypes.end()){
                try{
                    error("Type " + name + " was already declared", loc);
                }catch(...){
                    error("The second " + name + " was imported here", loc, ErrorType::Note);
                    throw;
                }
            }
            userTypes[pair.first()] = pair.second;
        }

        for(auto& pair : mod->traits){
            auto prevDecl = traits.find(name);
            if(prevDecl != traits.end()){
                try{
                    error("Trait " + name + " was already declared", loc);
                }catch(...){
                    error("The second " + name + " was imported here", loc, ErrorType::Note);
                    throw;
                }
            }
            traits[pair.first()] = pair.second;
        }
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

    /**
    * Converts a given filename (with its file
    * extension already removed) to a module name.
    *
    * - Replaces directory separators with '.'
    * - Capitalizes first letters of words
    * - Ignores non alphanumeric characters
    */
    string toModuleName(string &s){
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


    NameResolutionVisitor visitImport(string const& file){
        NameResolutionVisitor newVisitor;

        //The lexer stores the fileName in the loc field of all Nodes. The fileName is copied
        //to let Node's outlive the context they were made in, ensuring they work with imports.
        string* fileName_cpy = new string(file);
        fileNames.emplace_back(fileName_cpy);
        setLexer(new Lexer(fileName_cpy));
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

        RootNode *root = parser::getRootNode();
        newVisitor.compUnit->ast.reset(root);

        auto fileNameWithoutExt = removeFileExt(file);
        auto modName = toModuleName(fileNameWithoutExt);
        newVisitor.compUnit->name = modName;
        newVisitor.mergedCompUnits->name = modName;

        //Add this module to the cache to ensure it is not compiled twice
        allMergedCompUnits.emplace_back(newVisitor.mergedCompUnits);
        allCompiledModules.try_emplace(file, newVisitor.compUnit);
        
        root->accept(newVisitor);
        return newVisitor;
    }


    void NameResolutionVisitor::importFile(string const& fName, LOC_TY &loc){
        //f = fName with full directory
        string fullPath = findFile(fName);
        if(fullPath.empty()){
            error("No file named '" + string(fName) + "' was found.", loc);
        }

        auto it = allCompiledModules.find(fullPath);
        if(it != allCompiledModules.end()){
            //module already compiled
            auto *import = it->getValue().get();
            string fmodName = removeFileExt(fName);

            for(auto *mod : this->imports){
                if(mod->name == fmodName){
                    error("Module " + string(fName) + " has already been imported", loc, ErrorType::Warning);
                }
            }

            this->imports.push_back(import);
            this->mergedCompUnits->import(import, loc);
        }else{
            //module not found
            NameResolutionVisitor newVisitor = visitImport(fullPath);

            //old import code
            if(newVisitor.hasError()){
                error("Error when importing '" + string(fName) + "'", loc);
            }

            this->imports.push_back(newVisitor.compUnit);
            this->mergedCompUnits->import(newVisitor.compUnit, loc);
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
            n->decl = *maybeVar;
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
        }
        n->expr->accept(*this);
    }

    void NameResolutionVisitor::visit(ExtNode *n){
        if(!static_cast<FuncDeclNode*>(n->methods.get())->decl){
            declare(n);
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
        {
            TMP_SET(autoDeclare, true);
            n->pattern->accept(*this);
        }
        newScope();
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
        for(auto *p : *n->params){
            p->accept(*this);
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
        auto *nvn = (NamedValNode*)decl->child.get();

        auto generics = convertToTypeArgs(decl->generics);
        AnSumType *data = AnSumType::get(decl->name);
        if(!data)
            data = AnSumType::create(decl->name, {}, generics);

        while(nvn){
            TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
            AnType *tagTy = tyn->extTy ? toAnType(tyn->extTy.get()) : AnType::getVoid();

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
            define(nvn->name, tagdt);

            nvn = (NamedValNode*)nvn->next.get();
        }

        data->typeTag = TT_TaggedUnion;
        data->isAlias = decl->isAlias;
        define(decl->name, data);
    }


    void NameResolutionVisitor::visit(DataDeclNode *n){
        auto *nvn = (NamedValNode*)n->child.get();
        if(!nvn) return; //type family

        if(((TypeNode*) nvn->typeExpr.get())->typeTag == TT_TaggedUnion){
            visitUnionDecl(n);
            return;
        }

        AnProductType *data = AnProductType::get(n->name);
        if(!data)
            data = AnProductType::create(n->name, {}, convertToTypeArgs(n->generics));

        define(n->name, data);
        data->fields.reserve(n->fields);
        data->fieldNames.reserve(n->fields);
        data->isAlias = n->isAlias;

        while(nvn){
            TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
            auto ty = toAnType(tyn);

            auto var = new Variable(nvn->name, n);
            nvn->decl = var;

            validateType(ty, n);

            data->fields.push_back(ty);
            data->fieldNames.push_back(nvn->name);

            nvn = (NamedValNode*)nvn->next.get();
        }
    }

    void NameResolutionVisitor::visit(TraitNode *n){
        auto tr = new Trait();
        tr->name = n->name;

        AnType *genericSelfParam = toAnType(n->selfType.get());

        // trait type is created here but the internal trait
        // tr will still be mutated with additional methods after
        AnTraitType::create(tr, genericSelfParam, convertToTypeArgs(n->generics));

        for(auto *fn : *n->child){
            fn->accept(*this);

            auto *fdn = dynamic_cast<FuncDeclNode*>(fn);
            if(fdn){
                auto *fd = static_cast<FuncDecl*>(fdn->decl);
                mergedCompUnits->fnDecls[fdn->name] = fd;
                fdn->decl = fd;
                tr->funcs.emplace_back(fd);
            }
        }
    }
}
