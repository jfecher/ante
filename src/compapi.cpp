#include "compiler.h"

void linkInCompAPI(){}

/* Provide a callable C API from ante */
extern "C" {

    Node* Ante_getAST(){
        return ante::parser::getRootNode();
    }

}
