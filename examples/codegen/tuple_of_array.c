#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

void* malloc(size_t);
void* memcpy(void*, void*, size_t);
double fmod(double, double);
typedef struct {} Unit;

typedef struct { void* _0; void* _1; uint32_t _2; uint32_t _3; } Tuple0;
typedef struct { Unit (*_0)(Tuple0, void*); void* _1; } Tuple1;
typedef struct { Tuple1 _0; } Tuple2;
typedef struct { } Tuple3;
typedef struct { size_t (*_0)(size_t, size_t, void*); void* _1; } Tuple4;
typedef struct { size_t (*_0)(uint32_t, void*); void* _1; } Tuple5;
typedef struct { Tuple5 _0; } Tuple6;
typedef struct { int32_t (*_0)(void*, void*); void* _1; } Tuple7;
typedef struct { Tuple7 _0; } Tuple8;
typedef struct { int32_t _0[3]; int32_t _1; } Tuple9;
typedef struct { uint64_t (*_0)(char, void*); void* _1; } Tuple10;
typedef struct { Tuple10 _0; } Tuple11;
typedef struct { Unit (*_0)(void*, void*); void* _1; } Tuple12;
typedef struct { Tuple12 _0; } Tuple13;
typedef struct { Unit (*_0)(int32_t, void*); void* _1; } Tuple14;
typedef struct { Tuple14 _0; } Tuple15;
typedef struct { bool (*_0)(uint64_t, uint64_t, void*); void* _1; } Tuple16;
typedef struct { Tuple16 _0; } Tuple17;
typedef struct { uint64_t (*_0)(uint64_t, uint64_t, void*); void* _1; } Tuple18;
typedef struct { Tuple18 _0; } Tuple19;
typedef struct { bool (*_0)(int64_t, int64_t, void*); void* _1; } Tuple20;
typedef struct { Tuple20 _0; } Tuple21;
typedef struct { uint64_t _0; } Tuple22;
typedef struct { uint8_t _0; Tuple22 _1; } Tuple23;
typedef struct { char (*_0)(uint64_t, void*); void* _1; } Tuple24;
typedef struct { Tuple0 _0; } Tuple25;
typedef struct { uint32_t (*_0)(uint32_t, uint32_t, void*); void* _1; } Tuple26;
typedef struct { Tuple26 _0; } Tuple27;
typedef struct { Unit (*_0)(uint32_t, Tuple25); Tuple25 _1; } Tuple28;
typedef struct { bool (*_0)(bool, void*); void* _1; } Tuple29;
typedef struct { int64_t (*_0)(int64_t, int64_t, void*); void* _1; } Tuple30;
typedef struct { Tuple30 _0; } Tuple31;
typedef struct { Tuple4 _0; } Tuple32;
typedef struct { bool (*_0)(uint32_t, uint32_t, void*); void* _1; } Tuple33;
typedef struct { Tuple33 _0; } Tuple34;
typedef struct { bool (*_0)(size_t, size_t, void*); void* _1; } Tuple35;
typedef struct { Tuple35 _0; } Tuple36;
typedef struct { Tuple15 _0; Tuple8 _1; } Tuple37;
typedef struct { Tuple24 _0; } Tuple38;
typedef struct { Tuple23 (*_0)(int64_t, void*); void* _1; } Tuple39;
typedef struct { Tuple39 _0; } Tuple40;
typedef struct { Tuple29 _0; } Tuple41;
typedef struct { int64_t (*_0)(int32_t, void*); void* _1; } Tuple42;
typedef struct { Tuple42 _0; } Tuple43;

