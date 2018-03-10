#include "unittest.h"

auto c = new Compiler(nullptr);

TEST_CASE("Size in bits of primitive type", "[getSizeInBits]"){
    auto i8 = AnType::getI8();
    auto i16 = AnType::getI16();
    auto i32 = AnType::getI32();
    auto i64 = AnType::getI64();
    auto isz = AnType::getIsz();

    auto u8 = AnType::getU8();
    auto u16 = AnType::getU16();
    auto u32 = AnType::getU32();
    auto u64 = AnType::getU64();
    auto usz = AnType::getUsz();

    auto c8 = AnType::getPrimitive(TT_C8);
    auto c32 = AnType::getPrimitive(TT_C32);

    auto f16 = AnType::getF16();
    auto f32 = AnType::getF32();
    auto f64 = AnType::getF64();

    auto voidTy = AnType::getVoid();
    auto boolTy = AnType::getBool();

    REQUIRE(i8->getSizeInBits(c).getVal() == 8);
    REQUIRE(i16->getSizeInBits(c).getVal() == 16);
    REQUIRE(i32->getSizeInBits(c).getVal() == 32);
    REQUIRE(i64->getSizeInBits(c).getVal() == 64);
    REQUIRE(isz->getSizeInBits(c).getVal() == 8*sizeof(void*));

    REQUIRE(i8->getSizeInBits(c).getVal() == 8);
    REQUIRE(i16->getSizeInBits(c).getVal() == 16);
    REQUIRE(i32->getSizeInBits(c).getVal() == 32);
    REQUIRE(i64->getSizeInBits(c).getVal() == 64);
    REQUIRE(isz->getSizeInBits(c).getVal() == 8*sizeof(void*));

    REQUIRE(c8->getSizeInBits(c).getVal() == 8);
    REQUIRE(c32->getSizeInBits(c).getVal() == 32);

    REQUIRE(f16->getSizeInBits(c).getVal() == 16);
    REQUIRE(f32->getSizeInBits(c).getVal() == 32);
    REQUIRE(f64->getSizeInBits(c).getVal() == 64);

    REQUIRE(voidTy->getSizeInBits(c).getVal() == 0);
    REQUIRE(boolTy->getSizeInBits(c).getVal() == 8);
}

TEST_CASE("Size in bits of pointer type", "[getSizeInBits]"){
    auto ptrTy1 = AnPtrType::get(AnType::getVoid());
    auto ptrTy2 = AnPtrType::get(AnType::getBool());
    auto ptrTy3 = AnPtrType::get(AnTypeVarType::get("'t"));

    REQUIRE(ptrTy1->getSizeInBits(c).getVal() == 8*sizeof(void*));
    REQUIRE(ptrTy2->getSizeInBits(c).getVal() == 8*sizeof(void*));
    REQUIRE(ptrTy3->getSizeInBits(c).getVal() == 8*sizeof(void*));
}

TEST_CASE("Size in bits of array type", "[getSizeInBits]"){
    auto arrTy1 = AnArrayType::get(AnType::getI16(), 3);
    auto arrTy2 = AnArrayType::get(AnType::getF32(), 0);

    REQUIRE(arrTy1->getSizeInBits(c).getVal() == 3 * 16);
    REQUIRE(arrTy2->getSizeInBits(c).getVal() == 0);
}
    
TEST_CASE("Size in bits of function type", "[getSizeInBits]"){
    auto ptrTy1 = AnPtrType::get(AnType::getVoid());
    auto arrTy1 = AnArrayType::get(AnType::getI16(), 3);

    auto fnTy1 = AnFunctionType::get(AnType::getVoid(), {}, false);
    auto fnTy2 = AnFunctionType::get(AnType::getU16(), {AnType::getI32(), ptrTy1}, false);
    auto fnTy3 = AnFunctionType::get(AnType::getUsz(), {AnType::getUsz(), arrTy1}, true);

    REQUIRE(fnTy1->getSizeInBits(c).getVal() == 8*sizeof(void*));
    REQUIRE(fnTy2->getSizeInBits(c).getVal() == 8*sizeof(void*));
    REQUIRE(fnTy2->getSizeInBits(c).getVal() == 8*sizeof(void*));
}

TEST_CASE("Size in bits of aggregate (tuple) type", "[getSizeInBits]"){
    auto aggTy1 = AnAggregateType::get(TT_Tuple, {});
    auto aggTy2 = AnAggregateType::get(TT_Tuple, {AnType::getI32()});
    auto aggTy3 = AnAggregateType::get(TT_Tuple, {AnType::getU64(), AnType::getBool()});
    auto aggTy4 = AnAggregateType::get(TT_Tuple, {aggTy3, AnPtrType::get(AnType::getVoid())});

    REQUIRE(aggTy1->getSizeInBits(c).getVal() == 0);
    REQUIRE(aggTy2->getSizeInBits(c).getVal() == 32);
    REQUIRE(aggTy3->getSizeInBits(c).getVal() == 64 + 8);
    REQUIRE(aggTy4->getSizeInBits(c).getVal() == 8*sizeof(void*) + aggTy3->getSizeInBits(c).getVal());
}

TEST_CASE("Size in bits of generic type", "[getSizeInBits]"){
    auto t = AnTypeVarType::get("'t");
    auto u = AnTypeVarType::get("'u");
    auto ptr_t = AnPtrType::get(t);
    auto arr_u = AnArrayType::get(u, 5);
    auto tup = AnAggregateType::get(TT_Tuple, {AnType::getI32(), t});
    auto fn = AnFunctionType::get(u, {AnType::getI32(), t}, false);

    //All non-ptr types should return an error instead of a size
    REQUIRE(!t->getSizeInBits(c));
    REQUIRE(!u->getSizeInBits(c));
    REQUIRE(ptr_t->getSizeInBits(c).getVal() == 8*sizeof(void*));
    REQUIRE(!arr_u->getSizeInBits(c));
    REQUIRE(!tup->getSizeInBits(c));
    REQUIRE(fn->getSizeInBits(c).getVal() == 8*sizeof(void*));
}

TEST_CASE("Force size in bits of generic type", "[getSizeInBits]"){
    auto t = AnTypeVarType::get("'t");
    auto u = AnTypeVarType::get("'u");
    auto ptr_t = AnPtrType::get(t);
    auto arr_u = AnArrayType::get(u, 5);
    auto tup = AnAggregateType::get(TT_Tuple, {AnType::getI32(), t});
    auto fn = AnFunctionType::get(u, {AnType::getI32(), t}, false);

    //All typevars are guessed to be a ptr's size when getSizeInBits is forced
    REQUIRE(t->getSizeInBits(c, nullptr, true).getVal() == 8*sizeof(void*));
    REQUIRE(u->getSizeInBits(c, nullptr, true).getVal() == 8*sizeof(void*));
    REQUIRE(ptr_t->getSizeInBits(c, nullptr, true).getVal() == 8*sizeof(void*));
    REQUIRE(arr_u->getSizeInBits(c, nullptr, true).getVal() == 5 * 8*sizeof(void*));
    REQUIRE(tup->getSizeInBits(c, nullptr, true).getVal() == 32 + 8*sizeof(void*));
    REQUIRE(fn->getSizeInBits(c, nullptr, true).getVal() == 8*sizeof(void*));
}
