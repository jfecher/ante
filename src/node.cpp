#include "parser.h"


NodeIterator NodeIterator::operator++(){
    n = n->next.get();
    return *this;
}

NodeIterator NodeIterator::operator--(){
    n = n->prev;
    return *this;
}

Node* NodeIterator::operator*(){
    return n;
}
    
bool NodeIterator::operator==(NodeIterator ni){
    return n == ni.n;
}

bool NodeIterator::operator!=(NodeIterator ni){
    return n != ni.n;
}


NodeIterator Node::begin(){
    return {this};
}

NodeIterator Node::end(){
    return {0};
}
