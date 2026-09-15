use std::{cell::RefCell, sync::Arc};

use inc_complete::DbGet;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    incremental::{GetItem, GetItemRaw, GetTypeBody, TypeCheck},
    iterator_extensions::mapvec,
    mir::{EffectKey, EvidenceEntry, FunctionType, Generic, Type, builder::Context},
    name_resolution::{Origin, builtin::Builtin},
    parser::{
        cst::{TopLevelItem, TopLevelItemKind, TypeDefinition, TypeDefinitionBody},
        ids::{ExprId, PathId, PatternId, TopLevelId, TopLevelName},
    },
    type_inference::{
        TypeBody,
        types::{Effect, Type as TCType, TypeBindings, TypeVariableId},
    },
};

impl<'local, Db> Context<'local, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem> + DbGet<GetItemRaw> + DbGet<GetTypeBody>,
{
    pub(super) fn convert_expr_type(&self, expr: ExprId) -> Type {
        let typ = &self.types.result.maps.expr_types[&expr];
        self.convert_type(typ, None)
    }

    pub(super) fn convert_path_type(&self, path: PathId) -> Type {
        let typ = &self.types.result.maps.path_types[&path];
        self.convert_type(typ, None)
    }

    pub(super) fn convert_pattern_type(&self, pattern: PatternId) -> Type {
        let typ = &self.types.result.maps.pattern_types[&pattern];
        self.convert_type(typ, None)
    }

    pub(super) fn convert_context(&self) -> ConvertTypeContext<'_, Db> {
        ConvertTypeContext {
            compiler: self.compiler,
            type_bindings: &self.types.bindings,
            generics_in_scope: &self.generics_in_scope,
            in_progress: RefCell::new(FxHashSet::default()),
        }
    }

    pub(super) fn convert_type(&self, typ: &TCType, args: Option<&[TCType]>) -> Type {
        self.convert_context().convert_type(typ, args)
    }

    /// If `typ` resolves to a `shared` user-defined type, returns its inner layout behind the pointer.
    pub(super) fn shared_inner_layout_of(&self, typ: &TCType) -> Option<Type> {
        self.convert_context().shared_inner_layout_of(typ, None).map(|(layout, _)| layout)
    }

    /// Like [Self::shared_inner_layout_of] but only for `shared mut`. Used to decide whether `:=` mutates in place.
    pub(super) fn shared_mut_inner_layout_of(&self, typ: &TCType) -> Option<Type> {
        self.convert_context().shared_inner_layout_of(typ, None).and_then(|(layout, mutable)| mutable.then_some(layout))
    }

    /// Returns the nth field of the tuple type, or [Type::ERROR] if there is none
    pub(super) fn tuple_field_type(tuple: &Type, n: usize) -> Type {
        match tuple {
            Type::Tuple(fields) => fields.get(n).cloned().unwrap_or(Type::ERROR),
            _ => Type::ERROR,
        }
    }
}

/// Maps type inference generics to Mir generics
type GenericsInScope = FxHashMap<crate::type_inference::generics::Generic, crate::mir::Generic>;

pub(super) struct ConvertTypeContext<'a, Db> {
    compiler: &'a Db,
    type_bindings: &'a TypeBindings,
    generics_in_scope: &'a GenericsInScope,

    /// Tracks the (Origin, args) pairs whose user-defined type bodies are currently
    /// being expanded. Without this, recursive ADTs like `Nat = | Zero | Succ Nat`
    /// cause unbounded recursion. This does not guard against polymorphic recursion.
    in_progress: RefCell<FxHashSet<(Origin, Arc<Vec<TCType>>)>>,
}

#[derive(Clone)]
pub(super) struct Effects {
    /// The concrete effects of the row, each carrying its capability
    pub(super) entries: Vec<Effect>,

    /// The row generics in scope this row stays polymorphic over
    pub(super) generics: Vec<Generic>,
}

/// True if both row entries name the same capability
pub(super) fn same_effect_id(a: &Effect, b: &Effect, bindings: &TypeBindings) -> bool {
    a.id.follow(bindings) == b.id.follow(bindings)
}

