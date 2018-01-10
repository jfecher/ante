#ifndef AN_TYPES_H
#define AN_TYPES_H

#include "compiler.h"
#include "result.h"

#ifndef AN_USZ_SIZE
#define AN_USZ_SIZE (8*sizeof(void*))
#endif

namespace ante {

    TypedValue typeCheckWithImplicitCasts(Compiler *c, TypedValue &arg, AnType *ty);

    std::string modifiersToStr(const AnModifier *m);
    std::string anTypeToStrWithoutModifiers(const AnType *t);
    std::string anTypeToStr(const AnType *t);
    lazy_str anTypeToColoredStr(const AnType *t);

    std::string typeNodeToStr(const parser::TypeNode *t);
    lazy_str typeNodeToColoredStr(const parser::TypeNode *t);

    std::vector<std::pair<std::string, AnType*>>
    filterMatchingBindings(const AnDataType *dt, const std::vector<std::pair<std::string, AnType*>> &bindings);

    std::vector<std::pair<std::string, AnType*>>
    mapBindingsToDataType(const std::vector<AnType*> &bindings, const AnDataType *dt);

    AnAggregateType* flattenBoundTys(const Compiler *c, const AnDataType *dt);

    llvm::Type* updateLlvmTypeBinding(Compiler *c, AnDataType *dt, bool force = false);

    //conversions
    AnType* toAnType(Compiler *c, const parser::TypeNode *tn);

    llvm::Type* typeTagToLlvmType(TypeTag tagTy, llvm::LLVMContext &c);
    TypeTag llvmTypeToTypeTag(llvm::Type *t);
    std::string llvmTypeToStr(llvm::Type *ty);
    std::string typeTagToStr(TypeTag ty);
    bool llvmTypeEq(llvm::Type *l, llvm::Type *r);

    //typevar utility functions
    void validateType(Compiler *c, const AnType* tn, const parser::DataDeclNode* rootTy);
    void validateType(Compiler *c, const AnType *tn, const AnDataType *dt);
    AnType* extractTypeValue(const TypedValue &tv);
    AnType* bindGenericToType(Compiler *c, AnType *tn, const std::vector<std::pair<std::string, AnType*>> &bindings);
    AnType* bindGenericToType(Compiler *c, AnType *tn, const std::vector<AnType*> &bindings, AnDataType *dt);

    std::string getCastFnBaseName(AnType *t);

    AnType* getLargestExt(Compiler *c, AnDataType *tn, bool force = false);
    char getBitWidthOfTypeTag(const TypeTag tagTy);
    bool isPrimitiveTypeTag(TypeTag ty);
    bool isNumericTypeTag(const TypeTag ty);
    bool isIntTypeTag(const TypeTag ty);
    bool isFPTypeTag(const TypeTag tt);
    bool isUnsignedTypeTag(const TypeTag tagTy);

}

#endif
