#ifndef AN_TYPES_H
#define AN_TYPES_H

#include "compiler.h"
#include "result.h"

#ifndef AN_USZ_SIZE
#define AN_USZ_SIZE (8*sizeof(void*))
#endif

namespace ante {

    std::string modifiersToStr(const AnModifier *m);
    std::string anTypeToStr(const AnType *t);
    lazy_str anTypeToColoredStr(const AnType *t);

    std::string typeNodeToStr(const parser::TypeNode *t);
    lazy_str typeNodeToColoredStr(const parser::TypeNode *t);

    AnAggregateType* flattenBoundTys(const Compiler *c, const AnDataType *dt);

    llvm::Type* updateLlvmTypeBinding(Compiler *c, AnDataType *dt, bool force = false);

    //conversions
    AnType* toAnType(const parser::TypeNode *tn);

    llvm::Type* typeTagToLlvmType(TypeTag tagTy, llvm::LLVMContext &c);
    TypeTag llvmTypeToTypeTag(llvm::Type *t);
    std::string llvmTypeToStr(llvm::Type *ty);
    std::string typeTagToStr(TypeTag ty);
    bool llvmTypeEq(llvm::Type *l, llvm::Type *r);

    //typevar utility functions
    void validateType(const AnType *tn, const AnDataType *dt);
    AnType* extractTypeValue(const TypedValue &tv);

    std::string getCastFnBaseName(AnType *t);

    AnType* getLargestExt(Compiler *c, AnSumType *tn, bool force = false);
    char getBitWidthOfTypeTag(const TypeTag tagTy);
    bool isPrimitiveTypeTag(TypeTag ty);
    bool isNumericTypeTag(const TypeTag ty);
    bool isIntTypeTag(const TypeTag ty);
    bool isFPTypeTag(const TypeTag tt);
    bool isUnsignedTypeTag(const TypeTag tagTy);
}

#endif