impl<Db> ConvertTypeContext<'_, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem> + DbGet<GetItemRaw> + DbGet<GetTypeBody>,
{
    /// TODO: The split of this from [Context::convert_type] ended up being unnecessary.
    pub(super) fn convert_type(&self, typ: &TCType, args: Option<&[TCType]>) -> Type {
        match typ.follow(self.type_bindings) {
            TCType::Primitive(primitive_type) => self.convert_primitive_type(*primitive_type),
            TCType::Generic(generic) => self.generics_in_scope.get(generic).map_or(Type::ERROR, |g| Type::Generic(*g)),
            TCType::Variable(id) => {
                // Any unbound variables at this point should be defaultable with only slight
                // changes in behavior. Implicits should already be found so this won't affect
                // impl search. A residual row variable also becomes `()`, which
                // `Type::substitute_evidence` treats as empty evidence.
                self.convert_type_variable(*id, Type::tuple(Vec::new()))
            },
            TCType::Function(function_type) => {
                // Uniform evidence convention: every function takes one trailing evidence parameter.
                let mut parameters = mapvec(&function_type.parameters, |typ| self.convert_type(&typ.typ, None));
                parameters.push(self.evidence_type(&function_type.effects));
                self.build_function_type(function_type, parameters)
            },
            TCType::Application(constructor, new_args) => {
                assert!(args.is_none());
                if let TCType::Primitive(crate::type_inference::types::PrimitiveType::Array) =
                    constructor.follow(self.type_bindings)
                {
                    return self.convert_array_application(new_args);
                }
                self.convert_type(constructor, Some(new_args))
            },
            TCType::UserDefined(origin) => self.convert_type_origin(*origin, args),
            TCType::Forall(_, typ) => self.convert_type(typ, args),
            TCType::Tuple(elements) => {
                let elements = mapvec(elements.iter(), |t| self.convert_type(t, None));
                Type::Tuple(Arc::new(elements))
            },
            // Carry through to MIR so monomorphization can substitute into Array lengths.
            TCType::U32(n) => Type::U32(*n),
            // A row used as a type (e.g. a row-generic instantiation binding) is its evidence.
            TCType::Effects(_) => self.evidence_type(typ),
            TCType::EffectId(id) => unreachable!("convert_type: effect id #{id} in a type position"),
            TCType::Place(_) | TCType::Places(_) => Type::UNIT,
        }
    }

    /// The canonical evidence type of an effect row, see [Type::Evidence]
    pub(super) fn evidence_type(&self, effects: &TCType) -> Type {
        self.row_evidence_type(&self.split_row(effects))
    }

    pub(super) fn row_evidence_type(&self, row: &Effects) -> Type {
        let mut entries = mapvec(&row.entries, |effect| EvidenceEntry::Capability {
            key: self.effect_key(effect),
            capability: self.effect_capability_tuple_type_of(effect),
        });
        entries.extend(row.generics.iter().map(|generic| EvidenceEntry::Rest(*generic)));
        Type::evidence(entries)
    }

    /// The mir generic a row's open end converts to
    fn row_generic(&self, generic: &TCType) -> Generic {
        match self.convert_type(generic, None) {
            Type::Generic(generic) => generic,
            other => panic!("row end `{generic:?}` is not a generic in scope, it converted to `{other}`"),
        }
    }

    /// The identity of a row entry's effect within evidence
    pub(super) fn effect_key(&self, effect: &Effect) -> EffectKey {
        let Some((name, args)) = self.definition_head(&effect.typ) else {
            panic!("effect_key: not an effect type: {:?}", effect.typ);
        };
        self.effect_key_of(name, args)
    }

    /// The identity of the effect `name` applied to `args` within evidence
    pub(super) fn effect_key_of(&self, name: TopLevelName, args: Option<&[TCType]>) -> EffectKey {
        let args = mapvec(args.unwrap_or(&[]), |arg| self.convert_type(arg, None));
        EffectKey { effect: name, args }
    }

    /// Resolves a row into its concrete effects and its open ends.
    pub(super) fn split_row(&self, effects: &TCType) -> Effects {
        let mut entries = Vec::new();
        let mut ends = Vec::new();

        // Only entries have ids so the row starts with the id that matches nothing
        self.collect_row_items(effects, &TCType::ERROR, &mut entries, &mut ends);

        let mut deduped: Vec<Effect> = Vec::with_capacity(entries.len());
        for entry in entries {
            if !deduped.iter().any(|kept: &Effect| kept.typ == entry.typ) {
                deduped.push(entry);
            }
        }

        // No reason to carry capabilities for effects with no operations
        deduped.retain(|effect| !self.effect_has_no_operations(&effect.typ));
        Effects { entries: deduped, generics: mapvec(&ends, |end| self.row_generic(end)) }
    }