void* array_get_unchecked_1104(void* b0_0, size_t b0_1);
void* transmute_1105(void* b0_0);
void* offset_1107(void* b0_0, size_t b0_1);
uint64_t lambda_483(char b0_0, void* b0_1);
int64_t lambda_532(int64_t b0_0, int64_t b0_1, void* b0_2);
size_t transmute_1108(void* b0_0);
Unit putchar(char);
size_t lambda_317(uint32_t b0_0, void* b0_1);
size_t size_of_1109(Tuple3 b0_0);
bool lambda_654(int64_t b0_0, int64_t b0_1, void* b0_2);
int32_t ability_field_wrapper_726(void* b0_0, void* b0_1);
bool lambda_630(size_t b0_0, size_t b0_1, void* b0_2);
void* transmute_1111(size_t b0_0);
Unit exit(int32_t);
Unit deref_ptr_1112(void* b0_0);
Unit print_unsigned_800(uint64_t b0_0);
void* null_1113(Unit b0_0);
uint64_t lambda_610(uint64_t b0_0, uint64_t b0_1, void* b0_2);
uint64_t lambda_586(uint64_t b0_0, uint64_t b0_1, void* b0_2);
Unit deref_1114(void* b0_0);
size_t size_of_1115(Tuple3 b0_0);
size_t lambda_564(size_t b0_0, size_t b0_1, void* b0_2);
Unit main_1092(Unit b0_0);
size_t lambda_588(size_t b0_0, size_t b0_1, void* b0_2);
Tuple23 lambda_685(int64_t b0_0, void* b0_1);
Unit lambda_829(Tuple0 b0_0, void* b0_1);
bool ___1093(uint64_t b0_0, uint64_t b0_1, Tuple17 b0_2);
Unit array_set_unchecked_1117(void* b0_0, size_t b0_1, int32_t b0_2);
Unit recur_830(uint32_t b0_0, Tuple25 b0_1);
Tuple9 __1094(int32_t b0_0[3], int32_t b0_1);
Unit ptr_store_1118(void* b0_0, int32_t b0_1);
uint32_t lambda_518(uint32_t b0_0, uint32_t b0_1, void* b0_2);
bool lambda_662(uint32_t b0_0, uint32_t b0_1, void* b0_2);
uint64_t unwrap_1119(Tuple23 b0_0);
uint64_t lambda_519(uint64_t b0_0, uint64_t b0_1, void* b0_2);
bool lambda_663(uint64_t b0_0, uint64_t b0_1, void* b0_2);
Unit array_set_1095(void* b0_0, size_t b0_1, int32_t b0_2);
void* offset_bytes_1120(void* b0_0, size_t b0_1);
size_t lambda_520(size_t b0_0, size_t b0_1, void* b0_2);
bool lambda_664(size_t b0_0, size_t b0_1, void* b0_2);
Unit println_1096(void* b0_0, Tuple13 b0_1);
char lambda_472(uint64_t b0_0, void* b0_1);
void* array_get_1097(void* b0_0, size_t b0_1);
char deref_ptr_1121(void* b0_0);
Tuple13 print_ref_1098(Tuple15 b0_0, Tuple8 b0_1);
char deref_1122(void* b0_0);
int64_t lambda_354(int32_t b0_0, void* b0_1);
Unit print_signed_811(int64_t b0_0, int64_t b0_1, Tuple0 b0_2);
Unit println_1099(int32_t b0_0, Tuple15 b0_1);
int32_t deref_1123(void* b0_0);
bool lambda_619(int64_t b0_0, int64_t b0_1, void* b0_2);
Unit lambda_1100(void* b0_0, void* b0_1);
bool ___1101(size_t b0_0, size_t b0_1, Tuple36 b0_2);
Tuple23 Some_1125(uint64_t b0_0);
bool lambda_645(bool b0_0, void* b0_1);
uint64_t transmute_1126(int64_t b0_0);
size_t array_len_1102(void* b0_0);
Unit lambda_815(int32_t b0_0, void* b0_1);
Unit panic_1103(Tuple0 b0_0, Tuple2 b0_1);

extern void* (*ptr_to_mut_1106)(void*);
extern Tuple2 print_string_98;
extern Tuple6 cast_u32_usz_316;
extern Unit (*putchar_5)(char);
extern Tuple8 copy_i32_725;
extern Tuple11 cast_char_u64_150;
extern Tuple3 MkType_1110;
extern Unit (*exit_7)(int32_t);
extern Tuple17 cmp_u64_488;
extern Tuple19 add_u64_152;
extern Tuple27 add_u32_489;
extern Tuple19 div_u64_154;
extern Tuple32 div_usz_587;
extern Tuple3 MkType_1116;
extern Tuple21 cmp_i64_492;
extern Tuple31 sub_i64_493;
extern Tuple34 cmp_u32_661;
extern Tuple36 cmp_usz_182;
extern Tuple38 cast_u64_char_471;
extern Tuple40 try_cast_i64_u64_495;
extern Tuple43 cast_i32_i64_353;
extern Tuple21 eq_i64_618;
extern Tuple19 mod_u64_498;
extern Tuple23 None_1124;
extern Tuple32 add_usz_189;
extern Tuple41 not_bool_94;
extern Tuple15 print_i32_814;
extern Tuple32 mul_usz_190;
extern Tuple36 eq_usz_167;

