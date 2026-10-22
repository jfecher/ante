use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use smallbitvec::SmallBitVec;

use crate::{
    diagnostics::Diagnostic,
    incremental::GetType,
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::Name,
        ids::{ExprId, TopLevelId, TopLevelName},
    },
    type_inference::{
        Locateable, TraitDefinition, TypeChecker, Variance,
        errors::TypeErrorKind,
        row::{self, RowMode},
        types::{Type, TypeBindings},
    },
};

/// How the mir builder adapts a trait dictionary coerced to wider effect args
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryCoercion {
    pub source: CoercionSource,

    /// How each of the trait's members is adapted, in declaration order
    pub members: Vec<MemberCoercion>,
}

/// What the adapted methods of a coerced dictionary call
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoercionSource {
    /// The dictionary was created from the given impl, instantiated with the given bindings. Each
    /// method whose effects changed is adapted by a function calling that impl's method directly.
    /// The bindings are in the order of the generics of the impl's generalized type, which are also
    /// the generics the mir builder makes each of the impl's methods generic over.
    Direct { impl_name: TopLevelName, bindings: Vec<Type> },

    /// It is unknown which impl the dictionary was created from, so the coerced dictionary's impl
    /// data is the original dictionary, and its impl type argument is the original dictionary's type.
    /// Each method is adapted by a function calling the original dictionary's method.
    Wrapped,
}

/// How a member of a coerced dictionary is adapted
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberCoercion {
    /// A constant whose type is unchanged
    Constant,

    /// A method, which is adapted to take the wider effects' evidence if its effects changed
    Method,

    /// A constant holding a dictionary created from a known impl whose effect args were widened
    Nested(DictionaryCoercion),
}

/// A dictionary coerced to a trait type which may have wider effect args. Whether any were
/// widened is only decided once every item in the SCC is checked.
#[derive(Clone)]
pub(super) struct PendingDictionaryCoercion {
    actual: Type,
    expected: Type,
}

/// Why a dictionary can't be coerced to wider effect args
enum CoercionError {
    /// It isn't known which impl the dictionary was created from and the coerced dictionary's
    /// impl type is fixed, so it can't hold the original dictionary.
    UnknownImpl,
    /// The type of this field, which isn't a method, changes
    Field(Name),
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Unify the trait dictionary type `actual` with the trait type `expected` it is coerced to,
    /// which may have wider effect args. Its impl type argument is left unlinked if it is unknown.
    pub(super) fn coerce_dictionary(&mut self, actual: &Type, expected: &Type, expr: ExprId, kind: TypeErrorKind) {
        let unlinked = self.unlinked_dictionary_type(actual, expected);
        let unlinked = unlinked.as_ref().unwrap_or(actual);
        let mut bindings = TypeBindings::default();
        match self.subtype_widening(unlinked, expected, &mut bindings) {
            Ok(()) => self.bindings.extend(bindings),
            Err(()) => self.report_type_error(unlinked, expected, kind, expr),
        }
        self.record_dictionary_coercion(expr, actual, expected);
    }

    /// Check `actual <: expected` where, if both are the same trait, the trait dictionary `actual` may
    /// have narrower effect args than `expected`. Every other relation compares a dictionary's effect
    /// args exactly, so this may only be used where the coercion is recorded for the mir builder.
    pub(super) fn subtype_widening(
        &self, actual: &Type, expected: &Type, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let effect_args = self.trait_effect_args(actual, new_bindings);
        let actual_followed = actual.follow_two(&self.bindings, new_bindings).clone();
        let expected_followed = expected.follow_two(&self.bindings, new_bindings).clone();
        let (Some(effect_args), Type::Application(a_constructor, a_args), Type::Application(b_constructor, b_args)) =
            (effect_args, actual_followed, expected_followed)
        else {
            return self.subtype(actual, expected, Variance::Covariant, RowMode::Coercible, new_bindings);
        };
        if a_args.len() != b_args.len() {
            return Err(());
        }

        self.subtype(&a_constructor, &b_constructor, Variance::Invariant, RowMode::Exact, new_bindings)?;
        for (index, (a_arg, b_arg)) in a_args.iter().zip(b_args.iter()).enumerate() {
            let is_effect_arg = effect_args.get(index) == Some(true);
            if is_effect_arg && self.is_closed_row_below_open_end(a_arg, b_arg, new_bindings) {
                // A bare type variable would otherwise be bound to exactly this narrower row
                self.row_subtype(a_arg, b_arg, new_bindings)?;
            } else {
                let arg_rows = if is_effect_arg { RowMode::Coercible } else { RowMode::Exact };
                self.subtype(a_arg, b_arg, Variance::Invariant, arg_rows, new_bindings)?;
            }
        }
        Ok(())
    }

