//! This file implements the infinitely sized check on types to see if a type definition
//! stores itself inline. This needs to work on mutually recursive types so it is not
//! currently an inc-complete query.
//!
//! ```ante
//! type Inline t = x: t
//! type A = | ALeaf | ANode (Inline A)  // A contains Inline A which contains A
//! type B = | BLeaf | BNode (Rc B)      // Fine, Rc stores B behind a pointer
//! ```
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    incremental::{DbHandle, GetItem, Resolve},
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::{self, TopLevelItemKind},
        ids::{NameId, TopLevelName},
    },
    type_inference::{
        TypeChecker,
        generics::Generic,
        types::{PrimitiveType, Type},
    },
};

/// True if the type definition `type_name` stores itself inline
pub(super) fn is_infinitely_sized(type_name: TopLevelName, definition: &cst::TypeDefinition, db: &DbHandle) -> bool {
    let mut walk = InlineWalk { db, inline_generics: FxHashMap::default(), recursive: FxHashSet::default() };
    walk.walk(type_name, definition);

    walk.recursive.iter().any(|name| name.top_level_item == type_name.top_level_item)
}

/// A depth-first walk over each type stored inline
struct InlineWalk<'local, 'inner> {
    db: &'local DbHandle<'inner>,

    /// Whether each generic of each type visited so far is stored inline, or `None` while that
    /// type is still being visited. Types in a cycle may be missing some, but they're already errors.
    inline_generics: FxHashMap<TopLevelName, Option<Rc<[bool]>>>,

    /// Each type found stored inline within itself
    recursive: FxHashSet<TopLevelName>,
}

impl<'local, 'inner> InlineWalk<'local, 'inner> {
    /// Visit each field of `name`, returning whether each of its generics is stored inline
    fn walk(&mut self, name: TopLevelName, definition: &cst::TypeDefinition) -> Rc<[bool]> {
        self.inline_generics.insert(name, None);

        let mut generics = FxHashSet::default();
        for field in self.field_types(name, definition) {
            self.visit(&field, &mut generics);
        }

        let inline: Rc<[bool]> = definition.generics.iter().map(|generic| generics.contains(&generic.name)).collect();
        self.inline_generics.insert(name, Some(inline.clone()));
        inline
    }

    /// Converts the field types of `name`, which is either the type `definition` defines or
    /// one of its variants. For the type itself, this includes the fields of every variant.
    fn field_types(&self, name: TopLevelName, definition: &cst::TypeDefinition) -> Vec<Type> {
        let fields: Vec<&cst::Type> = match &definition.body {
            cst::TypeDefinitionBody::Struct(fields) => mapvec(fields, |(_, field)| field),
            cst::TypeDefinitionBody::Enum(variants, _) => variants
                .iter()
                .filter(|(variant, _)| name.local_name_id == definition.name || name.local_name_id == *variant)
                .flat_map(|(_, fields)| fields.iter().map(|(_, field)| field))
                .collect(),

            // Aliases are already expanded when converting the types referring to them
            cst::TypeDefinitionBody::Alias(_)
            | cst::TypeDefinitionBody::EffectAlias(_)
            | cst::TypeDefinitionBody::Error => Vec::new(),
        };

        let resolve = Resolve(name.top_level_item).get(self.db);
        let mut local_kinds = TypeChecker::local_kinds_from_generics(&definition.generics);
        let mut next_id = 0;
        mapvec(fields, |field| {
            Type::from_cst_type(field, &resolve, self.db, &mut next_id, &mut local_kinds, false, false)
        })
    }

    /// Record each generic `typ` stores inline in `generics`, visiting each type it stores inline.
    /// `generics` are the generics of the type definition `typ` is a field type of.
    fn visit(&mut self, typ: &Type, generics: &mut FxHashSet<NameId>) {
        match typ {
            Type::Generic(Generic::Named(Origin::Local(name))) => {
                generics.insert(*name);
            },
            Type::UserDefined(Origin::TopLevelDefinition(name)) => self.visit_type(*name, &[], generics),
            Type::Application(constructor, args) => match constructor.as_ref() {
                Type::Primitive(PrimitiveType::Pointer | PrimitiveType::Reference(_) | PrimitiveType::Error) => (),
                Type::UserDefined(Origin::TopLevelDefinition(name)) => self.visit_type(*name, args, generics),
                // We conservatively assume a generic constructor `f` in `f t` may store `t` inline.
                other => {
                    self.visit(other, generics);
                    for arg in args.iter() {
                        self.visit(arg, generics);
                    }
                },
            },
            Type::Tuple(elements) => {
                for element in elements.iter() {
                    self.visit(element, generics);
                }
            },
            Type::Function(function) => self.visit(&function.environment, generics),
            Type::Forall(_, body) => self.visit(body, generics),
            Type::Primitive(_)
            | Type::Generic(_)
            | Type::Variable(_)
            | Type::UserDefined(_)
            | Type::U32(_)
            | Type::Effects(_)
            | Type::Place(_)
            | Type::Places(_)
            | Type::PlaceProjection(..) => (),
        }
    }

    /// Visit `name` applied to `args`
    fn visit_type(&mut self, name: TopLevelName, args: &[Type], generics: &mut FxHashSet<NameId>) {
        let (item, _) = GetItem(name.top_level_item).get(self.db);
        let TopLevelItemKind::TypeDefinition(definition) = &item.kind else {
            return;
        };

        // `shared` types are always behind a pointer
        let is_shared = definition.shared && name.local_name_id == definition.name;
        if is_shared || definition.kind.is_ability() {
            return;
        }

        let inline = match self.inline_generics.get(&name) {
            Some(Some(inline)) => inline.clone(),
            // `name` is still being visited, so it is stored inline within itself
            Some(None) => {
                self.recursive.insert(name);
                return;
            },
            None => self.walk(name, definition),
        };

        for (&inline, arg) in inline.iter().zip(args) {
            if inline {
                self.visit(arg, generics);
            }
        }
    }
}
