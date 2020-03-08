#ifndef AN_TYPE_H
#define AN_TYPE_H

#include <string>
#include <vector>
#include <memory>
#include <algorithm>
#include <unordered_map>

#include <llvm/IR/Module.h>
#include <llvm/ADT/StringMap.h>

#include "tokens.h"
#include "parser.h"
#include "result.h"

#define AN_HASH_PRIME 0x9e3779e9

namespace ante {
    struct Compiler;
    struct TypeDecl;
    struct Module;
    struct TraitImpl;

    /**
     * A primitive type
     */
    class AnType {
        // primitive types are uniqued in this container
        static std::vector<AnType> typeContainer;

    protected:
        AnType(TypeTag id, bool ig) :
            typeTag(id), isGeneric(ig){}

    public:
        // Initialize typeContainer with all the primitive types
        static void initTypeSystem();

        virtual ~AnType() = default;

        TypeTag typeTag;
        bool isGeneric;

        // Exact equality
        // Matches typevars to only other typevars of the same name
        bool operator==(AnType const& other) const noexcept;
        bool operator!=(AnType const& other) const noexcept;

        // Approximate equality, for finding matching typeclasses
        // Matches typevars to any type
        bool approxEq(const AnType *other) const noexcept;

        virtual bool hasModifier(TokenType m) const;

        virtual bool isModifierType() const noexcept {
            return false;
        }

        /** Returns a version of the current type with the additional modifier m. */
        virtual const AnType* addModifier(TokenType m) const;

        /** Add all compatible modifiers from the current type to the given and return it. */
        virtual const AnType* addModifiersTo(const AnType* t) const;

        /** Returns the size of this type in bits or an error message if the type is invalid.
         *  @param incompleteType The name of an undeclared type, used to issue an IncompleteTypeError if
         *                        it is found within the type being sized.
         */
        Result<size_t, std::string> getSizeInBits(Compiler *c, std::string const& incompleteType = "") const;

        /** Print the contents of this type to stdout. */
        void dump() const;

        /** Gets a function's return type.
         *  Assumes that this AnType is a AnFuncionType instance. */
        AnType* getFunctionReturnType() const;

        bool isPrimitiveTy() const noexcept;
        bool isSignedTy() const noexcept;
        bool isUnsignedTy() const noexcept;
        bool isFloatTy() const noexcept;
        bool isIntegerTy() const noexcept;
        bool isNumericTy() const noexcept;

        /** Shortcut for casting to an AnTypeVarType and calling AnTypeVarType::isRhoVar */
        bool isRowVar() const;

        static AnType* getPrimitive(TypeTag tag);
        static AnType* getI8();
        static AnType* getI16();
        static AnType* getI32();
        static AnType* getI64();
        static AnType* getIsz();
        static AnType* getU8();
        static AnType* getU16();
        static AnType* getU32();
        static AnType* getU64();
        static AnType* getUsz();
        static AnType* getF16();
        static AnType* getF32();
        static AnType* getF64();
        static AnType* getBool();
        static AnType* getUnit();
    };

    bool isGeneric(const std::vector<AnType*> &vec);
    bool isGeneric(AnType *retTy, std::vector<AnType*> const& params, std::vector<TraitImpl*> const& traits);

    /**
     *  Virtual base class for modifier types.
     *
     *  Not all modifiers are valid types but new type modifiers
     *  may be defined by users.
     */
    class AnModifier : public AnType {
        protected:
        AnModifier(const AnType *modifiedType) :
            AnType(modifiedType->typeTag, modifiedType->isGeneric), extTy(modifiedType){}

        public:
        const AnType *extTy;

        bool isModifierType() const noexcept override {
            return true;
        }

        ~AnModifier() = default;
    };


    template<typename T>
    T* try_cast(AnType *type){
        if(!type || !T::istype(type)){
            return nullptr;
        }

        while(type->isModifierType()){
            auto *mod = static_cast<AnModifier*>(type);
            type = (AnType*)mod->extTy;
        }
        return static_cast<T*>(type);
    }

    template<typename T>
    const T* try_cast(const AnType *type){
        if(!T::istype(type)){
            return nullptr;
        }

        while(type->isModifierType()){
            auto *mod = static_cast<const AnModifier*>(type);
            type = mod->extTy;
        }
        return static_cast<const T*>(type);
    }

    template<typename T>
    T* cast(AnType* type){
        T *t = try_cast<T>(type);
        assert(t);
        return t;
    }

    template<typename T>
    const T* cast(const AnType* type){
        const T *t = try_cast<const T>(type);
        assert(t);
        return t;
    }


