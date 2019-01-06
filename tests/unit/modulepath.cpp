#include "unittest.h"
#include "module.h"
using namespace ante;

#define LOOP_CHECK(file) \
    if(count > 100){     \
        SECTION("Loop count exceeded 100 with path " file){ \
            REQUIRE(false); \
        } \
    }

TEST_CASE("ModulePaths can be iterated over", "[ModulePath]"){
    ModulePath path1{"container.an"};
    ModulePath path2{"d1/file.an"};
    ModulePath path3{"d1/d2/d3/file2.an"};

    size_t count = 0;
    for(std::string const& s : path1){
        count++;
        LOOP_CHECK("container.an");
    }
    REQUIRE(count == 1);

    count = 0;
    for(std::string const& s : path2){
        count++;
        LOOP_CHECK("d1/file.an");
    }
    REQUIRE(count == 2);

    count = 0;
    for(std::string const& s : path3){
        count++;
        LOOP_CHECK("d1/d2/d3/file2.an");
    }
    REQUIRE(count == 4);
}

// Concatenate all iterated elements, joining with .
template<typename T>
std::string pathStr(T &path){
    std::string ret;
    size_t count = 0;
    for(std::string const& s : path){
        ret += s + ".";
        LOOP_CHECK(+ ret);
        count++;
    }
    return ret.empty() ? ret : ret.substr(0, ret.length() - 1);
}


TEST_CASE("ModulePaths ignore . in the path", "[ModulePath]"){
    ModulePath prelude{"./stdlib/prelude.an"};
    ModulePath inSameDir{"./vec.an"};
    ModulePath empty{"."};
    ModulePath parent{".."};
    ModulePath invalid{"./."};

    REQUIRE(pathStr(prelude) == "Stdlib.Prelude");
    REQUIRE(pathStr(inSameDir) == "Vec");
    REQUIRE(pathStr(empty) == "");
    REQUIRE(pathStr(parent) == "..");
    REQUIRE(pathStr(invalid) == "");
}


TEST_CASE("ModulePaths treat both / and \\ as directory separators", "[ModulePath]"){
    ModulePath winPath{"d1\\d2/d3\\file4.an"};
    ModulePath unixPath{"d4/d5\\d6/file7.an"};

    REQUIRE(pathStr(winPath) == "D1.D2.D3.File4");
    REQUIRE(pathStr(unixPath) == "D4.D5.D6.File7");
}
