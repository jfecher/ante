#ifndef AN_TYPES_H
#define AN_TYPES_H

#include "compiler.h"
#include "result.h"
#include "module.h"

#ifndef AN_USZ_SIZE
#define AN_USZ_SIZE (8*sizeof(void*))
#endif

namespace ante {

    std::string modifiersToStr(const AnModifier *m);
    std::string anTypeToStr(const AnType *t);
    lazy_str anTypeToColoredStr(const AnType *t);

    std::string typeNodeToStr(const parser::TypeNode *t);
    lazy_str typeNodeToColoredStr(const parser::TypeNode *t);

    AnTupleType* flattenBoundTys(const Compiler *c, const AnDataType *dt);

    llvm::Type* updateLlvmTypeBinding(Compiler *c, AnDataType *dt);

    bool isEmptyType(Compiler *c, AnType *ty);

    //conversions
    AnType* toAnType(const parser::TypeNode *tn, Module *module);

    llvm::Type* typeTagToLlvmType(TypeTag tagTy, llvm::LLVMContext &c);
    TypeTag llvmTypeToTypeTag(llvm::Type *t);
    std::string llvmTypeToStr(llvm::Type *ty);
    std::string typeTagToStr(TypeTag ty);
    bool llvmTypeEq(llvm::Type *l, llvm::Type *r);

    //typevar utility functions
    void validateType(const AnType *tn, const AnDataType *dt);
    AnType* extractTypeValue(const TypedValue &tv);

    std::string getCastFnBaseName(AnType *t);

    /** Return the size of the given type in bits. Only for use on
     *  primitive, pointer, or function types.
     *
     * Note: Returns 0 for any aggregate type (sum/product types) that
     *       can't be described by typetag alone.
     */
    char getBitWidthOfTypeTag(const TypeTag tagTy);

    /** True for any type that has no additional type arguments.
     *  As a result, a primitive typetag will always be a base
     *  case for recursion over AnTypes. */
    bool isPrimitiveTypeTag(TypeTag ty);

    /** Shortcut for isIntTypeTag(t) || isFPTypeTag(t) */
    bool isNumericTypeTag(const TypeTag ty);

    /** True if tagTy is an integer typetag, signed or unsigned.
     *  Currently also returns true for c8 for purposes of integer
     *  conversions, but this is subject to change. */
    bool isIntegerTypeTag(const TypeTag ty);

    /** True if tagTy is floating point type (f16, f32, or f64) */
    bool isFloatTypeTag(const TypeTag tt);

    /** True if tagTy is an unsigned (integer) TypeTag */
    bool isUnsignedTypeTag(const TypeTag tagTy);

    /** True if tagTy is a signed integer TypeTag */
    bool isSignedTypeTag(const TypeTag tagTy);
}
#endif
