#ifndef AN_TYPES_H
#define AN_TYPES_H

#include "compiler.h"

#ifndef AN_USZ_SIZE
#define AN_USZ_SIZE (8*sizeof(void*))
#endif

namespace ante {

    TypedValue typeCheckWithImplicitCasts(Compiler *c, TypedValue &arg, AnType *ty);

    std::string modifiersToStr(const AnModifier *m);
    std::string anTypeToStrWithoutModifiers(const AnType *t);
    std::string anTypeToStr(const AnType *t);
    lazy_str anTypeToColoredStr(const AnType *t);
    
    std::string typeNodeToStr(const TypeNode *t);
    lazy_str typeNodeToColoredStr(const TypeNode *t);

    //Typevar creation with no yy::location
    //TypeNode* mkAnonTypeNode(TypeTag);
    //TypeNode* mkTypeNodeWithExt(TypeTag tt, TypeNode *ext);
    //TypeNode* mkDataTypeNode(std::string tyname);
    //TypeNode* createFnTyNode(NamedValNode *params, TypeNode *retTy);

    //conversions
    AnType* toAnType(Compiler *c, const TypeNode *tn);

    llvm::Type* typeTagToLlvmType(TypeTag tagTy, llvm::LLVMContext &c);
    TypeTag llvmTypeToTypeTag(llvm::Type *t);
    std::string llvmTypeToStr(llvm::Type *ty);
    std::string typeTagToStr(TypeTag ty);
    bool llvmTypeEq(llvm::Type *l, llvm::Type *r);

    //typevar utility functions
    void validateType(Compiler *c, const AnType* tn, const DataDeclNode* rootTy);
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