void* (*ptr_to_mut_1106)(void*) = transmute_1105;
Tuple2 print_string_98 = {{lambda_829, (void*){0}}};
Tuple6 cast_u32_usz_316 = {{lambda_317, (void*){0}}};
Unit (*putchar_5)(char) = putchar;
Tuple8 copy_i32_725 = {{ability_field_wrapper_726, (void*){0}}};
Tuple11 cast_char_u64_150 = {{lambda_483, (void*){0}}};
Tuple3 MkType_1110 = {};
Unit (*exit_7)(int32_t) = exit;
Tuple17 cmp_u64_488 = {{lambda_663, (void*){0}}};
Tuple19 add_u64_152 = {{lambda_519, (void*){0}}};
Tuple27 add_u32_489 = {{lambda_518, (void*){0}}};
Tuple19 div_u64_154 = {{lambda_586, (void*){0}}};
Tuple32 div_usz_587 = {{lambda_588, (void*){0}}};
Tuple3 MkType_1116 = {};
Tuple21 cmp_i64_492 = {{lambda_654, (void*){0}}};
Tuple31 sub_i64_493 = {{lambda_532, (void*){0}}};
Tuple34 cmp_u32_661 = {{lambda_662, (void*){0}}};
Tuple36 cmp_usz_182 = {{lambda_664, (void*){0}}};
Tuple38 cast_u64_char_471 = {{lambda_472, (void*){0}}};
Tuple40 try_cast_i64_u64_495 = {{lambda_685, (void*){0}}};
Tuple43 cast_i32_i64_353 = {{lambda_354, (void*){0}}};
Tuple21 eq_i64_618 = {{lambda_619, (void*){0}}};
Tuple19 mod_u64_498 = {{lambda_610, (void*){0}}};
Tuple23 None_1124 = {(uint8_t)0, (Tuple22){0}};
Tuple32 add_usz_189 = {{lambda_520, (void*){0}}};
Tuple41 not_bool_94 = {{lambda_645, (void*){0}}};
Tuple15 print_i32_814 = {{lambda_815, (void*){0}}};
Tuple32 mul_usz_190 = {{lambda_564, (void*){0}}};
Tuple36 eq_usz_167 = {{lambda_630, (void*){0}}};

