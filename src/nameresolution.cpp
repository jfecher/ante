#include "nameresolution.h"
#include "compiler.h"
#include "target.h"
#include "types.h"
#include "uniontag.h"
#include "nodecl.h"
#include "scopeguard.h"

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

    void NameResolutionVisitor::error(lazy_printer msg, LOC_TY loc, ErrorType t){
        ante::error(msg, loc, t);
        if(t == ErrorType::Error){
            errFlag = true;
            throw new CompilationError(msg, loc);
        }
    }

    /**
    * Converts a vector of typevar TypeNodes to a vector of GenericTypeParams
    * to be the positional generics of a generic datatype.
    *
    * Note that because the parent datatype is expected to not yet be created,
    * the 'dt' field of the GenericTypeParams will need to be updated afterward.
    */
    vector<GenericTypeParam>
    createStructuralGenericParams(vector<unique_ptr<TypeNode>> const& generics){
        auto ret = vecOf<GenericTypeParam>(generics.size());
        for(size_t i = 0; i < generics.size(); i++){
            TypeNode *tn = generics[i].get();
            ret.emplace_back(tn->typeName, nullptr, i);
        }
        return ret;
    }


    void NameResolutionVisitor::declare(string const& name, VarNode *decl){
        auto var = new Variable(name, decl);
        decl->decls.push_back(var);
        varTable.top().back().try_emplace(name, var);
    }


    void NameResolutionVisitor::declare(string const& name, NamedValNode *decl){
        auto var = new Variable(name, decl);
        decl->decls.push_back(var);
        varTable.top().back().try_emplace(name, var);
    }


    void NameResolutionVisitor::declare(string const& name, vector<unique_ptr<TypeNode>> const& generics){
        auto genericParams = createStructuralGenericParams(generics);
        AnDataType::create(name, {}, false, genericParams);
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


    vector<FuncDecl*>& NameResolutionVisitor::getFunctionList(string const& name) const{
        return mergedCompUnits->fnDecls[name];
    }


    void NameResolutionVisitor::visit(RootNode *n){
        for(auto &m : n->imports)
            m->accept(*this);
        for(auto &m : n->types)
            m->accept(*this);
        for(auto &m : n->traits)
            m->accept(*this);
        for(auto &m : n->extensions)
            m->accept(*this);
        for(auto &m : n->funcs)
            m->accept(*this);
        for(auto &m : n->main)
            m->accept(*this);
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
    }

    void NameResolutionVisitor::visit(UnOpNode *n){
        n->rval->accept(*this);
    }

    void NameResolutionVisitor::visit(SeqNode *n){
        for(auto &e : n->sequence)
            e->accept(*this);
    }


    optional<string> getIdentifier(Node *n){
        if(BinOpNode *bop = dynamic_cast<BinOpNode*>(n); bop && bop->op == '.'){
            auto l = getIdentifier(bop->lval.get());
            auto r = getIdentifier(bop->rval.get());
            if(!l || !r) return std::nullopt;
            return *l + "." + *r;
        }else if(VarNode *vn = dynamic_cast<VarNode*>(n)){
            return vn->name;
        }else{
            return std::nullopt;
        }
    }


    vector<Declaration*> NameResolutionVisitor::findCandidates(Node *n) const {
        auto name = getIdentifier(n);
        if(!name){
            return {new NoDecl(n)};
        }else{
            auto decls = getFunctionList(*name);
            auto ret = vecOf<Declaration*>(decls.size());
            for(auto &decl : decls)
                ret.push_back(decl);

            if(auto vn = dynamic_cast<VarNode*>(n); vn && !vn->decls[0]->isFuncDecl()){
                ret.push_back(vn->decls[0]);
            }

            return ret;
        }
    }

    void NameResolutionVisitor::visit(BinOpNode *n){
        n->lval->accept(*this);
        n->rval->accept(*this);

        if(n->op != '('){
            auto &candidates = getFunctionList(to_string(n->op));
            n->decls.reserve(candidates.size());
            for(auto &c : candidates)
                n->decls.push_back(c);

            if(candidates.empty()) //v TODO: memory leak here
                n->decls.push_back(new NoDecl(n));
        }else{
            n->decls = findCandidates(n->lval.get());
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
    void ante::Module::import(ante::Module *mod){
        for(auto& pair : mod->fnDecls)
            for(auto& fd : pair.second)
                fnDecls[pair.first()].push_back(fd);

        for(auto& pair : mod->userTypes)
            userTypes[pair.first()] = pair.second;

        for(auto& pair : mod->traits)
            traits[pair.first()] = pair.second;
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

            fputs("Syntax error, aborting.\n", stderr);
            exit(flag);
        }
        newVisitor.compUnit->ast.reset(parser::getRootNode());

        auto fileNameWithoutExt = removeFileExt(file);
        auto modName = toModuleName(fileNameWithoutExt);
        newVisitor.compUnit->name = modName;
        newVisitor.mergedCompUnits->name = modName;

        //Add this module to the cache to ensure it is not compiled twice
        allMergedCompUnits.emplace_back(newVisitor.mergedCompUnits);
        allCompiledModules.try_emplace(file, newVisitor.compUnit);
        return newVisitor;
    }


    void importFile(NameResolutionVisitor &v, string const& fName, LOC_TY &loc){
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

            for(auto *mod : v.imports){
                if(mod->name == fmodName){
                    error("Module " + string(fName) + " has already been imported", loc, ErrorType::Warning);
                    return;
                }
            }

            v.imports.push_back(import);
            v.mergedCompUnits->import(import);
        }else{
            //module not found
            NameResolutionVisitor newVisitor = visitImport(fullPath);

            //old import code
            if(newVisitor.errFlag){
                error("Error when importing '" + string(fName) + "'", loc);
            }

            v.imports.push_back(newVisitor.compUnit);
            v.mergedCompUnits->import(newVisitor.compUnit);
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

        importFile(*this, path.c_str(), n->loc);
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
            n->decls = {*maybeVar};
        }else{
            //if this is a function, then there can be multiple candidates of
            //the same name that must be filtered at the callsite
            auto& fnlist = getFunctionList(n->name);

            if(fnlist.size() == 1){
                auto& fd = *fnlist.begin();
                n->decls = {fd};
            }else if(fnlist.empty()){
                error("Variable or function '" + n->name + "' has not been declared.", n->loc);
            }else{
                // Cannot do a simple assignment since vector<ParentClass> != vector<BaseClass>
                n->decls.reserve(fnlist.size());
                for(auto *funcDecl : fnlist)
                    n->decls.push_back(funcDecl);
            }
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
        //TODO: declare methods contained within submodules
        for(auto *m : *n->methods)
            m->accept(*this);
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
        auto *fd = new FuncDecl(n, n->name, this->mergedCompUnits);
        mergedCompUnits->fnDecls[n->name].push_back(fd);
        n->decl = fd;

        enterFunction();
        for(auto *p : *n->params){
            p->accept(*this);
        }

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
    void NameResolutionVisitor::validateType(const AnType *tn, const DataDeclNode *rootTy) {
        if(!tn) return;

        if(tn->typeTag == TT_Data or tn->typeTag == TT_TaggedUnion){
            auto *dataTy = try_cast<AnDataType>(tn);

            if(dataTy->isStub()){
                if(dataTy->name == rootTy->name){
                    error("Recursive types are disallowed, wrap the type in a pointer instead", rootTy->loc);
                }

                error("Type "+dataTy->name+" has not been declared", rootTy->loc);
            }

            for(auto *t : dataTy->extTys)
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
        auto generics = createStructuralGenericParams(decl->generics);
        AnDataType *data = AnDataType::create(decl->name, {}, true, generics);

        while(nvn){
            TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
            AnType *tagTy = tyn->extTy ? toAnType(tyn->extTy.get()) : AnType::getVoid();

            vector<AnType*> exts;
            if(tagTy->typeTag == TT_Tuple){
                exts = try_cast<AnAggregateType>(tagTy)->extTys;
            }else{
                exts.push_back(tagTy);
            }

            //Each union member's type is a tuple of the tag (a u8 value), and the user-defined value
            auto *tup = AnAggregateType::get(TT_Tuple, {AnType::getU8(), tagTy});

            //Store the tag as a UnionTag and a AnDataType
            AnDataType *tagdt = AnDataType::create(nvn->name, exts, false, generics);

            //A tag's generics should have their dt set to the dt for the whole union
            for(auto &g : tagdt->generics){ g.dt = data; }

            tagdt->fields.emplace_back(decl->name);
            tagdt->parentUnionType = data;
            tagdt->isGeneric = isGeneric(exts);

            //Store tag vals as a UnionTag
            UnionTag *tag = new UnionTag(nvn->name, tagdt, data, data->tags.size());
            data->tags.emplace_back(tag);

            data->extTys.push_back(tup);

            validateType(tagTy, decl);
            define(nvn->name, tagdt);

            nvn = (NamedValNode*)nvn->next.get();
        }

        data->typeTag = TT_TaggedUnion;
        data->isAlias = decl->isAlias;
        define(decl->name, data);

        /* TODO: rebind generics
        for(auto &v : data->variants){
            v->extTys = data->extTys;
            v->isGeneric = data->isGeneric;
            v->typeTag = data->typeTag;
            v->tags = tags;
            v->unboundType = data;
            *v = *try_cast<AnDataType>(bindGenericToType(c, v, v->boundGenerics));
            if(v->parentUnionType)
                v->parentUnionType = try_cast<AnDataType>(bindGenericToType(c, v->parentUnionType, v->parentUnionType->boundGenerics));
            addGenerics(v->generics, v->extTys);
        } */
    }


    void NameResolutionVisitor::visit(DataDeclNode *n){
        auto *nvn = (NamedValNode*)n->child.get();
        if(((TypeNode*) nvn->typeExpr.get())->typeTag == TT_TaggedUnion){
            visitUnionDecl(n);
            return;
        }

        //Create the DataType as a stub first, have its contents be recursive
        //just to cause an error if something tries to use the stub
        auto generics = createStructuralGenericParams(n->generics);
        AnDataType *data = AnDataType::create(n->name, {}, true, generics);

        define(n->name, data);
        data->fields.reserve(n->fields);
        data->extTys.reserve(n->fields);
        data->isAlias = n->isAlias;

        while(nvn){
            TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
            auto ty = toAnType(tyn);

            validateType(ty, n);

            data->extTys.push_back(ty);
            data->fields.push_back(nvn->name);

            nvn = (NamedValNode*)nvn->next.get();
        }

        /* TODO: rebind generics
        for(auto &v : data->variants){
            v->extTys = data->extTys;
            v->isGeneric = data->isGeneric;
            v->typeTag = data->typeTag;
            v->fields = data->fields;
            v->unboundType = data;
            *v = *try_cast<AnDataType>(bindGenericToType(c, v, v->boundGenerics));
            if(v->parentUnionType)
                v->parentUnionType = try_cast<AnDataType>(bindGenericToType(c, v->parentUnionType, v->parentUnionType->boundGenerics));
            addGenerics(v->generics, v->extTys);
        }
        */
    }

    void NameResolutionVisitor::visit(TraitNode *n){
        for(auto *fn : *n->child){
            auto *fdn = static_cast<FuncDeclNode*>(fn);
            auto *fd = new FuncDecl(fdn, fdn->name, this->mergedCompUnits);
            mergedCompUnits->fnDecls[fdn->name].push_back(fd);
        }
    }
}
