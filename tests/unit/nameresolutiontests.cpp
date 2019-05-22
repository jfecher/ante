#include "unittest.h"
#include "ptree.h"
#include "nameresolution.h"

using namespace ante;
using namespace parser;

/** 
 * let var1 = 3
 * var1
 */
TEST_CASE("Sequence Resolution", "[nameResolution]"){
    LOC_TY loc;
    auto declThenReference = new SeqNode(loc);
    auto var1 = new VarNode(loc, "var1");
    auto var1Cpy = new VarNode(loc, "var1");
    auto three = new IntLitNode(loc, "3", TT_I32);
    auto van = new VarAssignNode(loc, var1, three, false);
    van->modifiers.emplace_back(new ModNode(loc, Tok_Mut, nullptr));

    declThenReference->sequence.emplace_back(van);
    declThenReference->sequence.emplace_back(var1Cpy);

    REQUIRE(!var1->decl);
    REQUIRE(!var1Cpy->decl);

    NameResolutionVisitor v{"SequenceResolutionTest"};
    declThenReference->accept(v);

    REQUIRE(var1->decl);
    REQUIRE(var1Cpy->decl);
    REQUIRE(var1Cpy->decl == var1->decl);
    REQUIRE(var1Cpy->decl->name == "var1");
    delete declThenReference;
}


/** 
 * mut var1 = 1
 * mut var2 = 2
 * var1
 * var2 := 3
 * var2
 * var1 := 4
 */
TEST_CASE("Mutability Resolution", "[nameResolution]"){
    LOC_TY loc;
    auto seq = new SeqNode(loc);
    auto var1a = new VarNode(loc, "var1");
    auto var1b = new VarNode(loc, "var1");
    auto var1c = new VarNode(loc, "var1");

    auto var2a = new VarNode(loc, "var2");
    auto var2b = new VarNode(loc, "var2");
    auto var2c = new VarNode(loc, "var2");

    auto one = new IntLitNode(loc, "1", TT_I32);
    auto two = new IntLitNode(loc, "2", TT_I32);
    auto three = new IntLitNode(loc, "3", TT_I32);
    auto four = new IntLitNode(loc, "4", TT_I32);

    auto decl1 = new VarAssignNode(loc, var1a, one, true);
    auto decl2 = new VarAssignNode(loc, var2a, two, true);

    decl1->modifiers.emplace_back(new ModNode(loc, Tok_Mut, nullptr));
    decl2->modifiers.emplace_back(new ModNode(loc, Tok_Mut, nullptr));

    auto assign1 = new VarAssignNode(loc, var1c, three, true);
    auto assign2 = new VarAssignNode(loc, var2b, four, true);

    seq->sequence.emplace_back(decl1);
    seq->sequence.emplace_back(decl2);
    seq->sequence.emplace_back(var1b);
    seq->sequence.emplace_back(assign2);
    seq->sequence.emplace_back(var2c);
    seq->sequence.emplace_back(assign1);

    NameResolutionVisitor v{"Mutability Resolution Test"};
    seq->accept(v);

    REQUIRE(var1a->decl);
    REQUIRE(var1b->decl);
    REQUIRE(var1c->decl);
    REQUIRE(var2a->decl);
    REQUIRE(var2b->decl);
    REQUIRE(var2c->decl);

    REQUIRE(var1a->decl == var1b->decl);
    REQUIRE(var1b->decl == var1c->decl);

    REQUIRE(var2a->decl == var2b->decl);
    REQUIRE(var2b->decl == var2c->decl);

    REQUIRE(var1a->decl != var2a->decl);
    delete seq;
}

/** 
 * let var1 = 1
 * block
 *     let var1 = 2
 *     var1
 * var1
 */
TEST_CASE("Shadowing Resolution", "[nameResolution]"){
    LOC_TY loc;
    auto seq = new SeqNode(loc);
    auto innerSeq = new SeqNode(loc);
    auto var1a = new VarNode(loc, "var1");
    auto var1b = new VarNode(loc, "var1");
    auto var1c = new VarNode(loc, "var1");
    auto var1d = new VarNode(loc, "var1");

    auto one = new IntLitNode(loc, "1", TT_I32);
    auto two = new IntLitNode(loc, "2", TT_I32);

    auto decl1 = new VarAssignNode(loc, var1a, one, false);
    auto decl2 = new VarAssignNode(loc, var1b, two, false);

    decl1->modifiers.emplace_back(new ModNode(loc, Tok_Let, nullptr));
    decl2->modifiers.emplace_back(new ModNode(loc, Tok_Let, nullptr));

    innerSeq->sequence.emplace_back(decl2);
    innerSeq->sequence.emplace_back(var1c);
    auto block = new BlockNode(loc, innerSeq);

    seq->sequence.emplace_back(decl1);
    seq->sequence.emplace_back(block);
    seq->sequence.emplace_back(var1d);

    NameResolutionVisitor v{"ShadowingResolutionTest"};
    seq->accept(v);

    REQUIRE(var1a->decl);
    REQUIRE(var1b->decl);
    REQUIRE(var1c->decl);
    REQUIRE(var1d->decl);

    REQUIRE(var1a->decl == var1d->decl);
    REQUIRE(var1b->decl == var1c->decl);

    REQUIRE(var1a->decl != var1b->decl);
    delete seq;
}


