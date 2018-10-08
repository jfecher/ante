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
    std::string anTypeToStr(const AnType *t);
    lazy_str anTypeToColoredStr(const AnType *t);

    std::string typeNodeToStr(const parser::TypeNode *t);
    lazy_str typeNodeToColoredStr(const parser::TypeNode *t);

    std::vector<TypeBinding>
    filterMatchingBindings(const AnDataType *dt, std::vector<TypeBinding> const& bindings);

    AnAggregateType* flattenBoundTys(const Compiler *c, const AnDataType *dt);

    llvm::Type* updateLlvmTypeBinding(Compiler *c, AnDataType *dt, bool force = false);

    const TypeBinding* findBindingFor(GenericTypeParam const& param,
            std::vector<TypeBinding> const& bindings);

    //conversions
    AnType* toAnType(const parser::TypeNode *tn);

    llvm::Type* typeTagToLlvmType(TypeTag tagTy, llvm::LLVMContext &c);
    TypeTag llvmTypeToTypeTag(llvm::Type *t);
    std::string llvmTypeToStr(llvm::Type *ty);
    std::string typeTagToStr(TypeTag ty);
    bool llvmTypeEq(llvm::Type *l, llvm::Type *r);

    /**
    * @brief Base for typeeq
    *
    * @param l Type to check
    * @param r Type to check against
    * @param tcr This parameter is passed recursively, pass a TypeCheckResult::Success
    * if at the beginning of the chain
    *
    * @return The resulting TypeCheckResult
    */
    TypeCheckResult& typeEqBase(const AnType *l, const AnType *r, TypeCheckResult &tcr);

    /** @brief Performs a type check against l and r */
    TypeCheckResult typeEq(const AnType *l, const AnType *r);

    /**
        * @brief Performs a type check against l and r
        *
        * Used for function parameters and similar situations where typevars
        * across multiple type checks need to be consistent.  Eg. a function
        * of type ('t, 't)->void should not match the arguments i32 and u64.
        * Performing a typecheck on each argument separately would give a different
        * bound value for 't.  Using this function would result in the appropriate
        * TypeCheckResult::Failure
        */
    TypeCheckResult typeEq(std::vector<AnType*> l, std::vector<AnType*> r);


    //typevar utility functions
    void validateType(const AnType *tn, const AnDataType *dt);
    AnType* extractTypeValue(const TypedValue &tv);
    AnType* bindGenericToType(AnType *tn, const std::vector<TypeBinding> &bindings);

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
