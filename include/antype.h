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
    struct UnionTag;
    struct Trait;

    class BasicModifier;
    class AnModifier;
    class AnAggregateType;
    class AnArrayType;
    class AnPtrType;
    class AnTypeVarType;
    class AnDataType;
    class AnFunctionType;
    class AnTypeContainer;

    /** A primitive type
     *
     *  All AnTypes are uniqued and immutable.  Instances are created via
     *  static methods and are uniqued, that is, no two perfectly equal
     *  instances are created.  Any two perfectly equal types are also
     *  equivalent using pointer equality.
     *
     *  Two types may still be equal even if their pointer comparison fails.
     *  This occurs if the underlying types are equal but the modifiers
     *  of the type are not.
     *
     *  No AnType should ever be manually allocated or freed, all construction
     *  and destruction is handled by an external AnTypeContainer.
     */
    class AnType {
        friend AnTypeContainer;

    protected:
        AnType(TypeTag id, bool ig, size_t mt) :
            typeTag(id), isGeneric(ig), numMatchedTys(mt){}

    public:

        virtual ~AnType() = default;

        TypeTag typeTag;
        bool isGeneric;

        /** The number of types contained within this AnType, including itself.
         * This is the number of types "matched" during type checking when this
         * type is equal to another. */
        size_t numMatchedTys;

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
         *                        it is found within the type being sized and not behind a pointer.
         *  @param force Set to true if this type is known to be generic and although its size is technically
         *               unknown, a guess for the size (by replacing unknown typevars with a pointer type)
         *               should be given anyway. */
        Result<size_t, std::string> getSizeInBits(Compiler *c, std::string *incompleteType = nullptr, bool force = false) const;

        /** Print the contents of this type to stdout. */
        void dump() const;

        /** Gets a function's return type.
         *  Assumes that this AnType is a AnFuncionType instance. */
        AnType* getFunctionReturnType() const;

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
        static AnType* getVoid();
        static AnPtrType* getPtr(AnType*);
        static AnArrayType* getArray(AnType*, size_t len = 0);
        static AnTypeVarType* getTypeVar(std::string const& name);
        static AnFunctionType* getFunction(AnType *r, const std::vector<AnType*>);
        static AnAggregateType* getAggregate(TypeTag t, const std::vector<AnType*>);
    };

    class AnProductType;
    bool isGeneric(const std::vector<AnType*> &vec);
    bool isGeneric(const std::vector<AnProductType*> &vec);

    /**
     *  Virtual base class for modifier types.
     *
     *  Not all modifiers are valid types but new type modifiers
     *  may be defined by users.
     */
    class AnModifier : public AnType {
        protected:
        AnModifier(const AnType *modifiedType) :
            AnType(modifiedType->typeTag, modifiedType->isGeneric,
                    modifiedType->numMatchedTys+1), extTy(modifiedType){}

        public:
        const AnType *extTy;

        bool isModifierType() const noexcept override {
            return true;
        }

        ~AnModifier() = default;
    };


    template<typename T>
    T* try_cast(AnType *type){
        if(!T::istype(type)){
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


    size_t getNumMatchedTys(const std::vector<AnType*> &types);

    /** Tuple types */
    class AnAggregateType : public AnType {
        protected:
        AnAggregateType(TypeTag ty, const std::vector<AnType*> exts) :
                AnType(ty, ante::isGeneric(exts), getNumMatchedTys(exts)+1), extTys(exts) {}

        AnAggregateType(TypeTag ty, const std::vector<AnType*> exts, bool isGeneric) :
                AnType(ty, isGeneric, getNumMatchedTys(exts)+1), extTys(exts) {}
        public:

        ~AnAggregateType() = default;

        /** The constituent types of this aggregate type. */
        std::vector<AnType*> extTys;

        static AnAggregateType* get(TypeTag t, std::vector<AnType*> types);

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        virtual bool isModifierType() const noexcept override {
            return false;
        }

        /** Returns true if this type is a tuple, function, or (a declared) data type */
        static bool istype(const AnType *t){
            return t->typeTag == TT_Tuple or t->typeTag == TT_Function
                or t->typeTag == TT_MetaFunction;
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
            AnType(TT_Array, ext->isGeneric, ext->numMatchedTys + (l == 0 ? 1 : 2)), extTy(ext), len(l) {}

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
        AnPtrType(AnType* ext) :
            AnType(TT_Ptr, ext->isGeneric, ext->numMatchedTys + 2), extTy(ext){}

        public:

        ~AnPtrType() = default;

        /** The type being pointed to. */
        AnType *extTy;

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
            AnType(TT_TypeVar, true, 1), name(n){}

        public:

        ~AnTypeVarType() = default;

        std::string name;

        static AnTypeVarType* get(std::string const& name);

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        virtual bool isModifierType() const noexcept override {
            return false;
        }

        bool isVarArgs() const noexcept {
            size_t len = name.size();
            // Using string::find would be more terse but would needlessly check the whole string
            return len > 3 && name[len-3] == '.' && name[len-2] == '.' && name[len-1] == '.';;
        }

        static bool istype(const AnType *t){
            return t->typeTag == TT_TypeVar;
        }
    };

    /** A function type */
    class AnFunctionType : public AnAggregateType {
        protected:
        AnFunctionType(AnType *ret, std::vector<AnType*> elems, bool isMetaFunction) :
                AnAggregateType(isMetaFunction ? TT_MetaFunction : TT_Function, elems,
                        ret->isGeneric || ante::isGeneric(elems)), retTy(ret){

            //numMatchedTys = #params + 1 ret ty + 1 fn ty itself
            numMatchedTys = elems.size() + 2;
        }

        public:

        ~AnFunctionType() = default;

        AnType *retTy;

        static AnFunctionType* get(AnType *retTy, const std::vector<AnType*> elems,
                bool isMetaFunction = false);

        static AnFunctionType* get(AnType* retty, parser::NamedValNode* params,
                bool isMetaFunction = false);

        /** Returns a version of the current type with an additional modifier m. */
        const AnType* addModifier(TokenType m) const override;

        virtual bool isModifierType() const noexcept override {
            return false;
        }

        bool isVarArgs() const noexcept {
            return !extTys.empty() && extTys.back()->typeTag == TT_TypeVar
                && try_cast<AnTypeVarType>(extTys.back())->isVarArgs();
        }

        /** Returns true if this type is a TT_Function or TT_MetaFunction */
        static bool istype(const AnType *t){
            return t->typeTag == TT_Function or t->typeTag == TT_MetaFunction;
        }
    };

    using TypeBindings = std::vector<AnType*>;
    using GenericParams = std::vector<AnTypeVarType*>;

    /**
     *  A base class for any user-declared data type.
     *
     *  Corresponds to a single 'type T = ...' instance
     */
    class AnDataType : public AnType {

        protected:
        AnDataType(std::string const& n, TypeTag tt) :
                AnType(TT_Data, false, 1), name(n), traitImpls(), unboundType(0),
                variants(), boundGenerics(), llvmType(0), isAlias(false){}

        public:

        ~AnDataType() = default;

        std::string name;

        /** The traits this data type implements. */
        std::vector<std::shared_ptr<Trait>> traitImpls;

        /** The unbound parent type of this generic type.
         * If this type is a bound version (eg. Maybe<i32>) of some generic
         * type (eg. say Maybe<'t>), unboundType will point to the generic type.
         * Otherwise, this field will be nullptr. */
        AnDataType *unboundType;

        /**
         *  Bound versions of generic types.
         *
         *  Only parent types (the unbound generic variant matching the type's definition)
         *  have variants.  If an incomplete binding such as Node<Maybe<'u>> is bound
         *  to Node<Maybe<i32>> the resulting type is flattened and stored as a variant
         *  of the parent type Node<'n> so that each parent type has a single vector
         *  of variants rather than a tree structure.
         */
        std::vector<AnDataType*> variants;

        /** Typevars this type is generic over */
        GenericParams generics;

        /**
         * The set of bindings used to bind the parent type to this variant.
         *
         * Empty if this type is not a bound version of some generic type.
         */
        TypeBindings boundGenerics;

        /** The llvm Type corresponding to this data type.
         * May be nullptr if this type has not yet been translated. */
        llvm::Type* llvmType;

        /** True if this type is just an alias for its contents
         *  rather than an entirely new type */
        bool isAlias;

        /** Search for a data type by name.
         * Returns null if no type with a matching name is found. */
        static AnDataType* get(std::string const& name);

        /** Returns true if the given AnType is an AnDataType */
        static bool istype(const AnType *t){
            return t->typeTag == TT_Data || t->typeTag == TT_TaggedUnion;
        }

        /** Returns true if this DataType is a bound generic variant of another */
        bool isVariant() const {
            return unboundType;
        }

        /** Returns true if this type is a bound variant of the generic type dt.
         *  If dt is not a generic type, this function will always return false. */
        bool isVariantOf(const AnDataType *dt) const;

        /** Returns the type this type is aliased to */
        AnType* getAliasedType() const;
    };

    class AnSumType;

    class AnProductType : public AnDataType {

        protected:
        AnProductType(std::string const& n, std::vector<AnType*> const& elems) :
                AnDataType(n, TT_Data), fields(elems), parentUnionType(nullptr) {}

        public:

        ~AnProductType() = default;

        std::vector<AnType*> fields;

        /** Names of each field. */
        std::vector<std::string> fieldNames;

        /** The parent union type of this type if it is a union tag */
        AnSumType *parentUnionType;

        /** Returns true if the given AnType is an AnDataType */
        static bool istype(const AnType *t){
            return t->typeTag == TT_Data;
        }

        /** Returns the given field index or found, or -1 otherwise
        * @param field Name of the field to search for
        * @return The index of the field on success, -1 on failure
        */
        int getFieldIndex(std::string const& field) const {
            for(unsigned int i = 0; i < fields.size(); i++)
                if(field == fieldNames[i])
                    return i;
            return -1;
        }

        /** Returns a new AnDataType* with the given modifier appended to the current type's modifiers. */
        const AnType* addModifier(TokenType m) const override;

        bool isModifierType() const noexcept override {
            return false;
        }

        /** Search for a data type by name.
         * Returns null if no type with a matching name is found. */
        static AnProductType* get(std::string const& name);

        /** Creates or overwrites the type specified by name. */
        static AnProductType* create(std::string const& name, std::vector<AnType*> const& elems,
                GenericParams const& generics);
    };


    class AnSumType : public AnDataType {

        protected:
        AnSumType(std::string const& n, std::vector<AnProductType*> const& elems) :
                AnDataType(n, TT_TaggedUnion), tags(elems){}

        public:

        ~AnSumType() = default;

        /** Contains the UnionTag of each of the union's variants. */
        std::vector<AnProductType*> tags;

        /** Returns true if the given AnType is an AnDataType */
        static bool istype(const AnType *t){
            return t->typeTag == TT_TaggedUnion;
        }

        /**
        * Returns the UnionTag of a tag within the union type.
        *
        * If the given tag is not found, this function issues an
        * error message and throws a CtError exception.
        *
        * @return the value of the tag found, or 0 on failure
        */
        size_t getTagVal(std::string const& name);

        /** Search for a data type by name.
         * Returns null if no type with a matching name is found. */
        static AnSumType* get(std::string const& name);

        /** Creates or overwrites the type specified by name. */
        static AnSumType* create(std::string const& name, std::vector<AnProductType*> const& elems,
                GenericParams const& generics);

        /** Returns a new AnDataType* with the given modifier appended to the current type's modifiers. */
        const AnType* addModifier(TokenType m) const override;

        bool isModifierType() const noexcept override {
            return false;
        }
    };

    size_t hashCombine(size_t l, size_t r);
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
}

namespace ante {
    /**
     *  An owning container for all AnTypes
     *
     *  Note that this class is a singleton, creating new instances
     *  of this class would be meaningless as the AnTypeContainer
     *  referenced by each AnType is unable to be swapped out.
     */
    class AnTypeContainer {
        friend AnType;
        friend BasicModifier;
        friend CompilerDirectiveModifier;
        friend AnAggregateType;
        friend AnArrayType;
        friend AnPtrType;
        friend AnTypeVarType;
        friend AnFunctionType;
        friend AnDataType;
        friend AnProductType;
        friend AnSumType;

        using FnTypeKey = std::pair<AnType*, std::pair<std::vector<AnType*>, bool>>;
        using AggTypeKey = std::pair<TypeTag, std::vector<AnType*>>;
        using VariantTypeKey = std::pair<std::string, std::vector<AnType*>>;

        std::unordered_map<TypeTag, std::unique_ptr<AnType>> primitiveTypes;
        std::unordered_map<std::pair<AnType*, TokenType>, std::unique_ptr<AnModifier>> basicModifiers;
        std::unordered_map<std::pair<AnType*, size_t>, std::unique_ptr<AnModifier>> cdModifiers;
        std::unordered_map<AnType*, std::unique_ptr<AnPtrType>> ptrTypes;
        std::unordered_map<std::pair<AnType*, size_t>, std::unique_ptr<AnArrayType>> arrayTypes;
        std::unordered_map<std::string, std::unique_ptr<AnTypeVarType>> typeVarTypes;
        std::unordered_map<AggTypeKey, std::unique_ptr<AnAggregateType>> aggregateTypes;
        std::unordered_map<FnTypeKey, std::unique_ptr<AnFunctionType>> functionTypes;
        std::unordered_map<std::string, std::unique_ptr<AnDataType>> dataTypes;

        /** generic variants are retrieved through their parent type,
         * never directly through the map of declaredTypes.  Keeping
         * all variants here avoids having to sift through every variant
         * of a type and makes ownership simpler. */
        std::unordered_map<VariantTypeKey, std::unique_ptr<AnDataType>> genericVariants;

    public:
        AnTypeContainer();
        ~AnTypeContainer() = default;

        void clearDeclaredTypes(){
            dataTypes.clear();
        }
    };
}

#endif