    /** Represents a built-in modifier type such as mut */
    class BasicModifier : public AnModifier {
        protected:
        BasicModifier(const AnType *modified_type, TokenType m) :
            AnModifier(modified_type), mod(m){}

        public:
        const TokenType mod;

        static BasicModifier* get(const AnType *modifiedType, TokenType mod);

        bool hasModifier(TokenType m) const override;

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        /** Add all compatible modifiers from the current type to the given and return it. */
        const AnType* addModifiersTo(const AnType* t) const override;

        ~BasicModifier() = default;
    };


    /**
     * A user-defined modifier.
     *
     * Has the chance to contain an invalid compiler-directive
     * that does not operate on a Ante.Type or Ante.TypeDecl.
     */
    class CompilerDirectiveModifier : public AnModifier {
        protected:
        CompilerDirectiveModifier(const AnType *modified_type, parser::Node *d) :
            AnModifier(modified_type), directive(d){}

        public:
        parser::Node *directive;

        static CompilerDirectiveModifier* get(const AnType *modifiedType, parser::Node *directive);
        
        bool hasModifier(TokenType m) const override;

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        /** Add all compatible modifiers from the current type to the given and return it. */
        const AnType* addModifiersTo(const AnType* t) const override;

        ~CompilerDirectiveModifier() = default;
    };


    /** Tuple types */
    class AnTupleType : public AnType {
        protected:
        AnTupleType(std::vector<AnType*> const& fields,
                    std::vector<std::string> const& fieldNames) :
                AnType(TT_Tuple, ante::isGeneric(fields)),
                fields(fields), fieldNames(fieldNames) {}

        AnTupleType(std::vector<AnType*> const& fields,
                    std::vector<std::string> const& fieldNames, bool isGeneric) :
                AnType(TT_Tuple, isGeneric), fields(fields), fieldNames(fieldNames) {}
        public:

        ~AnTupleType() = default;

        /** The constituent types of this tuple/anonymous-record type. */
        std::vector<AnType*> fields;

        /** Field names for each index.
         *  - If this type is an anonymous record type, this vector matches the fields vector index-wise.
         *      - If there is a rho variable in the type, the corresponding field name will be ""
         *  - If this type is a normal tuple type, this vector will be empty
         *  */
        std::vector<std::string> fieldNames;

        /** Get/Create a tuple type with fieldNames = {} */
        static AnTupleType* get(std::vector<AnType*> const& types);

        /** Get/Create an anonymous record  type with non-empty fieldNames */
        static AnTupleType* getAnonRecord(std::vector<AnType*> const& types,
                std::vector<std::string> const& fieldNames);

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        bool isAnonRecordType() const noexcept {
            return !fieldNames.empty();
        }

        bool hasRowVar() const noexcept {
            return !fields.empty() && fields.back()->isRowVar();
        }

        virtual bool isModifierType() const noexcept override {
            return false;
        }

        /** Returns true if this type is a tuple, function, or (a declared) data type */
        static bool istype(const AnType *t){
            return t->typeTag == TT_Tuple;
        }
    };

    /**
     * Arrays types, both sized and unsized.
     *
     * NOTE: Arrays have 2 or 3 contained types.  The array type itself,
     * its element type, and the optional size of the array.
     */
    class AnArrayType : public AnType {
        protected:
        AnArrayType(AnType* ext, size_t l) :
            AnType(TT_Array, ext->isGeneric), extTy(ext), len(l) {}

        public:

        ~AnArrayType() = default;

        /** The element type of this array. */
        AnType *extTy;

        /** Length of the array type.  0 if not specified */
        size_t len;

        static AnArrayType* get(AnType*, size_t len = 0);

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        virtual bool isModifierType() const noexcept override {
            return false;
        }

        static bool istype(const AnType *t){
            return t->typeTag == TT_Array;
        }
    };

    /** Pointer types */
    class AnPtrType : public AnType {
        protected:
        AnPtrType(AnType *elem) :
            AnType(TT_Ptr, elem->isGeneric), elemTy(elem){}

        public:

        ~AnPtrType() = default;

        /** The type being pointed to. */
        AnType *elemTy;

        static AnPtrType* get(AnType* l);

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        virtual bool isModifierType() const noexcept override {
            return false;
        }

        static bool istype(const AnType *t){
            return t->typeTag == TT_Ptr;
        }
    };

    /** A Typevar type.
     *  Typevar types are always generic. */
    class AnTypeVarType : public AnType {
        protected:
        AnTypeVarType(std::string const& n) :
            AnType(TT_TypeVar, true), name(n), isRowVariable(false){}