void* array_get_unchecked_1104(void* b0_0, size_t b0_1) {b0:;void* (*v0)(void*) = transmute_1105;void* v1 = v0(b0_0);void* (*v2)(void*) = ptr_to_mut_1106;void* (*v3)(void*, size_t) = offset_1107;void* v4 = v3(v1, b0_1);void* v5 = v2(v4);return v5;}
void* transmute_1105(void* b0_0) {b0:;void* v0_src = b0_0; void* v0; memcpy(&v0, &v0_src, sizeof(void*));return v0;}
void* offset_1107(void* b0_0, size_t b0_1) {b0:;size_t (*v0)(void*) = transmute_1108;size_t v1 = v0(b0_0);size_t (*v2)(Tuple3) = size_of_1109;Tuple3 v3 = MkType_1110;size_t v4 = v2(v3);Tuple4 v5 = mul_usz_190._0;size_t (*v11)(size_t, size_t, void*) = v5._0;void* v12 = v5._1;size_t v6 = v11(b0_1, v4, v12);Tuple4 v7 = add_usz_189._0;size_t (*v13)(size_t, size_t, void*) = v7._0;void* v14 = v7._1;size_t v8 = v13(v1, v6, v14);void* (*v9)(size_t) = transmute_1111;void* v10 = v9(v8);return v10;}
uint64_t lambda_483(char b0_0, void* b0_1) {b0:;uint64_t v0 = (uint8_t)b0_0;return v0;}
int64_t lambda_532(int64_t b0_0, int64_t b0_1, void* b0_2) {b0:;int64_t v0 = b0_0 - b0_1;return v0;}
size_t transmute_1108(void* b0_0) {b0:;void* v0_src = b0_0; size_t v0; memcpy(&v0, &v0_src, sizeof(size_t));return v0;}
size_t lambda_317(uint32_t b0_0, void* b0_1) {b0:;size_t v0 = (uint32_t)b0_0;return v0;}
size_t size_of_1109(Tuple3 b0_0) {b0:;size_t v0 = (size_t)4ull;return v0;}
bool lambda_654(int64_t b0_0, int64_t b0_1, void* b0_2) {b0:;bool v0 = (int64_t)b0_0 < (int64_t)b0_1;return v0;}
int32_t ability_field_wrapper_726(void* b0_0, void* b0_1) {b0:;int32_t (*v0)(void*) = deref_1123;int32_t v1 = v0(b0_0);return v1;}
bool lambda_630(size_t b0_0, size_t b0_1, void* b0_2) {b0:;bool v0 = b0_0 == b0_1;return v0;}
void* transmute_1111(size_t b0_0) {b0:;size_t v0_src = b0_0; void* v0; memcpy(&v0, &v0_src, sizeof(void*));return v0;}
Unit deref_ptr_1112(void* b0_0) {b0:;Unit (*v0)(void*) = deref_1114;void* (*v1)(void*) = transmute_1105;void* v2 = v1(b0_0);Unit v3 = v0(v2);return v3;}
Unit print_unsigned_800(uint64_t b0_0) {b0:;bool (*v0)(uint64_t, uint64_t, Tuple17) = ___1093;bool v1 = v0(b0_0, (uint64_t)10ull, cmp_u64_488);if (v1) { goto b1; } else { goto b2; }b1:;Tuple18 v2 = div_u64_154._0;uint64_t (*v14)(uint64_t, uint64_t, void*) = v2._0;void* v15 = v2._1;uint64_t v3 = v14(b0_0, (uint64_t)10ull, v15);Unit v4 = print_unsigned_800(v3);goto b2;b2:;Tuple18 v5 = mod_u64_498._0;uint64_t (*v16)(uint64_t, uint64_t, void*) = v5._0;void* v17 = v5._1;uint64_t v6 = v16(b0_0, (uint64_t)10ull, v17);Tuple10 v7 = cast_char_u64_150._0;uint64_t (*v18)(char, void*) = v7._0;void* v19 = v7._1;uint64_t v8 = v18('0', v19);Tuple18 v9 = add_u64_152._0;uint64_t (*v20)(uint64_t, uint64_t, void*) = v9._0;void* v21 = v9._1;uint64_t v10 = v20(v6, v8, v21);Tuple24 v11 = cast_u64_char_471._0;char (*v22)(uint64_t, void*) = v11._0;void* v23 = v11._1;char v12 = v22(v10, v23);Unit v13 = putchar_5(v12);return v13;}
void* null_1113(Unit b0_0) {b0:;void* (*v0)(size_t) = transmute_1111;void* v1 = v0((size_t)0ull);return v1;}
uint64_t lambda_610(uint64_t b0_0, uint64_t b0_1, void* b0_2) {b0:;uint64_t v0 = (uint64_t)b0_0 % (uint64_t)b0_1;return v0;}
uint64_t lambda_586(uint64_t b0_0, uint64_t b0_1, void* b0_2) {b0:;uint64_t v0 = (uint64_t)b0_0 / (uint64_t)b0_1;return v0;}
Unit deref_1114(void* b0_0) {b0:;Unit v0 = *(Unit*)b0_0;return v0;}
size_t size_of_1115(Tuple3 b0_0) {b0:;size_t v0 = (size_t)12ull;return v0;}
size_t lambda_564(size_t b0_0, size_t b0_1, void* b0_2) {b0:;size_t v0 = b0_0 * b0_1;return v0;}
Unit main_1092(Unit b0_0) {b0:;Tuple9 (*v0)(int32_t [3], int32_t) = __1094;int32_t v1[3] = {(int32_t)10, (int32_t)20, (int32_t)30};Tuple9 v2 = v0(v1, (int32_t)99);int32_t v3[3]; memcpy(v3, v2._0, sizeof(v3));int32_t v4_slot[3]; memcpy(v4_slot, v3, sizeof(v4_slot)); void* v4 = &v4_slot;Unit (*v5)(void*, size_t, int32_t) = array_set_1095;Unit v6 = v5(v4, (size_t)0ull, (int32_t)999);Unit (*v7)(void*, Tuple13) = println_1096;void* (*v8)(void*, size_t) = array_get_1097;void* v9 = v8(v4, (size_t)0ull);Tuple13 (*v10)(Tuple15, Tuple8) = print_ref_1098;Tuple13 v11 = v10(print_i32_814, copy_i32_725);Unit v12 = v7(v9, v11);Unit (*v13)(void*, Tuple13) = println_1096;void* (*v14)(void*, size_t) = array_get_1097;void* v15 = v14(v4, (size_t)2ull);Tuple13 (*v16)(Tuple15, Tuple8) = print_ref_1098;Tuple13 v17 = v16(print_i32_814, copy_i32_725);Unit v18 = v13(v15, v17);Unit (*v19)(int32_t, Tuple15) = println_1099;int32_t v20 = v2._1;Unit v21 = v19(v20, print_i32_814);return v21;}
size_t lambda_588(size_t b0_0, size_t b0_1, void* b0_2) {b0:;size_t v0 = (size_t)b0_0 / (size_t)b0_1;return v0;}
Tuple23 lambda_685(int64_t b0_0, void* b0_1) {Tuple23 b3_0; b0:;Tuple20 v0 = cmp_i64_492._0;bool (*v7)(int64_t, int64_t, void*) = v0._0;void* v8 = v0._1;bool v1 = v7(b0_0, (int64_t)0ll, v8);if (v1) { goto b1; } else { goto b2; }b1:;Tuple23 v2 = None_1124;b3_0 = v2; goto b3;b2:;Tuple23 (*v3)(uint64_t) = Some_1125;uint64_t (*v4)(int64_t) = transmute_1126;uint64_t v5 = v4(b0_0);Tuple23 v6 = v3(v5);b3_0 = v6; goto b3;b3:;return b3_0;}
Unit lambda_829(Tuple0 b0_0, void* b0_1) {b0:;Tuple25 v0 = {b0_0};Tuple28 v1 = {recur_830, v0};Unit (*v3)(uint32_t, Tuple25) = v1._0;Tuple25 v4 = v1._1;Unit v2 = v3((uint32_t)0, v4);return v2;}
bool ___1093(uint64_t b0_0, uint64_t b0_1, Tuple17 b0_2) {b0:;Tuple16 v0 = b0_2._0;bool (*v4)(uint64_t, uint64_t, void*) = v0._0;void* v5 = v0._1;bool v1 = v4(b0_0, b0_1, v5);Tuple29 v2 = not_bool_94._0;bool (*v6)(bool, void*) = v2._0;void* v7 = v2._1;bool v3 = v6(v1, v7);return v3;}
Unit array_set_unchecked_1117(void* b0_0, size_t b0_1, int32_t b0_2) {b0:;void* (*v0)(void*) = transmute_1105;void* v1 = v0(b0_0);Unit (*v2)(void*, int32_t) = ptr_store_1118;void* (*v3)(void*, size_t) = offset_1107;void* v4 = v3(v1, b0_1);Unit v5 = v2(v4, b0_2);return v5;}
Unit recur_830(uint32_t b0_0, Tuple25 b0_1) {b0:;Tuple0 v0 = b0_1._0;Tuple28 v1 = {recur_830, b0_1};uint32_t v2 = v0._2;Tuple33 v3 = cmp_u32_661._0;bool (*v16)(uint32_t, uint32_t, void*) = v3._0;void* v17 = v3._1;bool v4 = v16(b0_0, v2, v17);if (v4) { goto b1; } else { goto b2; }b1:;void* (*v5)(void*, size_t) = offset_bytes_1120;void* v6 = v0._0;Tuple5 v7 = cast_u32_usz_316._0;size_t (*v18)(uint32_t, void*) = v7._0;void* v19 = v7._1;size_t v8 = v18(b0_0, v19);void* v9 = v5(v6, v8);char (*v10)(void*) = deref_ptr_1121;char v11 = v10(v9);Unit v12 = putchar_5(v11);Tuple26 v13 = add_u32_489._0;uint32_t (*v20)(uint32_t, uint32_t, void*) = v13._0;void* v21 = v13._1;uint32_t v14 = v20(b0_0, (uint32_t)1, v21);Unit (*v22)(uint32_t, Tuple25) = v1._0;Tuple25 v23 = v1._1;Unit v15 = v22(v14, v23);goto b2;b2:;return (Unit){};}
Tuple9 __1094(int32_t b0_0[3], int32_t b0_1) {b0:;Tuple9 v0; memcpy(v0._0, b0_0, sizeof(v0._0)); v0._1 = b0_1;return v0;}
Unit ptr_store_1118(void* b0_0, int32_t b0_1) {b0:;void* (*v0)(void*) = transmute_1105;void* v1 = v0(b0_0);*(int32_t*)v1 = b0_1; Unit v2 = (Unit){};return (Unit){};}
uint32_t lambda_518(uint32_t b0_0, uint32_t b0_1, void* b0_2) {b0:;uint32_t v0 = b0_0 + b0_1;return v0;}
bool lambda_662(uint32_t b0_0, uint32_t b0_1, void* b0_2) {b0:;bool v0 = (uint32_t)b0_0 < (uint32_t)b0_1;return v0;}
uint64_t unwrap_1119(Tuple23 b0_0) {uint64_t b3_0; b0:;uint8_t v0 = b0_0._0;switch (v0) { case 0: { goto b1; } case 1: { goto b2; } default: { goto b4; } }b1:;Unit (*v1)(Tuple0, Tuple2) = panic_1103;static uint8_t v2_bytes[] = {84,114,105,101,100,32,116,111,32,117,110,119,114,97,112,32,97,32,78,111,110,101,32,118,97,108,117,101,0}; void* v2 = v2_bytes;size_t v3_src = (size_t)0ull; void* v3; memcpy(&v3, &v3_src, sizeof(void*));Tuple0 v4 = {v2, v3, (uint32_t)28, (uint32_t)0};Unit v5 = v1(v4, print_string_98);__builtin_unreachable();b2:;Tuple22 v6 = b0_0._1;Tuple22 v7_src = v6; Tuple22 v7; memcpy(&v7, &v7_src, sizeof(Tuple22));uint64_t v8 = v7._0;b3_0 = v8; goto b3;b4:;__builtin_unreachable();b3:;return b3_0;}
uint64_t lambda_519(uint64_t b0_0, uint64_t b0_1, void* b0_2) {b0:;uint64_t v0 = b0_0 + b0_1;return v0;}
bool lambda_663(uint64_t b0_0, uint64_t b0_1, void* b0_2) {b0:;bool v0 = (uint64_t)b0_0 < (uint64_t)b0_1;return v0;}
Unit array_set_1095(void* b0_0, size_t b0_1, int32_t b0_2) {b0:;bool (*v0)(size_t, size_t, Tuple36) = ___1101;size_t (*v1)(void*) = array_len_1102;size_t v2 = v1(b0_0);bool v3 = v0(b0_1, v2, cmp_usz_182);if (v3) { goto b1; } else { goto b2; }b1:;Unit (*v4)(Tuple0, Tuple2) = panic_1103;static uint8_t v5_bytes[] = {97,114,114,97,121,95,115,101,116,32,105,110,100,101,120,32,111,117,116,32,111,102,32,98,111,117,110,100,115,0}; void* v5 = v5_bytes;size_t v6_src = (size_t)0ull; void* v6; memcpy(&v6, &v6_src, sizeof(void*));Tuple0 v7 = {v5, v6, (uint32_t)29, (uint32_t)0};Unit v8 = v4(v7, print_string_98);__builtin_unreachable();b2:;Unit (*v9)(void*, size_t, int32_t) = array_set_unchecked_1117;Unit v10 = v9(b0_0, b0_1, b0_2);return v10;}
void* offset_bytes_1120(void* b0_0, size_t b0_1) {b0:;size_t (*v0)(void*) = transmute_1108;size_t v1 = v0(b0_0);void* (*v2)(size_t) = transmute_1111;Tuple4 v3 = add_usz_189._0;size_t (*v6)(size_t, size_t, void*) = v3._0;void* v7 = v3._1;size_t v4 = v6(v1, b0_1, v7);void* v5 = v2(v4);return v5;}
size_t lambda_520(size_t b0_0, size_t b0_1, void* b0_2) {b0:;size_t v0 = b0_0 + b0_1;return v0;}
bool lambda_664(size_t b0_0, size_t b0_1, void* b0_2) {b0:;bool v0 = (size_t)b0_0 < (size_t)b0_1;return v0;}
Unit println_1096(void* b0_0, Tuple13 b0_1) {b0:;Tuple12 v0 = b0_1._0;Unit (*v3)(void*, void*) = v0._0;void* v4 = v0._1;Unit v1 = v3(b0_0, v4);Unit v2 = putchar_5((char)10);return v2;}
char lambda_472(uint64_t b0_0, void* b0_1) {b0:;char v0 = (char)b0_0;return v0;}
void* array_get_1097(void* b0_0, size_t b0_1) {b0:;bool (*v0)(size_t, size_t, Tuple36) = ___1101;size_t (*v1)(void*) = array_len_1102;size_t v2 = v1(b0_0);bool v3 = v0(b0_1, v2, cmp_usz_182);if (v3) { goto b1; } else { goto b2; }b1:;Unit (*v4)(Tuple0, Tuple2) = panic_1103;static uint8_t v5_bytes[] = {97,114,114,97,121,95,103,101,116,32,105,110,100,101,120,32,111,117,116,32,111,102,32,98,111,117,110,100,115,0}; void* v5 = v5_bytes;size_t v6_src = (size_t)0ull; void* v6; memcpy(&v6, &v6_src, sizeof(void*));Tuple0 v7 = {v5, v6, (uint32_t)29, (uint32_t)0};Unit v8 = v4(v7, print_string_98);__builtin_unreachable();b2:;void* (*v9)(void*, size_t) = array_get_unchecked_1104;void* v10 = v9(b0_0, b0_1);return v10;}
char deref_ptr_1121(void* b0_0) {b0:;char (*v0)(void*) = deref_1122;void* (*v1)(void*) = transmute_1105;void* v2 = v1(b0_0);char v3 = v0(v2);return v3;}
Tuple13 print_ref_1098(Tuple15 b0_0, Tuple8 b0_1) {b0:;Unit (*v0)(void*, void*) = lambda_1100;Tuple37 v1 = {b0_0, b0_1};void* v2 = malloc(sizeof(Tuple37)); *(Tuple37*)v2 = v1;Tuple12 v3 = {v0, v2};Tuple13 v4 = {v3};return v4;}
char deref_1122(void* b0_0) {b0:;char v0 = *(char*)b0_0;return v0;}
int64_t lambda_354(int32_t b0_0, void* b0_1) {b0:;int64_t v0 = (int32_t)b0_0;return v0;}
Unit print_signed_811(int64_t b0_0, int64_t b0_1, Tuple0 b0_2) {Unit b3_0; Unit b6_0; b0:;Tuple20 v0 = cmp_i64_492._0;bool (*v19)(int64_t, int64_t, void*) = v0._0;void* v20 = v0._1;bool v1 = v19(b0_0, (int64_t)0ll, v20);if (v1) { goto b1; } else { goto b2; }b1:;Tuple20 v2 = eq_i64_618._0;bool (*v21)(int64_t, int64_t, void*) = v2._0;void* v22 = v2._1;bool v3 = v21(b0_0, b0_1, v22);if (v3) { goto b4; } else { goto b5; }b4:;Tuple1 v4 = print_string_98._0;Unit (*v25)(Tuple0, void*) = v4._0;void* v26 = v4._1;Unit v5 = v25(b0_2, v26);b6_0 = v5; goto b6;b5:;Unit v6 = putchar_5((char)45);uint64_t (*v7)(Tuple23) = unwrap_1119;Tuple30 v8 = sub_i64_493._0;int64_t (*v27)(int64_t, int64_t, void*) = v8._0;void* v28 = v8._1;int64_t v9 = v27((int64_t)0ll, b0_0, v28);Tuple39 v10 = try_cast_i64_u64_495._0;Tuple23 (*v29)(int64_t, void*) = v10._0;void* v30 = v10._1;Tuple23 v11 = v29(v9, v30);uint64_t v12 = v7(v11);Unit v13 = print_unsigned_800(v12);b6_0 = v13; goto b6;b6:;b3_0 = b6_0; goto b3;b2:;uint64_t (*v14)(Tuple23) = unwrap_1119;Tuple39 v15 = try_cast_i64_u64_495._0;Tuple23 (*v23)(int64_t, void*) = v15._0;void* v24 = v15._1;Tuple23 v16 = v23(b0_0, v24);uint64_t v17 = v14(v16);Unit v18 = print_unsigned_800(v17);b3_0 = v18; goto b3;b3:;return b3_0;}
Unit println_1099(int32_t b0_0, Tuple15 b0_1) {b0:;Tuple14 v0 = b0_1._0;Unit (*v3)(int32_t, void*) = v0._0;void* v4 = v0._1;Unit v1 = v3(b0_0, v4);Unit v2 = putchar_5((char)10);return v2;}
int32_t deref_1123(void* b0_0) {b0:;int32_t v0 = *(int32_t*)b0_0;return v0;}
bool lambda_619(int64_t b0_0, int64_t b0_1, void* b0_2) {b0:;bool v0 = b0_0 == b0_1;return v0;}
Unit lambda_1100(void* b0_0, void* b0_1) {b0:;Tuple37 v0 = *(Tuple37*)b0_1;Tuple15 v1 = v0._0;Tuple8 v2 = v0._1;Tuple7 v3 = v2._0;int32_t (*v7)(void*, void*) = v3._0;void* v8 = v3._1;int32_t v4 = v7(b0_0, v8);Tuple14 v5 = v1._0;Unit (*v9)(int32_t, void*) = v5._0;void* v10 = v5._1;Unit v6 = v9(v4, v10);return v6;}
bool ___1101(size_t b0_0, size_t b0_1, Tuple36 b0_2) {b0:;Tuple35 v0 = b0_2._0;bool (*v4)(size_t, size_t, void*) = v0._0;void* v5 = v0._1;bool v1 = v4(b0_0, b0_1, v5);Tuple29 v2 = not_bool_94._0;bool (*v6)(bool, void*) = v2._0;void* v7 = v2._1;bool v3 = v6(v1, v7);return v3;}
Tuple23 Some_1125(uint64_t b0_0) {b0:;Tuple22 v0 = {b0_0};Tuple22 v1_src = v0; Tuple22 v1; memcpy(&v1, &v1_src, sizeof(Tuple22));Tuple23 v2 = {(uint8_t)1, v1};return v2;}
bool lambda_645(bool b0_0, void* b0_1) {bool b3_0; b0:;if (b0_0) { goto b1; } else { goto b2; }b1:;b3_0 = false; goto b3;b2:;b3_0 = true; goto b3;b3:;return b3_0;}
uint64_t transmute_1126(int64_t b0_0) {b0:;int64_t v0_src = b0_0; uint64_t v0; memcpy(&v0, &v0_src, sizeof(uint64_t));return v0;}
size_t array_len_1102(void* b0_0) {size_t b3_0; b0:;size_t (*v0)(Tuple3) = size_of_1109;Tuple3 v1 = MkType_1110;size_t v2 = v0(v1);Tuple35 v3 = eq_usz_167._0;bool (*v10)(size_t, size_t, void*) = v3._0;void* v11 = v3._1;bool v4 = v10(v2, (size_t)0ull, v11);if (v4) { goto b1; } else { goto b2; }b1:;b3_0 = (size_t)0ull; goto b3;b2:;size_t (*v5)(Tuple3) = size_of_1115;Tuple3 v6 = MkType_1116;size_t v7 = v5(v6);Tuple4 v8 = div_usz_587._0;size_t (*v12)(size_t, size_t, void*) = v8._0;void* v13 = v8._1;size_t v9 = v12(v7, v2, v13);b3_0 = v9; goto b3;b3:;return b3_0;}
Unit lambda_815(int32_t b0_0, void* b0_1) {b0:;Tuple42 v0 = cast_i32_i64_353._0;int64_t (*v6)(int32_t, void*) = v0._0;void* v7 = v0._1;int64_t v1 = v6(b0_0, v7);static uint8_t v2_bytes[] = {45,50,95,49,52,55,95,52,56,51,95,54,52,56,0}; void* v2 = v2_bytes;size_t v3_src = (size_t)0ull; void* v3; memcpy(&v3, &v3_src, sizeof(void*));Tuple0 v4 = {v2, v3, (uint32_t)14, (uint32_t)0};Unit v5 = print_signed_811(v1, (int64_t)-2147483648ll, v4);return v5;}
Unit panic_1103(Tuple0 b0_0, Tuple2 b0_1) {b0:;Tuple1 v0 = b0_1._0;Unit (*v7)(Tuple0, void*) = v0._0;void* v8 = v0._1;Unit v1 = v7(b0_0, v8);Unit v2 = exit_7((int32_t)1);Unit (*v3)(void*) = deref_ptr_1112;void* (*v4)(Unit) = null_1113;void* v5 = v4((Unit){});Unit v6 = v3(v5);__builtin_unreachable();}
int main(void) { main_1092((Unit){}); return 0; }
