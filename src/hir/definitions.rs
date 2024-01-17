use std::collections::HashMap;

use crate::{cache::DefinitionInfoId, types};

use super::monomorphisation::Definition;

pub struct Definitions {
    // This is a rather inefficient representation with duplication
    // to prevent locals of one type overwriting locals of the same type
    // on different instantiations of a function, which can happen in some
    // rare instances.
    all: DefinitionMap,
    local: Vec<DefinitionMap>,
}

type DefinitionMap = HashMap<DefinitionKey, Definition>;

type DefinitionKey = (DefinitionInfoId, DefinitionType);

impl Definitions {
    pub fn new() -> Self {
        Self { all: HashMap::new(), local: vec![HashMap::new()] }
    }

    pub fn get(&self, id: DefinitionInfoId, typ: types::Type) -> Option<&Definition> {
        let locals = self.local.last().unwrap();
        if let Some(definition) = locals.get(&(id, DefinitionType(typ.clone()))) {
            return Some(definition);
        }

        self.all.get(&(id, DefinitionType(typ.clone())))
    }

    pub fn insert(&mut self, id: DefinitionInfoId, typ: types::Type, definition: Definition) {
        let locals = self.local.last_mut().unwrap();
        locals.insert((id, DefinitionType(typ.clone())), definition.clone());
        self.all.insert((id, DefinitionType(typ)), definition);
    }

    pub fn push_local_scope(&mut self) {
        self.local.push(HashMap::new());
    }

    pub fn pop_local_scope(&mut self) {
        self.local.pop();
    }
}

/// Wrapper around types::Type to change hashing behavior.
/// We now treat all type variables as if they are compatible
#[derive(Eq)]
struct DefinitionType(types::Type);

impl std::hash::Hash for DefinitionType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.traverse_no_follow(|typ| {
            std::mem::discriminant(typ).hash(state);

            // Hash any extra information, not including type variable ids.
            match typ {
                types::Type::Primitive(primitive) => primitive.hash(state),
                types::Type::UserDefined(id) => id.hash(state),
                types::Type::TypeVariable(_) => (), // Do nothing
                types::Type::Function(_) => (),
                types::Type::TypeApplication(_, _) => (),
                types::Type::Ref(_) => (),
                types::Type::Struct(field_names, _) => {
                    for name in field_names {
                        name.hash(state);
                    }
                },
                types::Type::Effects(set) => {
                    for (id, _) in &set.effects {
                        id.hash(state);
                    }
                },
            }
        })
    }
}

// TODO: May need a more complex, try-unify scheme to remember positions of compatible type variables
impl PartialEq for DefinitionType {
    fn eq(&self, other: &Self) -> bool {
        definition_type_eq(&self.0, &other.0)
    }
}

fn definition_type_eq(a: &types::Type, b: &types::Type) -> bool {
    use types::Type;
    match (a, b) {
        (Type::Primitive(primitive1), Type::Primitive(primitive2)) => primitive1 == primitive2,
        (Type::UserDefined(id1), Type::UserDefined(id2)) => id1 == id2,
        (Type::TypeVariable(_), Type::TypeVariable(_)) | (Type::Ref(_), Type::Ref(_)) => true, // Do nothing
        (Type::Function(f1), Type::Function(f2)) => {
            if f1.parameters.len() != f2.parameters.len() {
                return false;
            }
            f1.parameters.iter().zip(&f2.parameters).all(|(p1, p2)| definition_type_eq(p1, p2))
                && definition_type_eq(&f1.environment, &f2.environment)
                && definition_type_eq(&f1.return_type, &f2.return_type)
                && definition_type_eq(&f1.effects, &f2.effects)
        },
        (Type::TypeApplication(constructor1, args1), Type::TypeApplication(constructor2, args2)) => {
            if args1.len() != args2.len() {
                return false;
            }
            args1.iter().zip(args2).all(|(p1, p2)| definition_type_eq(p1, p2))
                && definition_type_eq(constructor1, constructor2)
        },
        (Type::Struct(field_names1, _), Type::Struct(field_names2, _)) => {
            if field_names1.len() != field_names2.len() {
                return false;
            }
            field_names1
                .iter()
                .zip(field_names2)
                .all(|((name1, t1), (name2, t2))| name1 == name2 && definition_type_eq(t1, t2))
        },
        (Type::Effects(set1), Type::Effects(set2)) => {
            if set1.effects.len() != set2.effects.len() {
                return false;
            }
            set1.effects.iter().zip(&set2.effects).all(|((id1, args1), (id2, args2))| {
                if args1.len() != args2.len() {
                    return false;
                }
                id1 == id2 && args1.iter().zip(args2).all(|(t1, t2)| definition_type_eq(t1, t2))
            })
        },
        (othera, otherb) => {
            assert_ne!(std::mem::discriminant(othera), std::mem::discriminant(otherb), "ICE: Missing match case");
            false
        },
    }
}