        public:

        ~AnTypeVarType() = default;

        std::string name;

        bool isRowVariable;

        static AnTypeVarType* get(std::string const& name);

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        virtual bool isModifierType() const noexcept override {
            return false;
        }

        static bool istype(const AnType *t){
            return t->typeTag == TT_TypeVar;
        }
    };

    /** A function type */
    class AnFunctionType : public AnType {
        protected:
        AnFunctionType(AnType *ret, std::vector<AnType*> params,
                std::vector<TraitImpl*> tcConstraints)
                : AnType(TT_Function, ante::isGeneric(ret, params, tcConstraints)),
                  paramTys(params), retTy(ret), typeClassConstraints(tcConstraints){
        }

        public:

        ~AnFunctionType() = default;

        /**
         * Contains the type of each parameter.
         * Note that this is never empty as every function always takes
         * at least unit as a parameter.  These unit values are later
         * optimized away during code generation.
         */
        std::vector<AnType*> paramTys;

        AnType *retTy;

        std::vector<TraitImpl*> typeClassConstraints;

        static AnFunctionType* get(AnType *retTy, std::vector<AnType*> const& elems,
                std::vector<TraitImpl*> const& tcConstraints);

        static AnFunctionType* get(AnType* retty, parser::NamedValNode* params, Module *module);

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        virtual bool isModifierType() const noexcept override {
            return false;
        }

        bool isVarArgs() const noexcept {
            return !paramTys.empty() && paramTys.back()->isRowVar();
        }

        /** Returns true if this type is a TT_Function or TT_MetaFunction */
        static bool istype(const AnType *t){
            return t->typeTag == TT_Function;
        }
    };

    using TypeArgs = std::vector<AnType*>;

    /**
     *  A base class for any user-declared data type.
     *
     *  Corresponds to a single 'type T = ...' instance
     */
    class AnDataType : public AnType {

        protected:
        AnDataType(std::string const& n, TypeArgs const& args, TypeDecl *decl) :
                AnType(TT_Data, false), name(n), unboundType(0), typeArgs(args), decl(decl){}

        public:

        ~AnDataType() = default;

        std::string name;

        /** The unbound parent type of this generic type.
         * If this type is a bound version (eg. Maybe<i32>) of some generic
         * type (eg. say Maybe<'t>), unboundType will point to the generic type.
         * Otherwise, this field will be nullptr. */
        AnDataType *unboundType;

        /** Typevars this type is generic over */
        TypeArgs typeArgs;

        TypeDecl *decl;

        static AnDataType* get(std::string const& name, TypeArgs const& elems, TypeDecl *decl);

        /** Returns true if the given AnType is an AnDataType */
        static bool istype(const AnType *t){
            return t->typeTag == TT_Data;
        }

        /** Returns true if this DataType is a bound generic variant of another */
        bool isVariant() const {
            return unboundType;
        }

        /** Returns true if this type is a bound variant of the generic type dt.
         *  If dt is not a generic type, this function will always return false. */
        bool isVariantOf(const AnDataType *dt) const;

        // Forwards for convenience from this->decl
        llvm::Type* toLlvmType(Compiler *c) const;
        std::vector<AnType*> getBoundFieldTypes() const;
        Result<size_t, std::string> getSizeInBits(Compiler *c, std::string const& incompleteType) const;
        llvm::Value* getTagValue(Compiler *c, std::string const& variantName, std::vector<TypedValue> const& args) const;
    };

    size_t hashCombine(size_t l, size_t r);
    bool allEq(std::vector<AnType*> const& l, std::vector<AnType*> const& r);
    bool allApproxEq(std::vector<AnType*> const& l, std::vector<AnType*> const& r);
}

namespace std {
    template<typename T, typename U>
    struct hash<std::pair<T, U>> {
        size_t operator()(std::pair<T, U> const& t) const {
            return ante::hashCombine(std::hash<T>()(t.first), std::hash<U>()(t.second));
        }
    };

    template<typename T>
    struct hash<std::vector<T>> {
        size_t operator()(std::vector<T> const& v) const {
            size_t ret = v.size();
            for(auto &e : v){
                ret = ante::hashCombine(ret, std::hash<T>()(e));
            }
            return ret;
        }
    };

    template<>
    struct hash<ante::TypeTag> {
        size_t operator()(ante::TypeTag tt) const {
            return tt;
        }
    };

    template<>
    struct hash<ante::TokenType> {
        size_t operator()(ante::TokenType tt) const {
            return tt;
        }
    };
}

#endif
