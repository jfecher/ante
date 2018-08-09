#include "unittest.h"

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
    REQUIRE(c.typeEq(voidTy, voidTy));

    REQUIRE(!c.typeEq(voidPtr, intPtr));

    SECTION("('t, bool) == (isz, 'u)"){
        auto tup1 = AnAggregateType::get(TT_Tuple, {t, boolTy});
        auto tup2 = AnAggregateType::get(TT_Tuple, {intTy, u});

        auto tc = c.typeEq(tup1, tup2);
        auto &bindings = tc->bindings;
        
        REQUIRE(tup1 != tup2);

        REQUIRE(tc->res == TypeCheckResult::SuccessWithTypeVars);

        REQUIRE(bindings.size() == 2);

        REQUIRE(contains(bindings, TypeBinding("'t", intTy)));

        REQUIRE(contains(bindings, TypeBinding("'u", boolTy)));
    }

    SECTION("Empty isz* == Empty isz*"){
        //Empty 't
        auto empty = AnDataType::create("Empty", {}, false, {string("'t")});

        //Empty isz*
        vector<TypeBinding> bindings {{"'t", intPtr}};
        auto empty_i32Ptr = AnDataType::getVariant(&c, empty, bindings);
        
        auto empty_i32Ptr2 = AnDataType::getVariant(&c, empty, bindings);

        REQUIRE(empty_i32Ptr == empty_i32Ptr2);

        REQUIRE(c.typeEq(empty_i32Ptr, empty_i32Ptr2));
    }
}


TEST_CASE("TypeVarType Checks", "[typeEq]"){
    auto&& c = Compiler(nullptr);
    auto t = AnTypeVarType::get("'t");
    auto u = AnTypeVarType::get("'u");


    auto empty = AnDataType::create("Empty", {}, false, {t->name});

    //'t -> 't
    auto empty_t = AnDataType::getVariant(&c, empty, {{"'t", t}});
    
    //'t -> 'u
    auto empty_u = AnDataType::getVariant(&c, empty, {{"'t", u}});

    REQUIRE(empty_t != empty_u);
    
    REQUIRE(c.typeEq(empty_t, empty));

    REQUIRE(c.typeEq(empty_t, empty_u));

    REQUIRE(c.typeEq(empty, empty_u));

    //When matching 't against 'u no bindings are given
    //as it is unclear if 't should be bound to 'u or vice versa
    REQUIRE(c.typeEq(empty, empty_u)->bindings.empty());
}


TEST_CASE("Datatype partial bindings"){
    auto&& compiler = Compiler(nullptr);

    auto ta = AnDataType::create("TypeA", {}, false, {string("'a")});
    auto tb = AnDataType::create("TypeB", {}, false, {string("'b")});

    auto c = AnTypeVarType::get("'c");
    auto tbc = AnDataType::getVariant(&compiler, tb, {{"'b", tb, 0, c}});

    //TypeA (TypeB 'c)
    //Should have binding (TypeA position 0) -> TypeB 'c, and generic 'c
    auto binding1 = TypeBinding("'a", ta, 0, tbc);
    auto ta_tbc = AnDataType::getVariant(&compiler, ta, {binding1});
    REQUIRE(ta_tbc->isGeneric);
    REQUIRE(ta_tbc->boundGenerics.size() == 1);
    REQUIRE(ta_tbc->boundGenerics[0] == binding1);
    REQUIRE(ta_tbc->generics.size() == 1);
    REQUIRE(ta_tbc->generics[0].typeVarName == "'c");

    //TypeA TypeB
    //Should have binding (TypeA position 0) -> TypeB, and generic (TypeB position 0)
    auto binding2 = TypeBinding("'a", ta, 0, tb);
    auto ta_tb = AnDataType::getVariant(&compiler, ta, {binding2});
    REQUIRE(ta_tb->isGeneric);
    REQUIRE(ta_tb->boundGenerics.size() == 1);
    REQUIRE(ta_tb->boundGenerics[0] == binding2);
    //no named generics, positional only (no longer any 'c, only generic was curried)
    REQUIRE(ta_tb->generics.empty());
}


TEST_CASE("Best Match", "[typeEq]"){
    auto&& c = Compiler(nullptr);
 
    auto i = AnType::getI32();
    auto t = AnTypeVarType::get("'t");

    auto tup1 = AnAggregateType::get(TT_Tuple, {i, i});
    
    auto tup2 = AnAggregateType::get(TT_Tuple, {t, i});
    auto tup3 = AnAggregateType::get(TT_Tuple, {i, t});
    auto tup4 = AnAggregateType::get(TT_Tuple, {t, t});

    auto tc1 = c.typeEq(tup1, tup1);
    auto tc2 = c.typeEq(tup1, tup2);
    auto tc3 = c.typeEq(tup1, tup3);
    auto tc4 = c.typeEq(tup1, tup4);

    REQUIRE(tc1->res == TypeCheckResult::Success);
    REQUIRE(tc2->res == TypeCheckResult::SuccessWithTypeVars);
    REQUIRE(tc3->res == TypeCheckResult::SuccessWithTypeVars);
    REQUIRE(tc4->res == TypeCheckResult::SuccessWithTypeVars);

    REQUIRE(tc1->matches > tc2->matches);

    REQUIRE(tc2->matches == tc3->matches);
    
    REQUIRE(tc3->matches > tc4->matches);
}
