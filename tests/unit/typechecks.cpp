#include "unittest.h"
#include "types.h"
using namespace ante;
using namespace std;

TEST_CASE("Type Checks", "[typeEq]"){
    auto&& c = Compiler(nullptr);

    auto voidTy = AnType::getVoid();
    auto intTy = AnType::getIsz();
    auto boolTy = AnType::getBool();

    auto voidPtr = AnPtrType::get(voidTy);
    auto intPtr = AnPtrType::get(intTy); 

    auto t = AnTypeVarType::get("'t");
    auto u = AnTypeVarType::get("'u");


    //pointer equality of exactly equal types
    REQUIRE(AnType::getI32() == AnType::getI32());
    
    REQUIRE(voidPtr == AnPtrType::get(AnType::getVoid()));

    //basic equality
    REQUIRE(voidTy == AnPtrType::get(AnType::getVoid()));

    REQUIRE(voidPtr != intPtr);

    /*
    SECTION("('t, bool) == (isz, 'u)"){
        auto tup1 = AnAggregateType::get(TT_Tuple, {t, boolTy});
        auto tup2 = AnAggregateType::get(TT_Tuple, {intTy, u});

        auto tc = typeEq(tup1, tup2);
        auto &bindings = tc->bindings;

        REQUIRE(tup1 != tup2);

        REQUIRE(tc->res == TypeCheckResult::SuccessWithTypeVars);

        REQUIRE(bindings.size() == 2);

        REQUIRE(contains(bindings, TypeBinding("'t", intTy)));

        REQUIRE(contains(bindings, TypeBinding("'u", boolTy)));
    }

    SECTION("Empty isz* == Empty isz*"){
        //Empty 't
        auto empty = AnProductType::create("Empty", {}, false, {string("'t")});

        //Empty isz*
        vector<TypeBinding> bindings {{"'t", empty, 0, intPtr}};
        auto empty_i32Ptr = AnProductType::getVariant(empty, bindings);
        
        auto empty_i32Ptr2 = AnProductType::getVariant(empty, bindings);

        REQUIRE(empty_i32Ptr != empty);

        REQUIRE(empty_i32Ptr == empty_i32Ptr2);

        REQUIRE(typeEq(empty_i32Ptr, empty_i32Ptr2));
    }
     */
}

/*
TEST_CASE("TypeVarType Checks", "[typeEq]"){
    auto&& c = Compiler(nullptr);
    auto t = AnTypeVarType::get("'t");
    auto u = AnTypeVarType::get("'u");

    auto empty = AnProductType::create("Empty", {}, {t});

    //'t -> 't
    auto empty_t = AnDataType::getVariant(empty, {{"'t", empty, 0, t}});

    //'t -> 'u
    auto empty_u = AnDataType::getVariant(empty, {{"'t", empty, 0, u}});

    REQUIRE(empty != nullptr);

    REQUIRE(empty_t != empty);

    REQUIRE(empty_t != empty_u);
    
    REQUIRE(typeEq(empty_t, empty));

    REQUIRE(typeEq(empty_t, empty_u));

    REQUIRE(typeEq(empty, empty_u));

    //both typevars must be bound, solution:
    //solution: bind 't and 'u to new typevar 'Tu
    //REQUIRE(typeEq(empty_t, empty_u)->bindings.size() == 2);

    //When matching an unbound type against a type bound to a
    //type variable, the only binding should be a positional binding
    //of (pos 0) => 'u
    REQUIRE(typeEq(empty, empty_u)->bindings.size() == 1);
}
*/

/*
TEST_CASE("Datatype partial bindings"){
    auto&& compiler = Compiler(nullptr);

    auto ta = AnDataType::create("TypeA", {}, false, {string("'a")});
    auto tb = AnDataType::create("TypeB", {}, false, {string("'b")});

    auto c = AnTypeVarType::get("'c");
    auto tbc = AnDataType::getVariant(tb, {{"'b", tb, 0, c}});

    //TypeA (TypeB 'c)
    //Should have binding (TypeA position 0) -> TypeB 'c, and generic 'c
    auto binding1 = TypeBinding("'a", ta, 0, tbc);
    auto ta_tbc = AnDataType::getVariant(ta, {binding1});
    REQUIRE(ta_tbc->isGeneric);
    REQUIRE(ta_tbc->boundGenerics.size() == 1);
    REQUIRE(ta_tbc->boundGenerics[0] == binding1);
    REQUIRE(ta_tbc->generics.size() == 1);
    REQUIRE(ta_tbc->generics[0].typeVarName == "'c");

    //TypeA TypeB
    //Should have binding (TypeA position 0) -> TypeB, and generic (TypeB position 0)
    auto binding2 = TypeBinding("'a", ta, 0, tb);
    auto ta_tb = AnDataType::getVariant(ta, {binding2});
    REQUIRE(ta_tb->isGeneric);
    REQUIRE(ta_tb->boundGenerics.size() == 1);
    REQUIRE(ta_tb->boundGenerics[0] == binding2);

    //should still have 1 (curried) generic param
    REQUIRE(ta_tb->generics.size() == 1);
}
 */

/*
TEST_CASE("Best Match", "[typeEq]"){
    auto&& c = Compiler(nullptr);
 
    auto i = AnType::getI32();
    auto t = AnTypeVarType::get("'t");

    auto tup1 = AnAggregateType::get(TT_Tuple, {i, i});
    
    auto tup2 = AnAggregateType::get(TT_Tuple, {t, i});
    auto tup3 = AnAggregateType::get(TT_Tuple, {i, t});
    auto tup4 = AnAggregateType::get(TT_Tuple, {t, t});

    auto tc1 = typeEq(tup1, tup1);
    auto tc2 = typeEq(tup1, tup2);
    auto tc3 = typeEq(tup1, tup3);
    auto tc4 = typeEq(tup1, tup4);

    REQUIRE(tc1->res == TypeCheckResult::Success);
    REQUIRE(tc2->res == TypeCheckResult::SuccessWithTypeVars);
    REQUIRE(tc3->res == TypeCheckResult::SuccessWithTypeVars);
    REQUIRE(tc4->res == TypeCheckResult::SuccessWithTypeVars);

    REQUIRE(tc1->matches > tc2->matches);

    REQUIRE(tc2->matches == tc3->matches);
    
    REQUIRE(tc3->matches > tc4->matches);
}
 */
