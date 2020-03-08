#include "typedecl.h"
#include "antype.h"
#include "compiler.h"
#include "util.h"

namespace ante {
    using std::vector;
    using std::string;

    void TypeDecl::addField(std::string const& name, AnType *type){
        fields.push_back(name);
        fieldTypes.push_back(type);
    }

    vector<AnType*> TypeDecl::getBoundFieldTypes(const AnDataType *dt) const {
        auto baseType = cast<AnDataType>(this->type);
        auto pair = ante::tryUnify(baseType->typeArgs, dt->typeArgs);
        assert(pair.first);

        return ante::applyToAll(fieldTypes, [&](AnType *field){
            return ante::applySubstitutions(pair.second, field);
        });
    }

    std::vector<AnType*>& TypeDecl::getUnboundFieldTypes(){
        return fieldTypes;
    }

    vector<AnType*> TypeDecl::getLargestVariantFieldTypes(Compiler *c, const AnDataType *dt) const {
        auto variant = dt->decl->getLargestVariant(c, dt);
        vector<AnType*> fields = { AnType::getU8(), const_cast<AnType*>(variant) };

        auto baseType = cast<AnDataType>(this->type);
        auto pair = ante::tryUnify(baseType->typeArgs, dt->typeArgs);
        assert(pair.first);

        return ante::applyToAll(fields, [&](AnType *field){
            return ante::applySubstitutions(pair.second, field);
        });
    }

    llvm::Type* TypeDecl::toLlvmType(Compiler *c, const AnDataType *dt) const {
        auto it = variantTypes.find(dt->typeArgs);
        if(it != variantTypes.end()){
            return it->second;
        }

        bool isPacked = this->isUnionType;
        auto structTy = llvm::StructType::create(*c->ctxt, dt->name);

        variantTypes[dt->typeArgs] = structTy;

        vector<AnType*> fields = isUnionType
            ? getLargestVariantFieldTypes(c, dt)
            : getBoundFieldTypes(dt);

        auto tys = ante::applyToAll(fields, [&](AnType *t){
            return c->anTypeToLlvmType(t);
        });

        structTy->setBody(tys, isPacked);
        return structTy;
    }

    Result<size_t, string>
    TypeDecl::getSizeInBits(Compiler *c, string const& incompleteType, const AnDataType *type) const {
        if(isUnionType){
            auto size = getLargestVariant(c, type)->getSizeInBits(c, incompleteType);
            if(size) return size.getVal() + 8;
            else return size;
        }else{
            auto t = AnTupleType::get(getBoundFieldTypes(type));
            auto size = t->getSizeInBits(c, incompleteType);
            delete t;
            return size;
        }
    }

    size_t TypeDecl::getFieldIndex(std::string const& field) const {
        assert(!isUnionType);
        auto it = ante::find(fields, field);
        assert(it != fields.end());
        return it - fields.begin();
    }

    size_t TypeDecl::getTagIndex(std::string const& tag) const {
        assert(isUnionType);
        auto it = ante::find(fields, tag);
        assert(it != fields.end());
        return it - fields.begin();
    }

    const AnType* TypeDecl::getLargestVariant(Compiler *c, const AnDataType *type) const {
        const AnType* largest = nullptr;
        size_t largestSize = 0;

        auto fields = getBoundFieldTypes(type);
        for(auto field : fields){
            auto size = field->getSizeInBits(c);
            if(size.getVal() >= largestSize){
                largest = field;
                largestSize = size.getVal();
            }
        }

        return largest;
    }

    llvm::Value* TypeDecl::getTagValue(Compiler *c, const AnDataType *type,
            string const& variantName, vector<TypedValue> const& args) const {

        vector<llvm::Type*> unionTys;
        vector<llvm::Constant*> unionVals;

        size_t tag = type->decl->getTagIndex(variantName);
        unionTys.push_back(llvm::Type::getInt8Ty(*c->ctxt));
        unionVals.push_back(llvm::ConstantInt::get(*c->ctxt, llvm::APInt(8, tag, true))); //tag

        for(auto &tval : args){
            unionTys.push_back(tval.getType());
            unionVals.push_back(llvm::UndefValue::get(tval.getType()));
        }

        auto unionTy = c->anTypeToLlvmType(type);

        //create a struct of (u8 tag, <union member type>)
        auto structTy = llvm::StructType::get(*c->ctxt, unionTys, true);
        llvm::Value *taggedUnion = llvm::ConstantStruct::get(structTy, unionVals);
        size_t i = 0;
        for(auto &arg : args){
            taggedUnion = c->builder.CreateInsertValue(taggedUnion, arg.val, ++i);
        }

        //allocate for the largest possible union member
        auto *alloca = c->builder.CreateAlloca(unionTy);

        //but bitcast it the the current member
        auto *castTo = c->builder.CreateBitCast(alloca, taggedUnion->getType()->getPointerTo());
        c->builder.CreateStore(taggedUnion, castTo);

        //load the original alloca, not the bitcasted one
        llvm::Value *unionVal = c->builder.CreateLoad(alloca);

        return unionVal;
    }
}
