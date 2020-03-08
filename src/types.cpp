#include <types.h>
#include <trait.h>
#include <nameresolution.h>
#include <util.h>
using namespace std;
using namespace llvm;
using namespace ante::parser;

namespace ante {

char getBitWidthOfTypeTag(const TypeTag ty){
    switch(ty){
        case TT_I8:  case TT_U8: case TT_C8:  return 8;
        case TT_I16: case TT_U16: case TT_F16: return 16;
        case TT_I32: case TT_U32: case TT_F32: return 32;
        case TT_I64: case TT_U64: case TT_F64: return 64;
        case TT_Isz: case TT_Usz: return AN_USZ_SIZE;
        case TT_Bool: return 8;

        case TT_Ptr:
        case TT_Function:
            return AN_USZ_SIZE;

        default:
            return 0;
    }
}

AnType* findBinding(Substitutions const& subs, const AnType *key){
    for(auto it = subs.rbegin(); it != subs.rend(); ++it){
        if(it->first == key){
            return it->second;
        }
    }
    return nullptr;
}

// TODO: Remove hardcoded check for Type type,
//       Add check for if an entire tuple/record type is empty
//       or full of only other empty types
bool isEmptyType(Compiler *c, AnType *ty){
    auto tv = try_cast<AnTypeVarType>(ty);
    if(tv){
        auto binding = findBinding(c->compCtxt->monomorphisationMappings, tv);
        return binding ? isEmptyType(c, binding) : true;
    }
    return ty->typeTag == TT_Unit
        || ty->hasModifier(Tok_Ante)
        || (ty->typeTag == TT_Data && try_cast<AnDataType>(ty)->name == "Type");
}


/*
 *  Returns the TypeNode* value of a TypedValue of type TT_Type
 */
AnType* extractTypeValue(const TypedValue &tv){
    auto zext = dyn_cast<ConstantInt>(tv.val)->getZExtValue();
    return (AnType*) zext;
}

Result<size_t, string> AnType::getSizeInBits(Compiler *c, string const& incompleteType) const{
    size_t total = 0;

    if(isPrimitiveTypeTag(this->typeTag))
        return getBitWidthOfTypeTag(this->typeTag);

    if(auto *dataTy = try_cast<AnDataType>(this)){
        if(dataTy->name == incompleteType){
            cerr << "Incomplete type " << anTypeToColoredStr(this) << endl;
            throw IncompleteTypeError();
        }

        return dataTy->decl->getSizeInBits(c, incompleteType, dataTy);

    // function & metafunction are aggregate types but have different sizes than
    // a tuple so this case must be checked for before AnTupleType is
    }else if(typeTag == TT_Ptr || typeTag == TT_Function){
        return AN_USZ_SIZE;

    }else if(auto *tup = try_cast<AnTupleType>(this)){
        for(auto *ext : tup->fields){
            auto val = ext->getSizeInBits(c, incompleteType);
            if(!val) return val;
            total += val.getVal();
        }

    }else if(auto *arr = try_cast<AnArrayType>(this)){
        auto val = arr->extTy->getSizeInBits(c, incompleteType);
        if(!val) return val;
        return arr->len * val.getVal();

    }else if(auto *tvt = try_cast<AnTypeVarType>(this)){
        AnType *lookup = findBinding(c->compCtxt->monomorphisationMappings, tvt);
        if(lookup)
            return lookup->getSizeInBits(c);
        else{
            // typevars that survive monomorphisation are values that are never used,
            // so we treat them as () here
            return 0; //"Unknown typevar " + tvt->name;
        }
    }

    return total;
}


size_t hashCombine(size_t l, size_t r){
    return l ^ (r + AN_HASH_PRIME + (l << 6) + (l >> 2));
}

bool isSignedTypeTag(const TypeTag tt){
    return tt==TT_I8||tt==TT_I16||tt==TT_I32||tt==TT_I64||tt==TT_Isz;
}

bool isUnsignedTypeTag(const TypeTag tt){
    return tt==TT_U8||tt==TT_U16||tt==TT_U32||tt==TT_U64||tt==TT_Usz||tt==TT_C8;
}

bool isIntegerTypeTag(const TypeTag ty){
    return isSignedTypeTag(ty) || isUnsignedTypeTag(ty);
}

bool isFloatTypeTag(const TypeTag tt){
    return tt == TT_F16 || tt == TT_F32 || tt == TT_F64;
}

bool isNumericTypeTag(const TypeTag ty){
    return isIntegerTypeTag(ty) || isFloatTypeTag(ty);
}

/*
 *  Returns true if the given typetag is a primitive type, and thus
 *  accurately represents the entire type without information loss.
 *  NOTE: this function relies on the fact all primitive types are
 *        declared before non-primitive types in the TypeTag definition.
 */
bool isPrimitiveTypeTag(TypeTag ty){
    return ty >= 0 && ty <= numPrimitiveTypeTags;
}

/*
 *  Translates an individual TypeTag to an llvm::Type.
 *  Only intended for primitive types, as there is not enough
 *  information stored in a TypeTag to convert to array, tuple,
 *  or function types.
 */
Type* typeTagToLlvmType(TypeTag ty, LLVMContext &ctxt){
    switch(ty){
        case TT_I8:  case TT_U8:  return Type::getInt8Ty(ctxt);
        case TT_I16: case TT_U16: return Type::getInt16Ty(ctxt);
        case TT_I32: case TT_U32: return Type::getInt32Ty(ctxt);
        case TT_I64: case TT_U64: return Type::getInt64Ty(ctxt);
        case TT_Isz:    return Type::getIntNTy(ctxt, AN_USZ_SIZE);
        case TT_Usz:    return Type::getIntNTy(ctxt, AN_USZ_SIZE);
        case TT_F16:    return Type::getHalfTy(ctxt);
        case TT_F32:    return Type::getFloatTy(ctxt);
        case TT_F64:    return Type::getDoubleTy(ctxt);
        case TT_C8:     return Type::getInt8Ty(ctxt);
        case TT_Bool:   return Type::getInt1Ty(ctxt);
        case TT_Unit:   return Type::getVoidTy(ctxt);
        case TT_TypeVar:
            throw TypeVarError();
        default:
            cerr << "typeTagToLlvmType: Unknown/Unsupported TypeTag " << ty << ", exiting.\n";
            exit(1);
    }
}

/*
 *  Converts a TypeNode to an llvm::Type.  While much less information is lost than
 *  llvmTypeToTokType, information on signedness of integers is still lost, causing the
 *  unfortunate necessity for the use of a TypedValue for the storage of this information.
 */
Type* Compiler::anTypeToLlvmType(const AnType *ty, int recursionLimit){
    vector<Type*> tys;
    if(!recursionLimit){
        ASSERT_UNREACHABLE("anTypeToLlvmType hit internal recursion limit");
    }

    if(ty->hasModifier(Tok_Mut)){
        auto bm = dynamic_cast<const BasicModifier*>(ty);
        return anTypeToLlvmType(bm->extTy, --recursionLimit)->getPointerTo();
    }

    switch(ty->typeTag){
        case TT_Ptr: {
            auto *ptr = cast<AnPtrType>(ty);
            return isEmptyType(this, ptr->elemTy) ?
                Type::getInt8Ty(*ctxt)->getPointerTo() :
                anTypeToLlvmType(ptr->elemTy, --recursionLimit)->getPointerTo();
        }
        case TT_Array:{
            auto *arr = cast<AnArrayType>(ty);
            return ArrayType::get(anTypeToLlvmType(arr->extTy, --recursionLimit), arr->len);
        }
        case TT_Tuple:
            for(auto *e : cast<AnTupleType>(ty)->fields){
                if(!isEmptyType(this, e))
                    tys.push_back(anTypeToLlvmType(e, --recursionLimit));
            }
            return StructType::get(*ctxt, tys);
        case TT_Data: {
            auto *dt = cast<AnDataType>(ty);
            return dt->decl->toLlvmType(this, dt);
        }
        case TT_Function: {
            auto *f = try_cast<AnFunctionType>(ty);
            for(size_t i = 0; i < f->paramTys.size(); i++){
                if(f->paramTys[i]->isRowVar()){
                    return FunctionType::get(anTypeToLlvmType(f->retTy, --recursionLimit), tys, true)->getPointerTo();
                }
                // All Ante functions take at least 1 arg: (), which are ignored in llvm ir
                // and translated to 0 arg functions instead
                if(!isEmptyType(this, f->paramTys[i]))
                    tys.push_back(anTypeToLlvmType(f->paramTys[i], --recursionLimit));
            }

            return FunctionType::get(anTypeToLlvmType(f->retTy, --recursionLimit), tys, false)->getPointerTo();
        }
        case TT_TypeVar: {
            auto binding = findBinding(compCtxt->monomorphisationMappings, ty); 
            if(binding){
                return anTypeToLlvmType(binding, --recursionLimit);
             }else{
                 // typevars that survive monomorphisation are values that are never used,
                 // so we treat them as () here
                 auto unit = AnType::getUnit();
                 compCtxt->insertMonomorphisationMappings({{(AnType*)ty, unit}});
                 return anTypeToLlvmType(unit, --recursionLimit);
            }
            std::cerr << "Typevar: " << (AnType*)ty << '\n' << "Bindings: " << compCtxt->monomorphisationMappings << '\n';
            ASSERT_UNREACHABLE("Unbound typevar found during monomorphisation");
        }
        default:
            return typeTagToLlvmType(ty->typeTag, *ctxt);
    }
}

/*
 *  Converts a TypeTag to its string equivalent for
 *  helpful error messages.  For most cases, llvmTypeToStr
 *  should be used instead to provide the full type.
 */
string typeTagToStr(TypeTag ty){
    switch(ty){
        case TT_I8:    return "i8" ;
        case TT_I16:   return "i16";
        case TT_I32:   return "i32";
        case TT_I64:   return "i64";
        case TT_U8:    return "u8" ;
        case TT_U16:   return "u16";
        case TT_U32:   return "u32";
        case TT_U64:   return "u64";
        case TT_F16:   return "f16";
        case TT_F32:   return "f32";
        case TT_F64:   return "f64";
        case TT_Isz:   return "isz";
        case TT_Usz:   return "usz";
        case TT_C8:    return "c8" ;
        case TT_Bool:  return "bool";
        case TT_Unit:  return "unit";

        /*
         * Because of the loss of specificity for these last types,
         * these strings are most likely insufficient.  The entire
         * AnType should be preferred to be used instead.
         */
        case TT_Tuple:        return "Tuple";
        case TT_Array:        return "Array";
        case TT_Ptr:          return "Ptr"  ;
        case TT_Data:         return "Data" ;
        case TT_TypeVar:      return "'t";
        case TT_Function:     return "Function";
    }
    cerr << "typeTag = " << ty << endl;
    ASSERT_UNREACHABLE("Unhandled typetag in typeTagToStr");
    return "";
}

bool shouldWrapInParenthesis(TypeNode *type){
    return !type->params.empty() || type->typeTag == TT_Array;
}

/*
 *  Converts a typeNode directly to a string with no information loss.
 *  Used in ExtNode::compile
 */
string typeNodeToStr(const TypeNode *t){
    if(!t) return "null";

    if(t->typeTag == TT_Tuple){
        string ret = "(";
        TypeNode *elem = t->extTy.get();
        while(elem){
            if(elem->next.get())
                ret += typeNodeToStr(elem) + ", ";
            else
                ret += typeNodeToStr(elem) + ")";
            elem = (TypeNode*)elem->next.get();
        }
        return ret;
    }else if(t->typeTag == TT_Data or t->typeTag == TT_TypeVar){
        string name = t->typeName;
        if(!t->params.empty()){
            for(auto &param : t->params){
                auto pstr = typeNodeToStr(param.get());
                if(shouldWrapInParenthesis(param.get())) name += " (" + pstr + ")";
                else name += ' ' + pstr;
            }
        }
        return name;
    }else if(t->typeTag == TT_Array){
        auto *len = (IntLitNode*)t->extTy->next.get();
        return '[' + len->val + " " + typeNodeToStr(t->extTy.get()) + ']';
    }else if(t->typeTag == TT_Ptr){
        return "ref " + typeNodeToStr(t->extTy.get());
    }else if(t->typeTag == TT_Function){
        string ret = "";
        string retTy = typeNodeToStr(t->extTy.get());
        TypeNode *cur = (TypeNode*)t->extTy->next.get();
        while(cur){
            auto pstr = typeNodeToStr(cur);
            if(shouldWrapInParenthesis(cur)) ret += "(" + pstr + ") ";
            else ret += pstr + ' ';
            cur = (TypeNode*)cur->next.get();
        }
        return ret + "-> " + retTy;
    }else{
        return typeTagToStr(t->typeTag);
    }
}


/**
 * true if the type should be wrapped in parenthesis
 * when being outputted as a string as a datatype typeArg
 *
 * eg.  in  Vec (Vec i32)
 * type = (Vec i32) and the return value should be true.
 */
bool shouldWrapInParenthesis(AnType *type){
    //Quick and dirty checks just to see if we need parenthesis wrapping the type or not
    if(ante::isPrimitiveTypeTag(type->typeTag) || type->typeTag == TT_TypeVar || type->typeTag == TT_Array)
        return false;

    if(type->typeTag == TT_Ptr || type->typeTag == TT_Function)
        return true;

    auto adt = try_cast<AnDataType>(type);
    if (!adt) return false;
    return !adt->typeArgs.empty();
}


string traitToStr(const TraitImpl *trait){
    string ret = trait->getName();

    for(auto &type : trait->typeArgs){
        if(shouldWrapInParenthesis(type))
            ret += " (" + anTypeToStr(type) + ')';
        else
            ret += ' ' + anTypeToStr(type);
    }
    return ret;
}


lazy_str traitToColoredStr(const TraitImpl *trait){
    return lazy_str(traitToStr(trait), AN_TYPE_COLOR);
}


string commaSeparated(std::vector<TraitImpl*> const& traits){
    string ret = "";
    for(const auto &tr : traits){
        ret += traitToStr(tr);
        if(&tr != &traits.back())
            ret += ", ";
    }
    return ret;
}


string anTypeToStr(const AnType *t){
    if(!t) return "(null)";

    /** Must check for modifiers first as they can be lost after dyn_cast */
    if(t->isModifierType()){
        if(auto *mod = dynamic_cast<const BasicModifier*>(t)){
            return Lexer::getTokStr(mod->mod) + ' ' + anTypeToStr(mod->extTy);

        }else if(auto *cdmod = dynamic_cast<const CompilerDirectiveModifier*>(t)){
            //TODO: modify printingvisitor to print to streams
            // PrintingVisitor::print(cdmod->directive.get());
            return anTypeToStr(cdmod->extTy);
        }else{
            return "(unknown modifier type)";
        }
    }else if(auto *dt = try_cast<AnDataType>(t)){
        string n = dt->name;

        for(auto &a : dt->typeArgs){
            if(shouldWrapInParenthesis(a))
                n += " (" + anTypeToStr(a) + ')';
            else
                n += ' ' + anTypeToStr(a);
        }
        return n;
    }else if(auto *tvt = try_cast<AnTypeVarType>(t)){
        return tvt->name;
    }else if(auto *f = try_cast<AnFunctionType>(t)){
        string ret = "";
        for(auto &param : f->paramTys){
            auto pstr = anTypeToStr(param);
            ret += (shouldWrapInParenthesis(param) ? '(' + pstr + ')' : pstr) + ' ';
        }

        string tcConstraints = f->typeClassConstraints.empty() ? ""
            : " given " + commaSeparated(f->typeClassConstraints);

        return ret + "-> " + anTypeToStr(f->retTy) + tcConstraints;
    }else if(auto *tup = try_cast<AnTupleType>(t)){
        string ret = "(";

        for(const auto &field : tup->fields){
            ret += anTypeToStr(field);

            if(&field != &tup->fields.back()){
                ret += ", ";
            }else if(tup->fields.size() == 1){
                ret += ',';
            }
        }
        return ret + ")";
    }else if(auto *arr = try_cast<AnArrayType>(t)){
        return '[' + to_string(arr->len) + " " + anTypeToStr(arr->extTy) + ']';
    }else if(auto *ptr = try_cast<AnPtrType>(t)){
        return "ref " + anTypeToStr(ptr->elemTy);
    }else{
        return typeTagToStr(t->typeTag);
    }
}

} //end of namespace ante
