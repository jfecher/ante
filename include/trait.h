#ifndef AN_TRAIT_H
#define AN_TRAIT_H

#include "funcdecl.h"
#include "antype.h"

namespace ante {
    class AnTypeVarType;

    /** Abstract base class for TraitDecl, TraitImpl */
    struct TraitBase {
        std::string name;
        TypeArgs typeArgs;
        TypeArgs fundeps;

        TraitBase(std::string const& name, TypeArgs const& typeArgs, TypeArgs const& fundeps)
          : name{name}, typeArgs{typeArgs}, fundeps{fundeps}{}
    };

    struct TraitDecl : public TraitBase {
        std::vector<std::shared_ptr<FuncDecl>> funcs;

        TraitDecl(std::string const& name, TypeArgs const& typeArgs, TypeArgs const& fundeps)
          : TraitBase(name, typeArgs, fundeps){}
    };

    /**
     * @brief Holds the name of a trait and the functions needed to implement it
     */
    struct TraitImpl : public TraitBase {
        TraitImpl(TraitDecl *decl, TypeArgs const& args)
                : TraitBase(decl->name, {}, {}), decl{decl} {

            size_t lowerBound = decl->typeArgs.size();
            size_t upperBound = decl->fundeps.size() + lowerBound;
            assert(lowerBound <= args.size() && args.size() <= upperBound);

            size_t i = 0;
            for (; i < lowerBound; i++){
                this->typeArgs.push_back(args[i]);
            }
            for (; i < upperBound; i++){
                this->fundeps.push_back(args[i]);
            }
        }

        TraitImpl(TraitDecl *decl, TypeArgs const& tArgs, TypeArgs const& fundeps)
                : TraitBase(decl->name, tArgs, fundeps), decl{decl} {

            assert(tArgs.size() == decl->typeArgs.size());
            assert(fundeps.size() == decl->fundeps.size());
        }

        /** Pointer to the ExtNode of where this trait instance is
         *  implemented or nullptr if it is not implemented. */
        parser::ExtNode *impl = nullptr;

        TraitDecl *decl = nullptr;

        ~TraitImpl() = default;

        bool implemented() const noexcept {
            return impl;
        }

        const std::string& getName() const noexcept {
            return name;
        }

        bool operator==(TraitImpl const& r){
            return name == r.name && typeArgs == r.typeArgs;
        }

        bool hasTrivialImpl() const;
    };

    TraitImpl* toTrait(parser::TypeNode *tn, Module *m);
    std::string traitToStr(const TraitImpl *trait);
    lazy_str traitToColoredStr(const TraitImpl *trait);
}


#endif /* end of include guard: AN_TRAIT_H */
