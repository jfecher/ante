use std::{cell::RefCell, sync::Arc};

use inc_complete::DbGet;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    incremental::{GetItem, GetItemRaw, TypeCheck},
    iterator_extensions::mapvec,
    mir::{FunctionType, Type, builder::Context},
    name_resolution::{Origin, builtin::Builtin},
    parser::{
        cst::TopLevelItemKind,
        ids::{ExprId, PathId, PatternId, TopLevelId},
    },
    type_inference::{
        TypeBody,
        types::{Type as TCType, TypeBindings, TypeVariableId},
    },
};

impl<'local, Db> Context<'local, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem> + DbGet<GetItemRaw>,
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

    /// Like [Self::shared_inner_layout_of] but only returns the inner layout when the type is
    /// declared `shared mut`, used to decide whether `:=` mutates the pointee in place.
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

/// What a resolved effect row ends in after its concrete effects.
pub(super) enum RowEnd {
    /// The row is closed (or ends in a residual unbound variable, which defaults to pure).
    Closed,
    /// The row is genuinely polymorphic: it ends in a row generic.
    Generic(TCType),
}

impl<Db> ConvertTypeContext<'_, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem> + DbGet<GetItemRaw>,
{
    /// TODO: The split of this from [Context::convert_type] ended up being unnecessary.
    pub(super) fn convert_type(&self, typ: &TCType, args: Option<&[TCType]>) -> Type {
        match typ.follow(self.type_bindings) {
            TCType::Primitive(primitive_type) => self.convert_primitive_type(*primitive_type),
            TCType::Generic(generic) => self.generics_in_scope.get(generic).map_or(Type::ERROR, |g| Type::Generic(*g)),
            TCType::Variable(id) => {
                // Any unbound variables at this point should be defaultable with only slight
                // changes in behavior. Implicits should already be found so this won't affect
                // impl search. The empty tuple doubles as the evidence of a residual row
                // variable, keeping instantiation bindings consistent with `evidence_type`.
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
            TCType::UserDefined(origin) => self.convert_type_origin(*origin, args, None),
            TCType::Forall(_, typ) => self.convert_type(typ, args),
            TCType::Tuple(elements) => {
                let elements = mapvec(elements.iter(), |t| self.convert_type(t, None));
                Type::Tuple(Arc::new(elements))
            },
            // Carry through to MIR so monomorphization can substitute into Array lengths.
            TCType::U32(n) => Type::U32(*n),
            // A row used as a type (e.g. a row-generic instantiation binding) is its evidence.
            TCType::Effects(_, _) => self.evidence_type(typ),
        }
    }

    /// The type of a row's evidence: a cons list of capability tuples for the row's resolved
    /// effects, ending in `()` (closed) or the row's generic. Determined solely by the row's
    /// resolved content, so every view of a row agrees on it.
    pub(super) fn evidence_type(&self, effects: &TCType) -> Type {
        let (concretes, end) = self.split_row(effects);
        let mut evidence = match end {
            RowEnd::Closed => Type::tuple(Vec::new()),
            RowEnd::Generic(generic) => self.convert_type(&generic, None),
        };
        for effect in concretes.iter().rev() {
            evidence = Type::tuple(vec![self.effect_capability_tuple_type_of(effect), evidence]);
        }
        evidence
    }

    /// Splits a row into its resolved concrete effects and its end. Purely structural (no
    /// sorting or deduplication) so that converting an instantiated row is identical to
    /// substituting into the converted generic row.
    pub(super) fn split_row(&self, effects: &TCType) -> (Vec<TCType>, RowEnd) {
        let mut concretes = Vec::new();
        let mut current = effects.clone();
        let end = loop {
            match current.follow(self.type_bindings).clone() {
                TCType::Effects(list, tail) => {
                    concretes.extend(list.iter().cloned());
                    match tail {
                        Some(tail) => current = (*tail).clone(),
                        None => break RowEnd::Closed,
                    }
                },
                generic @ TCType::Generic(_) => break RowEnd::Generic(generic),
                TCType::Variable(id) => {
                    // An unbound variable is a generic if it was generalized, else residual (pure).
                    let generic = crate::type_inference::generics::Generic::Inferred(id);
                    break match self.generics_in_scope.contains_key(&generic) {
                        true => RowEnd::Generic(TCType::Variable(id)),
                        false => RowEnd::Closed,
                    };
                },
                _ => break RowEnd::Closed,
            }
        };
        // A chain ending at a generic is a definition-side view: canonicalize its prefix to
        // match the generalized scheme callers instantiate (same head-stable order as
        // [crate::type_inference::types::Type::effects]). A closed chain mirrors the
        // instantiation structure exactly and must stay as-is.
        if matches!(end, RowEnd::Generic(_)) {
            concretes.sort_by_key(|effect| effect.effect_head().copied());
            let mut deduped = Vec::with_capacity(concretes.len());
            for effect in concretes {
                if !deduped.contains(&effect) {
                    deduped.push(effect);
                }
            }
            concretes = deduped;
        }
        (concretes, end)
    }

    /// C-compatible function conversion: no evidence parameter. Used for `extern` symbols
    /// and `resume` (a coroutine primitive), whose shapes are fixed by their consumers.
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
            let method_type = crate::type_inference::type_body::apply_type_constructor(&method_type, args, &checked);
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

    pub(super) fn build_function_type(&self, function_type: &crate::type_inference::types::FunctionType, parameters: Vec<Type>) -> Type {
        let environment = match function_type.environment.follow(self.type_bindings) {
            TCType::Variable(id) => self.convert_type_variable(*id, Type::NO_CLOSURE_ENV),
            other => self.convert_type(other, None),
        };
        let return_type = self.convert_type(&function_type.return_type, None);
        Type::Function(Arc::new(FunctionType { parameters, environment, return_type }))
    }

    /// Resolves a concrete effect to its capability tuple type.
    pub(super) fn effect_capability_tuple_type_of(&self, mut effect_type: &TCType) -> Type {
        while let TCType::Effects(list, None) = effect_type
            && let [single] = list.as_slice()
        {
            effect_type = single;
        }
        let (origin, args) = match effect_type {
            TCType::UserDefined(origin) => (*origin, None),
            TCType::Application(constructor, args) => match constructor.as_ref() {
                TCType::UserDefined(origin) => (*origin, Some(args.as_slice())),
                _ => panic!("effect_capability_tuple_type_of: not an effect type: {effect_type:?}"),
            },
            _ => panic!("effect_capability_tuple_type_of: not an effect type: {effect_type:?}"),
        };
        let Origin::TopLevelDefinition(name) = origin else {
            panic!("effect_capability_tuple_type_of: effect origin is not a top-level definition");
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

    fn convert_type_origin(&self, origin: Origin, args: Option<&[TCType]>, variant_index: Option<usize>) -> Type {
        match origin {
            Origin::TopLevelDefinition(id) => {
                // `shared` types are always represented as a pointer in MIR.
                if Self::is_shared_type_definition(self.compiler, id.top_level_item) {
                    return Type::POINTER;
                }
                let key = (origin, Arc::new(args.unwrap_or(&[]).to_vec()));
                if !self.in_progress.borrow_mut().insert(key.clone()) {
                    // The type recursively references itself in a non-pointer position.
                    return Type::ERROR;
                }
                let result = self.expand_user_defined_body(id.top_level_item, args, variant_index);
                self.in_progress.borrow_mut().remove(&key);
                result
            },
            Origin::Local(_) => unreachable!("Types cannot be declared locally"),
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
                let (shared, mutable) = Self::shared_type_flags(self.compiler, id.top_level_item);
                shared.then(|| (self.expand_user_defined_body(id.top_level_item, args, None), mutable))
            },
            _ => None,
        }
    }

    fn expand_user_defined_body(&self, id: TopLevelId, args: Option<&[TCType]>, variant_index: Option<usize>) -> Type {
        let body = id.type_body(args, self.compiler);
        self.convert_type_body(body, variant_index)
    }

    fn is_shared_type_definition(compiler: &Db, id: TopLevelId) -> bool {
        Self::shared_type_flags(compiler, id).0
    }

    /// Returns the `(shared, mutable)` flags of a top-level type definition, or `(false, false)`.
    fn shared_type_flags(compiler: &Db, id: TopLevelId) -> (bool, bool) {
        let (item, _) = GetItem(id).get(compiler);
        match &item.kind {
            TopLevelItemKind::TypeDefinition(td) => (td.shared, td.mutable),
            _ => (false, false),
        }
    }

    /// Converts a type body to the general representation of that type.
    ///
    /// If `variant_index` is specified, the default index used to represent the sum type
    /// is overridden with the given index. In either case, sum types with multiple possible
    /// constructors will always include the tag type.
    fn convert_type_body(&self, body: TypeBody, variant_index: Option<usize>) -> Type {
        match body {
            TypeBody::Product { type_name: _, fields } => {
                Type::tuple(mapvec(fields, |(_, field)| self.convert_type(&field, None)))
            },
            TypeBody::Sum(variants) => {
                let union = if let Some((_, variant_args)) = variant_index.and_then(|i| variants.get(i)) {
                    // If we want to retrieve 1 specific variant then create a tuple of each field
                    Type::tuple(mapvec(variant_args, |field| self.convert_type(field, None)))
                } else {
                    // Otherwise we need a raw union of the fields of all variants
                    Type::union(mapvec(variants, |(_, fields)| {
                        Type::tuple(mapvec(fields, |field| self.convert_type(&field, None)))
                    }))
                };
                // Then pack the result with a separate tag value.
                Type::tuple(vec![Type::tag_type(), union])
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