    /// If `typ` is a trait, return which of its parameters are effect rows. A trait's impl generic
    /// is never an effect row so it is always compared exactly.
    fn trait_effect_args(&self, typ: &Type, new_bindings: &TypeBindings) -> Option<SmallBitVec> {
        let Some(Origin::TopLevelDefinition(name)) = typ.effect_head(&self.bindings, new_bindings) else { return None };
        Some(TraitDefinition::get(name, self.compiler)?.effect_generics())
    }

    /// True if `a` is a closed effect row and `b` is a bare unbound type variable, which a trait
    /// dictionary's effect arg `a` may be coerced to without binding it.
    fn is_closed_row_below_open_end(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> bool {
        matches!(b.follow_two(&self.bindings, new_bindings), Type::Variable(_))
            && !self.collect_and_merge_effects(a, new_bindings).iter().any(row::is_open)
    }

    /// If the impl type argument of the trait type `expected` is unknown, return the trait dictionary
    /// type `dictionary` with a fresh impl type argument. Unifying that with `expected` leaves their
    /// impl type arguments unlinked until [Self::resolve_dictionary_coercions].
    pub(super) fn unlinked_dictionary_type(&self, dictionary: &Type, expected: &Type) -> Option<Type> {
        let expected_impl = expected.impl_arg(&self.bindings, self.compiler)?;
        if !matches!(self.follow_type(expected_impl), Type::Variable(_)) {
            return None;
        }
        let index = self.trait_definition(dictionary)?.impl_generic_index();
        self.follow_type(dictionary).with_arg(index, self.next_type_variable())
    }

    /// Record that `expr` of the dictionary type `actual` is coerced to the trait type `expected`
    pub(super) fn record_dictionary_coercion(&mut self, expr: ExprId, actual: &Type, expected: &Type) {
        let item = self.current_item.expect("dictionary coercions are within an item");
        let pending = PendingDictionaryCoercion { actual: actual.clone(), expected: expected.clone() };
        self.pending_dictionary_coercions.insert((item, expr), pending);
    }

    /// Decide how each dictionary coerced within the SCC is adapted, linking the impl type
    /// arguments left unlinked by [Self::coerce_dictionary].
    pub(super) fn resolve_dictionary_coercions(&mut self) {
        let previous = self.current_item;
        for ((item, expr), pending) in std::mem::take(&mut self.pending_dictionary_coercions) {
            self.current_item = Some(item);
            self.resolve_dictionary_coercion(expr, pending);
        }
        self.current_item = previous;
    }

    /// Decide how the dictionary `expr` of the current item is adapted. The item has already been
    /// checked, so only state shared by the whole SCC is available: the item's own state, such as
    /// its type maps and implicit scopes, is gone.
    fn resolve_dictionary_coercion(&mut self, expr: ExprId, pending: PendingDictionaryCoercion) {
        let (Some(actual_impl), Some(expected_impl)) =
            (self.impl_arg(&pending.actual), self.impl_arg(&pending.expected))
        else {
            return;
        };

        if !self.effect_args_widened(&pending.actual, &pending.expected) {
            self.unify(&actual_impl, &expected_impl, TypeErrorKind::General, expr);
            return;
        }

        let actual = pending.actual.follow_all(&self.bindings);
        let expected = pending.expected.follow_all(&self.bindings);
        match self.plan_dictionary_coercion(&actual, &expected, true) {
            Ok(coercion) => self.current_extended_context_mut().insert_dictionary_coercion(expr, expected, coercion),
            Err(error) => {
                let field = match error {
                    CoercionError::UnknownImpl => None,
                    CoercionError::Field(field) => Some(field),
                };
                let (actual, expected) = self.type_pair_to_error_strings(&actual, &expected);
                let location = expr.locate(self);
                self.compiler.accumulate(Diagnostic::CannotWidenDictionaryEffects {
                    actual,
                    expected,
                    field,
                    location,
                });
            },
        }
    }

    /// The impl type argument of a trait dictionary type, see [crate::parser::cst::TypeDefinitionKind::Trait]
    pub(super) fn impl_arg(&self, dictionary: &Type) -> Option<Type> {
        dictionary.impl_arg(&self.bindings, self.compiler).cloned()
    }

    /// True if any effect arg of the trait dictionary type `actual` differs from that of `expected`
    fn effect_args_widened(&self, actual: &Type, expected: &Type) -> bool {
        let (Type::Application(_, actual_args), Some((trait_, Some(expected_args)))) =
            (self.follow_type(actual), expected.trait_application(&self.bindings, self.compiler))
        else {
            return false;
        };
        let mut args = trait_.effect_generics().into_iter().zip(actual_args.iter().zip(expected_args));
        args.any(|(is_effect, (actual_arg, expected_arg))| is_effect && !self.effect_rows_identical(actual_arg, expected_arg))
    }

    /// Decide how to adapt a dictionary of type `actual` to the type `expected` with wider effect args.
    /// If `allow_wrapping` is set and the impl the dictionary was created from is unknown, `expected`'s
    /// impl type argument is bound to `actual`. See [CoercionSource::Wrapped].
    fn plan_dictionary_coercion(
        &mut self, actual: &Type, expected: &Type, allow_wrapping: bool,
    ) -> Result<DictionaryCoercion, CoercionError> {
        let members = self.plan_member_coercions(actual, expected)?;
        let (Some(actual_impl), Some(expected_impl)) = (self.impl_arg(actual), self.impl_arg(expected)) else {
            return Err(CoercionError::UnknownImpl);
        };
        let expected_impl = expected_impl.follow_all(&self.bindings);
        let impl_unconstrained = matches!(expected_impl, Type::Variable(_)) && expected_impl != actual_impl;

        // An impl in the same SCC has no generalized type to instantiate yet, so a dictionary created
        // from one is wrapped as if its impl were unknown. Since only a dictionary created from a known
        // impl can be coerced as another dictionary's field, such a field can't be widened.
        let known_impl = actual_impl.impl_data_name(&self.bindings).filter(|name| !self.item_types.contains_key(name));
        if let Some(impl_name) = known_impl
            && (impl_unconstrained || expected_impl == actual_impl)
        {
            self.bind_impl_arg(&expected_impl, &actual_impl)?;
            let bindings = self.impl_instantiation(impl_name, actual);
            Ok(DictionaryCoercion { source: CoercionSource::Direct { impl_name, bindings }, members })
        } else if allow_wrapping && impl_unconstrained {
            self.bind_impl_arg(&expected_impl, actual)?;
            Ok(DictionaryCoercion { source: CoercionSource::Wrapped, members })
        } else {
            Err(CoercionError::UnknownImpl)
        }
    }

    /// Bind the impl type argument `expected_impl` of a coerced dictionary's type to `impl_type`
    fn bind_impl_arg(&mut self, expected_impl: &Type, impl_type: &Type) -> Result<(), CoercionError> {
        let bindings = self.try_unify(expected_impl, impl_type).map_err(|()| CoercionError::UnknownImpl)?;
        self.bindings.extend(bindings);
        Ok(())
    }

    /// Methods are always adapted when their effects change but constants can't be, with the
    /// exception of dictionaries created from a known impl, which are coerced recursively.
    fn plan_member_coercions(&mut self, actual: &Type, expected: &Type) -> Result<Vec<MemberCoercion>, CoercionError> {
        let Some(definition) = self.trait_definition(actual) else { return Ok(Vec::new()) };
        let methods = definition.method_flags(self.compiler);
        let mut members = mapvec(&methods, |is_method| match is_method {
            true => MemberCoercion::Method,
            false => MemberCoercion::Constant,
        });

        let actual_fields = self.get_field_types(actual, None);
        let expected_fields = self.get_field_types(expected, None);

        for (name, (actual_field, index)) in actual_fields {
            let Some((expected_field, _)) = expected_fields.get(&name) else { continue };
            if methods.get(index as usize) == Some(true) || self.types_identical(&actual_field, expected_field) {
                continue;
            }
            if !self.trait_has_effect_args(expected_field) {
                return Err(CoercionError::Field(name));
            }
            let coercion = self.plan_dictionary_coercion(&actual_field, expected_field, false);
            members[index as usize] = MemberCoercion::Nested(coercion.map_err(|_| CoercionError::Field(name))?);
        }
        Ok(members)
    }

    /// True if `a` and `b` are the same type without binding any type variables
    fn types_identical(&self, a: &Type, b: &Type) -> bool {
        let mut bindings = TypeBindings::default();
        self.subtype(a, b, Variance::Invariant, RowMode::Exact, &mut bindings).is_ok() && bindings.is_empty()
    }

    /// The generic bindings to instantiate the impl `impl_name` with to create a dictionary of type
    /// `dictionary`. The impl must be outside of this SCC so that it has a generalized type.
    fn impl_instantiation(&mut self, impl_name: TopLevelName, dictionary: &Type) -> Vec<Type> {
        let (impl_type, bindings) = self.instantiate_unrecorded(GetType(impl_name).get(self.compiler));
        let dictionary_type = impl_type.return_type().unwrap_or(&impl_type).clone();
        if let Ok(new_bindings) = self.try_unify(&dictionary_type, dictionary) {
            self.bindings.extend(new_bindings);
        }
        mapvec(bindings.unwrap_or_default(), |binding| binding.follow_all(&self.bindings))
    }
}

/// The pending dictionary coercions of each item in an SCC
pub(super) type PendingDictionaryCoercions = BTreeMap<(TopLevelId, ExprId), PendingDictionaryCoercion>;