    /// Recursively flattens `typ` into concrete effects and any open ends
    ///
    /// `id` is the id of the row entry `typ` came from.
    fn collect_row_items(&self, typ: &TCType, id: &TCType, entries: &mut Vec<Effect>, ends: &mut Vec<TCType>) {
        let followed = typ.follow(self.type_bindings);
        match followed {
            TCType::Effects(_) => {
                for effect in followed.effect_entries() {
                    self.collect_row_items(&effect.typ, &effect.id, entries, ends);
                }
            },
            // An unbound variable is only open if it is one of the enclosing function's own generics,
            // anything else is defaulted to `pure`
            TCType::Generic(_) | TCType::Variable(_) if self.is_open_end(followed) => {
                if !ends.contains(followed) {
                    ends.push(followed.clone());
                }
            },
            TCType::Variable(_) => (),
            other if other.is_error() => (),
            typ => entries.push(Effect { id: id.clone(), typ: typ.clone() }),
        }
    }

    /// True if `effect` refers to an effect definition with zero operations.
    fn effect_has_no_operations(&self, effect: &TCType) -> bool {
        let Some((name, _)) = self.definition_head(effect) else { return false };
        let (item, _) = GetItem(name.top_level_item).get(self.compiler);
        matches!(Self::type_definition(&item), Some(definition) if Self::is_empty_effect(definition))
    }

