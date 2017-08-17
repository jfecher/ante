#ifndef AN_TYPES_H
#define AN_TYPES_H

#include "compiler.h"

#ifndef AN_USZ_SIZE
#define AN_USZ_SIZE (8*sizeof(void*))
#endif

namespace ante {

    TypedValue* typeCheckWithImplicitCasts(Compiler *c, TypedValue *arg, TypeNode *ty);

    TypeNode* deepCopyTypeNode(const TypeNode *n);
    std::string typeNodeToStr(const TypeNode *t);
    lazy_str typeNodeToColoredStr(const TypeNode *t);
    lazy_str typeNodeToColoredStr(const std::unique_ptr<TypeNode>& tn);

    //Typevar creation with no yy::location
    TypeNode* mkAnonTypeNode(TypeTag);
    TypeNode* mkTypeNodeWithExt(TypeTag tt, TypeNode *ext);
    TypeNode* mkDataTypeNode(std::string tyname);
    TypeNode* createFnTyNode(NamedValNode *params, TypeNode *retTy);

    //conversions
    llvm::Type* typeTagToLlvmType(TypeTag tagTy, llvm::LLVMContext &c, std::string typeName = "");
    TypeTag llvmTypeToTypeTag(llvm::Type *t);
    std::string llvmTypeToStr(llvm::Type *ty);
    std::string typeTagToStr(TypeTag ty);
    bool llvmTypeEq(llvm::Type *l, llvm::Type *r);

    //typevar utility functions
    void validateType(Compiler *c, const TypeNode* tn, const DataDeclNode* rootTy);
    void validateType(Compiler *c, const TypeNode *tn, const DataType *dt);
    TypeNode* extractTypeValue(const TypedValue *tv);
    TypeNode* extractTypeValue(const std::unique_ptr<TypedValue> &tv);
    void bindGenericToType(TypeNode *tn, const std::vector<std::pair<std::string, std::unique_ptr<TypeNode>>> &bindings);
    void bindGenericToType(TypeNode *tn, const std::vector<std::unique_ptr<TypeNode>> &bindings, DataType *dt);

    std::string getCastFnBaseName(TypeNode *t);

    TypeNode* getLargestExt(Compiler *c, TypeNode *tn);
    char getBitWidthOfTypeTag(const TypeTag tagTy);
    bool isPrimitiveTypeTag(TypeTag ty);
    bool isNumericTypeTag(const TypeTag ty);
    bool isIntTypeTag(const TypeTag ty);
    bool isFPTypeTag(const TypeTag tt);
    bool isUnsignedTypeTag(const TypeTag tagTy);

}

#endif
