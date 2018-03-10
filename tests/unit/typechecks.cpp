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

        REQUIRE(contains(bindings, pair<string, AnType*>{"'t", intTy}));

        REQUIRE(contains(bindings, pair<string, AnType*>("'u", boolTy)));
    }

    SECTION("Empty isz* == Empty isz*"){
        //Empty 't
        auto empty = AnDataType::create("Empty", {}, false, {t});

        //Empty isz*
        vector<pair<string,AnType*>> bindings {{"'t", intPtr}};
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


    auto empty = AnDataType::create("Empty", {}, false, {t});

    //'t -> 't
    vector<pair<string,AnType*>> bindings {{"'t", t}};
    auto empty_t = AnDataType::getVariant(&c, empty, bindings);
    
    //'t -> 'u
    vector<pair<string,AnType*>> bindings2 {{"'t", u}};
    auto empty_u = AnDataType::getVariant(&c, empty, bindings2);

    REQUIRE(empty_t != empty_u);
    
    REQUIRE(c.typeEq(empty_t, empty));

    REQUIRE(c.typeEq(empty_t, empty_u).failed());
    
    REQUIRE(c.typeEq(empty, empty_u));

    REQUIRE(c.typeEq(empty, empty_u)->bindings == bindings2);
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