    /// The top-level definition `typ` names with its arguments if applied
    fn definition_head<'t>(&'t self, typ: &'t TCType) -> Option<(TopLevelName, Option<&'t [TCType]>)> {
        match typ.follow(self.type_bindings) {
            TCType::UserDefined(Origin::TopLevelDefinition(name)) => Some((*name, None)),
            TCType::Application(constructor, args) => match constructor.follow(self.type_bindings) {
                TCType::UserDefined(Origin::TopLevelDefinition(name)) => Some((*name, Some(args.as_slice()))),
                _ => None,
            },
            _ => None,
        }
    }

    fn type_definition(item: &TopLevelItem) -> Option<&TypeDefinition> {
        match &item.kind {
            TopLevelItemKind::TypeDefinition(definition) => Some(definition),
            _ => None,
        }
    }

    /// A desugared effect's fields are its operations
    fn is_empty_effect(definition: &TypeDefinition) -> bool {
        definition.kind.is_effect()
            && matches!(&definition.body, TypeDefinitionBody::Struct(fields) if fields.is_empty())
    }

    /// The method types of a trait dictionary type
    pub(super) fn trait_method_types(&self, dictionary: &TCType) -> Vec<TCType> {
        let Some((name, args)) = self.definition_head(dictionary) else {
            panic!("trait_method_types: `{dictionary:?}` is not a trait");
        };
        match name.top_level_item.type_body(args, self.compiler, None) {
            TypeBody::Product { fields, .. } => mapvec(fields, |(_, typ)| typ),
            TypeBody::Sum(_) => panic!("trait_method_types: trait is a sum type"),
        }
    }

    /// C-compatible conversion (no evidence parameter) for `extern` symbols and `resume`
    pub(super) fn convert_c_function_type(&self, typ: &TCType) -> Type {
        let TCType::Function(function_type) = typ.follow(self.type_bindings) else {
            return self.convert_type(typ, None);
        };
        let parameters = mapvec(&function_type.parameters, |typ| self.convert_type(&typ.typ, None));
        self.build_function_type(function_type, parameters)
    }

    /// Builds an effect's capability tuple type. The resulting tuple has each effect in declared order.
    pub(super) fn effect_capability_tuple_type(&self, effect_item: TopLevelId, args: Option<&[TCType]>) -> Type {
        let (item, _) = GetItemRaw(effect_item).get(self.compiler);
        let TopLevelItemKind::EffectDefinition(effect) = &item.kind else {
            panic!("effect_capability_tuple_type: item is not an effect definition");
        };
        let checked = TypeCheck(effect_item).get(self.compiler);
        let fields = mapvec(effect.body.iter(), |decl| {
            let method_type = checked.get_generalized(decl.name);
            let method_type =
                crate::type_inference::type_body::apply_type_constructor(&method_type, args, &checked, None);
            self.convert_operation_type(&method_type)
        });
        Type::Tuple(Arc::new(fields))
    }

    /// An effect operation's own signature, as provided by a handler branch: no capability parameters, `Pointer` environment.
    fn convert_operation_type(&self, typ: &TCType) -> Type {
        let TCType::Function(function_type) = typ.follow(self.type_bindings) else {
            return self.convert_type(typ, None);
        };
        let parameters = mapvec(&function_type.parameters, |typ| self.convert_type(&typ.typ, None));
        let return_type = self.convert_type(&function_type.return_type, None);
        Type::Function(Arc::new(FunctionType { parameters, environment: Type::POINTER, return_type }))
    }

    pub(super) fn build_function_type(
        &self, function_type: &crate::type_inference::types::FunctionType, parameters: Vec<Type>,
    ) -> Type {
        let environment = match function_type.environment.follow(self.type_bindings) {
            TCType::Variable(id) => self.convert_type_variable(*id, Type::NO_CLOSURE_ENV),
            other => self.convert_type(other, None),
        };
        let return_type = self.convert_type(&function_type.return_type, None);
        Type::Function(Arc::new(FunctionType { parameters, environment, return_type }))
    }

    fn is_open_end(&self, typ: &TCType) -> bool {
        match typ {
            TCType::Generic(_) => true,
            TCType::Variable(id) => {
                let generic = crate::type_inference::generics::Generic::Inferred(*id);
                self.generics_in_scope.contains_key(&generic)
            },
            _ => false,
        }
    }

    /// Resolves a [Self::split_row] entry to its capability tuple type
    pub(super) fn effect_capability_tuple_type_of(&self, effect: &Effect) -> Type {
        let Some((name, args)) = self.definition_head(&effect.typ) else {
            panic!("effect_capability_tuple_type_of: not an effect type: {:?}", effect.typ);
        };
        self.effect_capability_tuple_type(name.top_level_item, args)
    }

    fn convert_type_variable(&self, id: TypeVariableId, default: Type) -> Type {
        let generic = crate::type_inference::generics::Generic::Inferred(id);
        self.generics_in_scope.get(&generic).map_or(default, |g| Type::Generic(*g))
    }

    /// Build the MIR `Type::Array { length, element }` for an applied `Array n t`.
    fn convert_array_application(&self, new_args: &[TCType]) -> Type {
        assert_eq!(new_args.len(), 2, "Array applied to wrong arity; kind-checking should reject this");
        let length_type = new_args[0].follow(self.type_bindings);
        let elem = self.convert_type(&new_args[1], None);
        let length = match length_type {
            TCType::U32(n) => Type::U32(*n),
            TCType::Generic(generic) => self.generics_in_scope.get(generic).map_or(Type::ERROR, |g| Type::Generic(*g)),
            other => unreachable!("Array length is not a TypeLevelU32 or Generic: {other:?}"),
        };
        Type::array_with_length(length, elem)
    }

    fn convert_type_origin(&self, origin: Origin, args: Option<&[TCType]>) -> Type {
        match origin {
            Origin::TopLevelDefinition(id) => {
                let (item, _) = GetItem(id.top_level_item).get(self.compiler);
                if let Some(definition) = Self::type_definition(&item) {
                    // `shared` types are always represented as a pointer in MIR.
                    if definition.shared && id.local_name_id == definition.name {
                        return Type::POINTER;
                    }
                    // An effect used as a type is the evidence of the row holding just that effect
                    if definition.kind.is_effect() {
                        if Self::is_empty_effect(definition) {
                            return Type::evidence(Vec::new());
                        }
                        let capability = self.effect_capability_tuple_type(id.top_level_item, args);
                        let key = self.effect_key_of(id, args);
                        return Type::evidence(vec![EvidenceEntry::Capability { key, capability }]);
                    }
                }
                let key = (origin, Arc::new(args.unwrap_or(&[]).to_vec()));
                if !self.in_progress.borrow_mut().insert(key.clone()) {
                    // The type recursively references itself in a non-pointer position.
                    return Type::ERROR;
                }
                let result = self.expand_user_defined_body(id, args);
                self.in_progress.borrow_mut().remove(&key);
                result
            },
            Origin::Local(_) => unreachable!("Types cannot be declared locally"),
            Origin::TraitMember(_) | Origin::EffectOperation { .. } => unreachable!("Ability members are not types"),
            Origin::TypeResolution => unreachable!("Types should never be Origin::TypeResolution"),
            Origin::Builtin(builtin) => self.convert_builtin_type(builtin),
        }
    }

    /// Look through `Type::Application` and `Type::UserDefined` to find a top-level type
    /// definition. If it is `shared`, return the inner layout the pointer wraps along with
    /// whether the type is `shared mut`.
    fn shared_inner_layout_of(&self, typ: &TCType, args: Option<&[TCType]>) -> Option<(Type, bool)> {
        match typ.follow(self.type_bindings) {
            TCType::Application(constructor, new_args) => {
                assert!(args.is_none());
                self.shared_inner_layout_of(constructor, Some(new_args))
            },
            TCType::Forall(_, inner) => self.shared_inner_layout_of(inner, args),
            TCType::UserDefined(Origin::TopLevelDefinition(id)) => {
                let (item, _) = GetItem(id.top_level_item).get(self.compiler);
                let definition = Self::type_definition(&item)?;
                (definition.shared && id.local_name_id == definition.name)
                    .then(|| (self.expand_user_defined_body(*id, args), definition.mutable))
            },
            _ => None,
        }
    }

    fn expand_user_defined_body(&self, id: TopLevelName, args: Option<&[TCType]>) -> Type {
        let body = GetTypeBody(id, args.map(|args| args.to_vec())).get(self.compiler);
        self.convert_type_body(&body, self.common_field_count(id))
    }

    /// Number of with-clause fields on the union type `id` names
    pub(super) fn common_field_count(&self, id: TopLevelName) -> usize {
        id.common_field_count(self.compiler)
    }

    /// Like `common_field_count`, but resolves `typ` to its top-level definition first.
    pub(super) fn common_field_count_of_type(&self, typ: &TCType) -> usize {
        match typ.follow(self.type_bindings) {
            TCType::Application(constructor, _) => self.common_field_count_of_type(constructor),
            TCType::Forall(_, inner) => self.common_field_count_of_type(inner),
            TCType::UserDefined(Origin::TopLevelDefinition(id)) => self.common_field_count(*id),
            _ => 0,
        }
    }

    /// Converts a type body to the general representation of that type.
    ///
    /// Generally this is straightforward but with-clause fields on a union type are hoisted out
    /// of the union into an outer tuple: `(common_fields.., (tag, Union[variant_tuples...]))`.
    fn convert_type_body(&self, body: &TypeBody, common_field_count: usize) -> Type {
        match body {
            TypeBody::Product { type_name: _, fields } => {
                Type::tuple(mapvec(fields, |(_, field)| self.convert_type(field, None)))
            },
            TypeBody::Sum(variants) => {
                let k = common_field_count;
                let union = Type::union(mapvec(variants, |(_, _, fields)| {
                    Type::tuple(mapvec(&fields[..fields.len() - k], |(_, field)| self.convert_type(field, None)))
                }));
                let tagged = Type::tuple(vec![Type::tag_type(), union]);
                if k == 0 {
                    tagged
                } else {
                    let (_, _, first_variant_fields) = &variants[0];
                    let mut outer = mapvec(&first_variant_fields[first_variant_fields.len() - k..], |(_, field)| {
                        self.convert_type(field, None)
                    });
                    outer.push(tagged);
                    Type::tuple(outer)
                }
            },
        }
    }

    fn convert_builtin_type(&self, builtin: Builtin) -> Type {
        match builtin {
            Builtin::Unit => Type::UNIT,
            Builtin::Char => Type::CHAR,
            Builtin::Bool => Type::BOOL,
            Builtin::Ptr => Type::POINTER,
            Builtin::Array => unreachable!("bare Array reached MIR; kind-checking should reject partial application"),
            // LLVM has no bottom type. The builder pairs every divergent call with an
            // `Unreachable` terminator, so the erased Unit is dead at runtime.
            Builtin::Never => Type::UNIT,
            Builtin::Intrinsic => unreachable!("Builtin::Intrinsic is not a type"),
        }
    }

    fn convert_primitive_type(&self, typ: crate::type_inference::types::PrimitiveType) -> Type {
        match typ {
            crate::type_inference::types::PrimitiveType::Error => Type::ERROR,
            crate::type_inference::types::PrimitiveType::Unit => Type::UNIT,
            crate::type_inference::types::PrimitiveType::Bool => Type::BOOL,
            crate::type_inference::types::PrimitiveType::Pointer => Type::POINTER,
            crate::type_inference::types::PrimitiveType::Char => Type::CHAR,
            // See `Builtin::Never` above.
            crate::type_inference::types::PrimitiveType::Never => Type::UNIT,
            crate::type_inference::types::PrimitiveType::Int(kind) => Type::int(kind),
            crate::type_inference::types::PrimitiveType::Float(kind) => Type::float(kind),
            crate::type_inference::types::PrimitiveType::Reference(..) => Type::POINTER,
            crate::type_inference::types::PrimitiveType::Array => {
                unreachable!("bare Array reached MIR; applied form is handled in convert_type")
            },
            crate::type_inference::types::PrimitiveType::NoClosureEnv => Type::NO_CLOSURE_ENV,
        }
    }
}