/**
 * fun func: i32 param1 param2 =
 *     if true then param1
 *     else param2
 *
 * func
 * let param1 = 1
 * func 2 3
 * param1
 */
TEST_CASE("Function Resolution", "[nameResolution]"){
    LOC_TY loc;
    auto root = new RootNode(loc);

    auto p1Type = new TypeNode(loc, TT_I32, "", nullptr);
    auto p2Type = new TypeNode(loc, TT_I32, "", nullptr);
    auto p1a = new NamedValNode(loc, "param1", p1Type);
    auto p2a = new NamedValNode(loc, "param2", p2Type);
    p1a->next.reset(p2a);

    auto cond = new BoolLitNode(loc, true);
    auto p1b = new VarNode(loc, "param1");
    auto p2b = new VarNode(loc, "param2");

    auto ifn = new IfNode(loc, cond, p1b, p2b);

    auto fdn = new FuncDeclNode(loc, "func", nullptr, p1a, {}, ifn);

    root->funcs.emplace_back(fdn);
    auto funcA = new VarNode(loc, "func");
    auto funcB = new VarNode(loc, "func");
    auto one = new IntLitNode(loc, "1", TT_I32);
    auto two = new IntLitNode(loc, "2", TT_I32);
    auto three = new IntLitNode(loc, "3", TT_I32);

    auto p1c = new VarNode(loc, "param1");
    auto param1Decl = new VarAssignNode(loc, p1c, one, false);
    param1Decl->modifiers.emplace_back(new ModNode(loc, Tok_Let, nullptr));
    auto p1d = new VarNode(loc, "param1");

    std::vector<std::unique_ptr<Node>> argvec;
    argvec.emplace_back(two);
    argvec.emplace_back(three);
    auto args = new TupleNode(loc, argvec);
    auto call = new BinOpNode(loc, '(', funcB, args);

    root->main.emplace_back(funcA);
    root->main.emplace_back(param1Decl);
    root->main.emplace_back(call);
    root->main.emplace_back(p1d);

    NameResolutionVisitor visitor{"FunctionResolutionTest"};
    root->accept(visitor);

    REQUIRE(funcA->decl);
    REQUIRE(funcB->decl);
    REQUIRE(funcA->decl == funcB->decl);
    REQUIRE(funcA->decl == fdn->decl);

    REQUIRE(p1a->decl);
    REQUIRE(p1b->decl);
    REQUIRE(p1a->decl == p1b->decl);
    REQUIRE(p1a->decl != funcA->decl);

    REQUIRE(p2a->decl);
    REQUIRE(p2b->decl);
    REQUIRE(p2a->decl == p2b->decl);
    REQUIRE(p2a->decl != funcA->decl);

    //Shadowing of function parameters
    REQUIRE(p1c->decl);
    REQUIRE(p1d->decl);
    REQUIRE(p1c->decl == p1d->decl);
    REQUIRE(p1c->decl != p1a->decl);
    REQUIRE(p1c->decl != p2a->decl);
    REQUIRE(p1c->decl != funcA->decl);
    delete root;
}


TEST_CASE("Integer Type Resolution", "[typeResolution]"){
    LOC_TY loc;
    auto i32 = new TypeNode(loc, TT_I32, "", nullptr);
    
    NameResolutionVisitor{"IntTypeResolutionTest"}.visit(i32);

    REQUIRE(i32->getType() == AnType::getI32());
}


TEST_CASE("Array Type Resolution", "[typeResolution]"){
    LOC_TY loc;

    auto usz = new TypeNode(loc, TT_Usz, "", nullptr);
    auto uszPtr = new TypeNode(loc, TT_Ptr, "", usz);
    auto arrOfUszPtr = new TypeNode(loc, TT_Array, "", uszPtr);
    
    NameResolutionVisitor{"ArrayTypeResolutionTest"}.visit(arrOfUszPtr);

    REQUIRE(arrOfUszPtr->getType() == AnArrayType::get(AnPtrType::get(AnType::getUsz())));

    /** Name resolution should not do unnecessary deep-resolving */
    REQUIRE(uszPtr->getType() == nullptr);
}
