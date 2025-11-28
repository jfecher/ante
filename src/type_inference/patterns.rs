//! patterns.rs handles compilation of Match expressions into decision trees
//!
//! This entire file is adapted from https://github.com/yorickpeterse/pattern-matching-in-rust/tree/main/jacobs2021

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet}, sync::Arc,
};

use rustc_hash::FxHashMap;

use crate::{
    diagnostics::{Diagnostic, Location, UnimplementedItem}, incremental::{GetItem, Resolve}, iterator_extensions::{btree_map, opt_vecmap, try_vecmap, vecmap}, name_resolution::Origin, parser::{
        cst::{self, Literal, Path, TopLevelItem},
        ids::{ExprId, NameId, PathId, PatternId, TopLevelName},
    }, type_inference::{
        TypeChecker, type_id::TypeId, types::{PrimitiveType, Type}
    }
};

const WILDCARD_PATTERN: &str = "_";

struct MatchCompiler<'tc, 'local, 'db> {
    checker: &'tc mut TypeChecker<'local, 'db>,

    has_missing_cases: bool,

    /// This is a BTreeMap for deterministic iteration later on
    unreachable_cases: BTreeMap<RowBody, Location>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecisionTree {
    /// Match succeeded, jump directly to the given branch
    Success(ExprId),

    Failure {
        missing_case: bool,
    },

    Guard {
        condition: ExprId,
        then: ExprId,
        else_: Box<DecisionTree>,
    },

    Switch(PathId, Vec<Case>, Option<Box<DecisionTree>>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Case {
    pub constructor: Constructor,
    pub arguments: Vec<PathId>,
    pub body: DecisionTree,
}

impl Case {
    pub fn new(constructor: Constructor, arguments: Vec<PathId>, body: DecisionTree) -> Self {
        Self { constructor, arguments, body }
    }
}

/// Anything that can appear before the `=>` in a match rule.
///
/// This form is a bit easier to work with than a [cst::Pattern].
#[derive(Debug, Clone)]
enum Pattern {
    /// A pattern checking for a tag and possibly binding variables such as `Some(42)`
    Constructor(Constructor, Vec<Pattern>),

    /// A pattern binding a variable such as `a` or `_`
    Variable(NameId),

    /// Multiple patterns combined with `|` where we should match this pattern if any
    /// constituent pattern matches. e.g. `Some(3) | None` or `Some(1) | Some(2) | None`
    #[allow(unused)]
    Or(Vec<Pattern>),

    /// An integer range pattern such as `1..20` which will match any integer n such that
    /// 1 <= n < 20.
    #[allow(unused)]
    Range(u64, u64),

    /// An error occurred while translating this pattern. This Pattern kind always translates
    /// to a Fail branch in the decision tree, although the compiler is expected to halt
    /// with errors before execution.
    Error,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Constructor {
    True,
    False,
    Unit,
    Int(u64),
    Variant(TypeId, /* variant index */ usize),

    /// Inclusive (!) range between start and end
    Range(u64, u64),
}

impl Constructor {
    /// Structs are treated as a single-variant enum
    fn struct_(id: TypeId) -> Constructor {
        Constructor::Variant(id, 0)
    }

    fn variant_index(&self) -> usize {
        match self {
            Constructor::False | Constructor::Int(_) | Constructor::Unit | Constructor::Range(_, _) => 0,
            Constructor::True => 1,
            Constructor::Variant(_, index) => *index,
        }
    }
}

#[derive(Clone)]
struct Column {
    variable_to_match: PathId,
    pattern: Pattern,
}

impl Column {
    fn new(variable_to_match: PathId, pattern: Pattern) -> Self {
        Column { variable_to_match, pattern }
    }
}

#[derive(Clone)]
pub(super) struct Row {
    columns: Vec<Column>,
    guard: Option<RowBody>,
    body: RowBody,
    original_body: RowBody,
    location: Location,
}

type RowBody = ExprId;

impl Row {
    fn new(columns: Vec<Column>, guard: Option<RowBody>, body: RowBody, location: Location) -> Row {
        Row { columns, guard, body, original_body: body, location }
    }
}

impl Row {
    fn remove_column(&mut self, variable: PathId) -> Option<Column> {
        self.columns.iter().position(|c| c.variable_to_match == variable).map(|idx| self.columns.remove(idx))
    }
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Creates a decision tree from the given match expression
    pub(super) fn compile_decision_tree(
        &mut self, variable_to_match: PathId, rules: &[(PatternId, ExprId)], pattern_type: TypeId, location: Location,
    ) -> Option<DecisionTree> {
        let rows = opt_vecmap(rules, |(pattern, branch)| {
            let pattern_location = self.current_context().pattern_locations[*pattern].clone();
            let pattern = self.convert_pattern(*pattern)?;
            let columns = vec![Column::new(variable_to_match, pattern)];
            let guard = None;
            Some(Row::new(columns, guard, *branch, pattern_location))
        })?;

        Some(MatchCompiler::run(self, rows, pattern_type, location))
    }

    /// Converts a [cst::Pattern] into an easier form usable by the match compiler.
    ///
    /// If the given pattern is unable to be converted, an error is issued and None is returned.
    fn convert_pattern(&mut self, pattern: PatternId) -> Option<Pattern> {
        Some(match &self.current_context().patterns[pattern] {
            cst::Pattern::Error => Pattern::Error,
            cst::Pattern::Variable(name_id) => Pattern::Variable(*name_id),
            cst::Pattern::Literal(Literal::Unit) => Pattern::Constructor(Constructor::Unit, Vec::new()),
            cst::Pattern::Literal(Literal::Bool(value)) => {
                let constructor = if *value { Constructor::True } else { Constructor::False };
                Pattern::Constructor(constructor, Vec::new())
            },
            cst::Pattern::Literal(Literal::Integer(value, _kind)) => {
                Pattern::Constructor(Constructor::Int(*value), Vec::new())
            },
            cst::Pattern::Literal(_) => {
                let location = self.current_context().pattern_locations[pattern].clone();
                self.compiler.accumulate(Diagnostic::InvalidPattern { location });
                return None;
            },
            cst::Pattern::Constructor(path_id, arguments) => {
                let constructor = self.path_to_constructor(*path_id)?;
                let arguments = opt_vecmap(arguments, |argument| self.convert_pattern(*argument))?;
                Pattern::Constructor(constructor, arguments)
            },
            cst::Pattern::TypeAnnotation(pattern, _) => return self.convert_pattern(*pattern),
            cst::Pattern::MethodName { .. } => {
                let location = self.current_context().pattern_locations[pattern].clone();
                self.compiler.accumulate(Diagnostic::InvalidPattern { location });
                return None;
            },
        })
    }

    /// Try to convert the given path to a constructor, issuing an error and returning [None] on
    /// failure.
    fn path_to_constructor(&mut self, path: PathId) -> Option<Constructor> {
        let mut origin = &self.current_resolve().path_origins[&path];
        // Most times we can immediately grab the origin, but in the case of
        // Origin::TypeResolution we need to grab it from another map. A loop
        // is used here instead of recursion to prevent infinite recursion in the
        // case of a bug elsewhere in the compiler.
        for _ in 0..2 {
            match origin {
                Origin::TopLevelDefinition(top_level_name) => {
                    let item = GetItem(top_level_name.top_level_item).get(self.compiler);
                    return self.item_to_constructor(&item.0, top_level_name.local_name_id, path);
                },
                Origin::Local(_) => unreachable!("Origin::Local used in path_to_constructor"),
                Origin::TypeResolution => {
                    // The type checker should hold the origin of paths that require type resolution
                    origin = self.path_origins.get(&path)?;
                },
                Origin::Builtin(builtin) => {
                    if let Some((type_id, variant_index)) = builtin.constructor() {
                        return Some(Constructor::Variant(type_id, variant_index));
                    } else {
                        let location = self.current_context().path_locations[path].clone();
                        self.compiler.accumulate(Diagnostic::InvalidPattern { location });
                        return None;
                    }
                },
            }
        }
        panic!("Unable to find origin of path in path_to_constructor!")
    }

    /// Given a source item (must be a TypeDefinition), and a name in that item, return the constructor
    /// corresponding to that name. The given `NameId` should be local to the given `TopLevelItem`.
    /// The `PathId` provided should be the path used in the match expression - its location is
    /// used for error messages.
    fn item_to_constructor(&mut self, item: &TopLevelItem, name: NameId, path: PathId) -> Option<Constructor> {
        let cst::TopLevelItemKind::TypeDefinition(type_definition) = &item.kind else {
            return None;
        };

        let variant_index = match &type_definition.body {
            cst::TypeDefinitionBody::Error => return None,
            // A struct only has 1 constructor, and its name should be the only NameId externally
            // visible.
            cst::TypeDefinitionBody::Struct(_) => 0,
            cst::TypeDefinitionBody::Enum(variants) => {
                let result =
                    variants.iter().enumerate().find_map(|(i, (variant_name, _))| (*variant_name == name).then_some(i));
                // The only other name visible within the enum should be the type name.
                // Issue an error suggesting using a constructor instead.
                if result.is_none() {
                    self.issue_constructor_expected_found_type_error(item, variants, type_definition.name, path);
                }
                result?
            },
            cst::TypeDefinitionBody::Alias(_) => {
                let location = self.item_contexts[&item.id].1.name_locations[name].clone();
                self.compiler.accumulate(Diagnostic::Unimplemented { item: UnimplementedItem::TypeAlias, location });
                return None;
            },
        };

        let type_name = TopLevelName::named(item.id, type_definition.name);
        let type_id = self.types.get_or_insert_type(Type::UserDefined(Origin::TopLevelDefinition(type_name)));
        Some(Constructor::Variant(type_id, variant_index))
    }

    /// Issue a [Diagnostic::ConstructorExpectedFoundType] error using the location of the given [PathId].
    /// - `type_name` is local to `item` and is included as part of the error message.
    /// - `path` is local to the match and is used for its location.
    fn issue_constructor_expected_found_type_error(
        &self, item: &TopLevelItem, variants: &[(NameId, Vec<cst::Type>)], type_name: NameId, path: PathId,
    ) {
        let item_context = &self.item_contexts[&item.id].1;
        let constructor_names = vecmap(variants.iter().take(2), |(name, _)| item_context.names[*name].clone());

        let type_name = item_context.names[type_name].clone();

        let location = self.current_context().path_locations[path].clone();
        self.compiler.accumulate(Diagnostic::ConstructorExpectedFoundType { type_name, constructor_names, location });
    }

    pub(super) fn fresh_match_variable(&mut self, index: usize, variable_type: TypeId, location: Location) -> PathId {
        let name = Path { components: vec![(format!("internal_match_variable_{index}"), location.clone())] };
        let id = self.push_path(name, location);
        self.path_types.insert(id, variable_type);
        id
    }

    /// Creates:
    /// `<variable> = <rhs>; <body>`
    fn let_binding_with_path(&mut self, variable: NameId, rhs: PathId, body: ExprId) -> ExprId {
        let location = self.current_extended_context().path_location(rhs);
        let rhs_type = self.path_types[&rhs];
        let rhs = self.push_expr(cst::Expr::Variable(rhs)       , rhs_type, location.clone());
        self.let_binding(variable, rhs, body)
    }

    /// Creates:
    /// `<variable> = <rhs>; <body>`
    pub(super) fn let_binding(&mut self, variable: NameId, rhs: ExprId, body: ExprId) -> ExprId {
        let location = self.current_extended_context().expr_location(rhs);
        let body_type = self.expr_types[&body];

        let pattern = cst::Pattern::Variable(variable);
        let pattern = self.push_pattern(pattern, location.clone());

        let definition = cst::Expr::Definition(cst::Definition {
            implicit: false,
            mutable: false,
            pattern,
            rhs,
        });
        let definition = self.push_expr(definition, TypeId::UNIT, location.clone());

        let seq_item = |expr| cst::SequenceItem { comments: Vec::new(), expr };
        let block = cst::Expr::Sequence(vec![seq_item(definition), seq_item(body)]);
        self.push_expr(block, body_type, location)
    }
}

impl<'tc, 'local, 'db> MatchCompiler<'tc, 'local, 'db> {
    fn run(
        checker: &'tc mut TypeChecker<'local, 'db>, rows: Vec<Row>, pattern_type: TypeId, location: Location,
    ) -> DecisionTree {
        let mut matcher = Self {
            checker,
            has_missing_cases: false,
            unreachable_cases: rows.iter().map(|row| (row.body, row.location.clone())).collect(),
        };

        let tree = matcher.compile_rows(rows).unwrap_or_else(|error| {
            matcher.checker.compiler.accumulate(error);
            DecisionTree::Failure { missing_case: false }
        });

        if matcher.has_missing_cases {
            matcher.issue_missing_cases_error(&tree, pattern_type, location);
        }

        if !matcher.unreachable_cases.is_empty() {
            matcher.issue_unreachable_cases_warning();
        }

        tree
    }

    fn compile_rows(&mut self, mut rows: Vec<Row>) -> Result<DecisionTree, Diagnostic> {
        if rows.is_empty() {
            self.has_missing_cases = true;
            return Ok(DecisionTree::Failure { missing_case: true });
        }

        self.push_tests_against_bare_variables(&mut rows);

        // If the first row is a match-all we match it and the remaining rows are ignored.
        if rows.first().is_some_and(|row| row.columns.is_empty()) {
            let row = rows.remove(0);

            return Ok(match row.guard {
                None => {
                    self.unreachable_cases.remove(&row.original_body);
                    DecisionTree::Success(row.body)
                },
                Some(condition) => {
                    let remaining = self.compile_rows(rows)?;
                    DecisionTree::Guard { condition, then: row.body, else_: Box::new(remaining) }
                },
            });
        }

        let branch_var = self.branch_variable(&rows);
        let location = self.checker.current_extended_context().path_location(branch_var);

        let definition_type = self.checker.path_types[&branch_var];
        match self.checker.follow_type(definition_type) {
            Type::Primitive(PrimitiveType::Int(_)) => {
                let (cases, fallback) = self.compile_int_cases(rows, branch_var)?;
                Ok(DecisionTree::Switch(branch_var, cases, Some(fallback)))
            },
            Type::Primitive(PrimitiveType::Bool) => {
                let cases =
                    vec![(Constructor::False, Vec::new(), Vec::new()), (Constructor::True, Vec::new(), Vec::new())];

                let (cases, fallback) = self.compile_constructor_cases(rows, branch_var, cases)?;
                Ok(DecisionTree::Switch(branch_var, cases, fallback))
            },
            Type::Primitive(PrimitiveType::Unit) => {
                let cases = vec![(Constructor::Unit, Vec::new(), Vec::new())];
                let (cases, fallback) = self.compile_constructor_cases(rows, branch_var, cases)?;
                Ok(DecisionTree::Switch(branch_var, cases, fallback))
            },
            Type::Application(constructor, arguments) => match self.checker.follow_type(*constructor) {
                Type::Primitive(PrimitiveType::Pair) => {
                    let field_variables = self.fresh_match_variables(arguments.clone(), location);
                    let cases = vec![(Constructor::struct_(TypeId::PAIR), field_variables, Vec::new())];
                    let (cases, fallback) = self.compile_constructor_cases(rows, branch_var, cases)?;
                    Ok(DecisionTree::Switch(branch_var, cases, fallback))
                },
                Type::UserDefined(origin) => {
                    let origin = *origin;
                    let arguments = arguments.clone();
                    self.compile_userdefined_cases(rows, branch_var, definition_type, origin, &arguments, location)
                },
                _ => {
                    let typ = self.checker.type_to_string(definition_type);
                    Err(Diagnostic::CannotMatchOnType { typ, location })
                },
            },
            Type::UserDefined(origin) => self.compile_userdefined_cases(rows, branch_var, definition_type, *origin, &[], location),
            Type::Generic(_) | Type::Variable(_) | Type::Primitive(_) | Type::Function(_) | Type::Reference(..) => {
                let typ = self.checker.type_to_string(definition_type);
                Err(Diagnostic::CannotMatchOnType { typ, location })
            },
        }
    }

    fn compile_userdefined_cases(
        &mut self, rows: Vec<Row>, branch_var: PathId, type_id: TypeId, origin: Origin, generics: &[TypeId], location: Location,
    ) -> Result<DecisionTree, Diagnostic> {
        match self.classify_type_origin(origin, generics) {
            Some(UserDefinedTypeKind::Sum(variants)) => {
                let cases = vecmap(variants.iter().enumerate(), |(idx, (_name, args))| {
                    let constructor = Constructor::Variant(type_id, idx);
                    let args = self.fresh_match_variables(args.clone(), location.clone());
                    (constructor, args, Vec::new())
                });

                let (cases, fallback) = self.compile_constructor_cases(rows, branch_var, cases)?;
                Ok(DecisionTree::Switch(branch_var, cases, fallback))
            },
            Some(UserDefinedTypeKind::Product(fields)) => {
                let constructor = Constructor::struct_(type_id);
                let field_variables = self.fresh_match_variables(fields, location);
                let cases = vec![(constructor, field_variables, Vec::new())];
                let (cases, fallback) = self.compile_constructor_cases(rows, branch_var, cases)?;
                Ok(DecisionTree::Switch(branch_var, cases, fallback))
            },
            Some(UserDefinedTypeKind::NotUserDefined(typ)) => {
                let typ = self.checker.type_to_string(typ);
                Err(Diagnostic::CannotMatchOnType { typ, location })
            },
            // Name resolution error, assume a relevant diagnostic has already been issued
            None => {
                // Prevent irrelevant unreachable pattern errors
                for row in rows {
                    self.unreachable_cases.remove(&row.original_body);
                }
                Ok(DecisionTree::Failure { missing_case: true })
            },
        }
    }

    fn fresh_match_variables(&mut self, variable_types: Vec<TypeId>, location: Location) -> Vec<PathId> {
        vecmap(variable_types.into_iter().enumerate(), |(index, typ)| {
            self.checker.fresh_match_variable(index, typ, location.clone())
        })
    }

    /// Compiles the cases and fallback cases for integer and range patterns.
    ///
    /// Integers have an infinite number of constructors, so we specialize the
    /// compilation of integer and range patterns.
    fn compile_int_cases(
        &mut self, rows: Vec<Row>, branch_var: PathId,
    ) -> Result<(Vec<Case>, Box<DecisionTree>), Diagnostic> {
        let mut raw_cases: Vec<(Constructor, Vec<PathId>, Vec<Row>)> = Vec::new();
        let mut fallback_rows = Vec::new();
        let mut tested: FxHashMap<(u64, u64), usize> = FxHashMap::default();

        for mut row in rows {
            if let Some(col) = row.remove_column(branch_var) {
                let (key, cons) = match &col.pattern {
                    Pattern::Constructor(Constructor::Int(val), _) => ((*val, *val), Constructor::Int(*val)),
                    Pattern::Range(start, stop) => ((*start, *stop), Constructor::Range(*start, *stop)),
                    // Any other pattern shouldn't have an integer type and we expect a type
                    // check error to already have been issued.
                    _ => continue,
                };

                if let Some(index) = tested.get(&key) {
                    raw_cases[*index].2.push(row);
                    continue;
                }

                tested.insert(key, raw_cases.len());

                let mut rows = fallback_rows.clone();

                rows.push(row);
                raw_cases.push((cons, Vec::new(), rows));
            } else {
                for (_, _, rows) in &mut raw_cases {
                    rows.push(row.clone());
                }

                fallback_rows.push(row);
            }
        }

        let cases = try_vecmap(raw_cases, |(cons, vars, rows)| {
            let rows = self.compile_rows(rows)?;
            Ok::<_, Diagnostic>(Case::new(cons, vars, rows))
        })?;

        Ok((cases, Box::new(self.compile_rows(fallback_rows)?)))
    }

    /// Compiles the cases and sub cases for the constructor located at the
    /// column of the branching variable.
    ///
    /// What exactly this method does may be a bit hard to understand from the
    /// code, as there's simply quite a bit going on. Roughly speaking, it does
    /// the following:
    ///
    /// 1. It takes the column we're branching on (based on the branching
    ///    variable) and removes it from every row.
    /// 2. We add additional columns to this row, if the constructor takes any
    ///    arguments (which we'll handle in a nested match).
    /// 3. We turn the resulting list of rows into a list of cases, then compile
    ///    those into decision (sub) trees.
    ///
    /// If a row didn't include the branching variable, we simply copy that row
    /// into the list of rows for every constructor to test.
    ///
    /// For this to work, the `cases` variable must be prepared such that it has
    /// a triple for every constructor we need to handle. For an ADT with 10
    /// constructors, that means 10 triples. This is needed so this method can
    /// assign the correct sub matches to these constructors.
    ///
    /// Types with infinite constructors (e.g. integers and strings) are handled
    /// separately; they don't need most of this work anyway.
    #[allow(clippy::type_complexity)]
    fn compile_constructor_cases(
        &mut self, rows: Vec<Row>, branch_var: PathId, mut cases: Vec<(Constructor, Vec<PathId>, Vec<Row>)>,
    ) -> Result<(Vec<Case>, Option<Box<DecisionTree>>), Diagnostic> {
        for mut row in rows {
            if let Some(col) = row.remove_column(branch_var) {
                if let Pattern::Constructor(constructor, args) = col.pattern {
                    // TODO: Convert to dedicated pattern enum
                    let idx = constructor.variant_index();
                    let mut cols = row.columns;

                    for (var, pat) in cases[idx].1.iter().zip(args.into_iter()) {
                        cols.push(Column::new(*var, pat));
                    }

                    cases[idx].2.push(Row::new(cols, row.guard, row.body, row.location));
                }
            } else {
                for (_, _, rows) in &mut cases {
                    rows.push(row.clone());
                }
            }
        }

        let cases = try_vecmap(cases, |(cons, vars, rows)| {
            let rows = self.compile_rows(rows)?;
            Ok::<_, Diagnostic>(Case::new(cons, vars, rows))
        })?;

        Ok(Self::deduplicate_cases(cases))
    }

    /// Move any cases with duplicate branches into a shared 'else' branch
    fn deduplicate_cases(mut cases: Vec<Case>) -> (Vec<Case>, Option<Box<DecisionTree>>) {
        let mut else_case = None;
        let mut ending_cases = Vec::with_capacity(cases.len());
        let mut previous_case: Option<Case> = None;

        // Go through each of the cases, looking for duplicates.
        // This is simplified such that the first (consecutive) duplicates
        // we find we move to an else case. Each case afterward is then compared
        // to the else case. This could be improved in a couple ways:
        // - Instead of the the first consecutive duplicates we find, we could
        //   expand the check to find non-consecutive duplicates as well.
        // - We should also ideally move the most duplicated case to the else
        //   case, not just the first duplicated case we find. I suspect in most
        //   actual code snippets these are the same but it could still be nice to guarantee.
        while let Some(case) = cases.pop() {
            if let Some(else_case) = &else_case {
                if case.body == *else_case {
                    // Delete the current case by not pushing it to `ending_cases`
                    continue;
                } else {
                    ending_cases.push(case);
                }
            } else if let Some(previous) = previous_case {
                if case.body == previous.body {
                    // else_case is known to be None here
                    else_case = Some(previous.body);

                    // Delete both previous_case and case
                    previous_case = None;
                    continue;
                } else {
                    previous_case = Some(case);
                    ending_cases.push(previous);
                }
            } else {
                previous_case = Some(case);
            }
        }

        if let Some(case) = previous_case {
            ending_cases.push(case);
        }

        ending_cases.reverse();
        (ending_cases, else_case.map(Box::new))
    }

    /// Return the variable that was referred to the most in `rows`
    fn branch_variable(&mut self, rows: &[Row]) -> PathId {
        let mut counts = FxHashMap::default();

        for row in rows {
            for col in &row.columns {
                *counts.entry(&col.variable_to_match).or_insert(0_usize) += 1;
            }
        }

        rows[0].columns.iter().map(|col| col.variable_to_match).max_by_key(|var| counts[var]).unwrap()
    }

    fn push_tests_against_bare_variables(&mut self, rows: &mut Vec<Row>) {
        for row in rows {
            row.columns.retain(|col| {
                if let Pattern::Variable(variable) = &col.pattern {
                    row.body = self.checker.let_binding_with_path(*variable, col.variable_to_match, row.body);
                    false
                } else {
                    true
                }
            });
        }
    }

    /// Any case that isn't branched to when the match is finished must be covered by another
    /// case and is thus redundant.
    fn issue_unreachable_cases_warning(&mut self) {
        for location in self.unreachable_cases.values().cloned() {
            self.checker.compiler.accumulate(Diagnostic::UnreachableCase { location });
        }
    }

    /// Traverse the resulting DecisionTree to build counter-examples of values which would
    /// not be covered by the match.
    fn issue_missing_cases_error(&mut self, tree: &DecisionTree, type_matched_on: TypeId, location: Location) {
        let starting_id = match tree {
            DecisionTree::Switch(id, ..) => *id,
            _ => return self.issue_missing_cases_error_for_type(type_matched_on, location),
        };

        let mut cases = BTreeSet::new();
        self.find_missing_values(tree, &mut Default::default(), &mut cases, starting_id, &location);

        // It's possible to trigger this matching on an empty enum like `enum Void {}`
        if !cases.is_empty() {
            self.checker.compiler.accumulate(Diagnostic::MissingCases { cases, location });
        }
    }

    /// Issue a missing cases error if necessary for the given type, assuming that no
    /// case of the type is covered. This is the case for empty matches `match foo {}`.
    /// Note that this is expected not to error if the given type is an enum with zero variants.
    fn issue_missing_cases_error_for_type(&mut self, type_matched_on: TypeId, location: Location) {
        match self.classify_type(type_matched_on) {
            Some(UserDefinedTypeKind::Sum(variants)) => {
                if !variants.is_empty() {
                    let cases: BTreeSet<_> = variants.into_iter().map(|(name, _)| {
                        // TODO: These names should be in the variant's resolve data
                        self.checker.current_context().names[name].clone()
                    }).collect();
                    self.checker.compiler.accumulate(Diagnostic::MissingCases { cases, location });
                }
                return;
            }
            Some(UserDefinedTypeKind::NotUserDefined(TypeId::BOOL)) => {
                let cases = vec![Arc::new("false".to_string()), Arc::new("true".to_string())].into_iter().collect();
                self.checker.compiler.accumulate(Diagnostic::MissingCases { cases, location });
                return;
            }
            _ => (),
        }
        let typ = self.checker.type_to_string(type_matched_on);
        self.checker.compiler.accumulate(Diagnostic::MissingManyCases { typ, location });
    }

    fn find_missing_values(
        &mut self, tree: &DecisionTree, env: &mut FxHashMap<PathId, (String, Vec<Option<PathId>>)>,
        missing_cases: &mut BTreeSet<Arc<String>>, starting_id: PathId, location: &Location
    ) {
        match tree {
            DecisionTree::Success(_) | DecisionTree::Failure { missing_case: false } => (),
            DecisionTree::Guard { else_, .. } => {
                self.find_missing_values(else_, env, missing_cases, starting_id, location);
            },
            DecisionTree::Failure { missing_case: true } => {
                let case = Self::construct_missing_case(Some(starting_id), env);
                missing_cases.insert(Arc::new(case));
            },
            DecisionTree::Switch(variable, cases, else_case) => {
                for case in cases {
                    let name = self.constructor_string(&case.constructor).to_string();
                    env.insert(*variable, (name, vecmap(case.arguments.iter().copied(), Some)));
                    self.find_missing_values(&case.body, env, missing_cases, starting_id, location);
                }

                if let Some(else_case) = else_case {
                    let typ = self.checker.path_types[variable];

                    for case in self.missing_cases(cases, typ, location) {
                        env.insert(*variable, case);
                        self.find_missing_values(else_case, env, missing_cases, starting_id, location);
                    }
                }

                env.remove(variable);
            },
        }
    }

    fn missing_cases(&mut self, cases: &[Case], typ: TypeId, location: &Location) -> Vec<(String, Vec<Option<PathId>>)> {
        // We expect `cases` to come from a `Switch` which should always have
        // at least 2 cases, otherwise it should be a Success or Failure node.
        let Some(first) = cases.first() else {
            return Vec::new()
        };

        if matches!(&first.constructor, Constructor::Int(_) | Constructor::Range(..)) {
            return self.missing_integer_cases(cases, typ);
        }

        let all_constructors = self.all_constructors(&first.constructor, location);
        let mut all_constructors = btree_map(all_constructors, |(constructor, arg_count)| (constructor, arg_count));

        for case in cases {
            all_constructors.remove(&case.constructor);
        }

        vecmap(all_constructors, |(constructor, arg_count)| {
            let args = vec![None; arg_count];
            (self.constructor_string(&constructor).into_owned(), args)
        })
    }

    fn missing_integer_cases(&self, cases: &[Case], typ: TypeId) -> Vec<(String, Vec<Option<PathId>>)> {
        // We could give missed cases for field ranges of `0 .. field_modulus` but since the field
        // used in Noir may change we recommend a match-all pattern instead.
        // If the type is a type variable, we don't know exactly which integer type this may
        // resolve to so also just suggest a catch-all in that case.
        if typ.is_integer() || self.type_is_bindable(typ) {
            return vec![(WILDCARD_PATTERN.to_string(), Vec::new())];
        }

        let mut missing_cases = rangemap::RangeInclusiveSet::new();
        missing_cases.insert(u64::MIN..=u64::MAX);

        for case in cases {
            match &case.constructor {
                Constructor::Int(signed_field) => {
                    missing_cases.remove(*signed_field..=*signed_field);
                },
                Constructor::Range(start, end) if start >= end => (),
                Constructor::Range(start, end) => {
                    // Ranges `a..b` in ante are exclusive, so we need to adapt it to an inclusive range
                    missing_cases.remove(*start..=end.saturating_sub(1));
                },
                _ => unreachable!("missing_integer_cases called with non-Int/Range constructor"),
            }
        }

        vecmap(missing_cases, |range| {
            if range.start() == range.end() {
                (format!("{}", range.start()), Vec::new())
            } else {
                (format!("{}..={}", range.start(), range.end()), Vec::new())
            }
        })
    }

    /// True if the type can be bound to (= is an unbound type variable)
    fn type_is_bindable(&self, typ: TypeId) -> bool {
        matches!(self.checker.follow_type(typ), Type::Variable(_))
    }

    fn construct_missing_case(starting_id: Option<PathId>, env: &FxHashMap<PathId, (String, Vec<Option<PathId>>)>) -> String {
        let Some((constructor, arguments)) = starting_id.and_then(|id| env.get(&id)) else {
            return WILDCARD_PATTERN.to_string();
        };

        let args = vecmap(arguments, |arg| Self::construct_missing_case(*arg, env));

        if args.is_empty() {
            constructor.clone()
        } else if constructor == "," {
            format!("{}", args.join(", "))
        } else {
            format!("{constructor} {}", args.join(" "))
        }
    }

    fn constructor_string<'this>(&'this self, constructor: &Constructor) -> Cow<'this, String> {
        Cow::Owned(match constructor {
            Constructor::True => "true".to_string(),
            Constructor::False => "false".to_string(),
            Constructor::Unit => "()".to_string(),
            Constructor::Int(x) => format!("{x}"),
            Constructor::Variant(typ, variant_index) => return self.user_defined_type_name(*typ, *variant_index),
            Constructor::Range(start, end) => format!("{start} .. {end}"),
        })
    }

    fn user_defined_type_name(&self, typ: TypeId, _variant_index: usize) -> Cow<String> {
        if let Type::UserDefined(origin) = self.checker.follow_type(typ) {
            match origin {
                Origin::TopLevelDefinition(top_level_name) => {
                    let item = &self.checker.item_contexts[&top_level_name.top_level_item].1;
                    Cow::Borrowed(item.names[top_level_name.local_name_id].as_ref())
                },
                Origin::Local(name_id) => Cow::Borrowed(self.checker.current_context().names[*name_id].as_ref()),
                Origin::TypeResolution => unreachable!("Types cannot be Origin::TypeResolution"),
                Origin::Builtin(builtin) => Cow::Owned(builtin.to_string()),
            }
        } else if typ == TypeId::PAIR {
            Cow::Owned(",".to_string())
        } else {
            unreachable!("Non-struct or enum datatype: {}", self.checker.type_to_string(typ))
        }
    }

    /// Return all the constructors of the result type of the given constructor. Intended to be used
    /// for error reporting in cases where there are at least 2 constructors.
    pub(crate) fn all_constructors(&mut self, constructor: &Constructor, location: &Location) -> Vec<(Constructor, /*arg count:*/ usize)> {
        match constructor {
            Constructor::True | Constructor::False => {
                vec![(Constructor::True, 0), (Constructor::False, 0)]
            },
            Constructor::Unit => vec![(Constructor::Unit, 0)],
            Constructor::Variant(type_id, _) => {
                match self.classify_type(*type_id) {
                    Some(UserDefinedTypeKind::Product(fields)) => {
                        vec![(Constructor::Variant(*type_id, 0), fields.len())]
                    }
                    Some(UserDefinedTypeKind::Sum(variants)) => {
                        vecmap(variants.into_iter().enumerate(), |(i, (_, fields))| {
                            (Constructor::Variant(*type_id, i), fields.len())
                        })
                    }
                    Some(UserDefinedTypeKind::NotUserDefined(type_id)) => {
                        let typ = self.checker.type_to_string(type_id);
                        let location = location.clone();
                        self.checker.compiler.accumulate(Diagnostic::CannotMatchOnType { typ, location });
                        Vec::new()
                    }
                    None => {
                        unreachable!("Unresolved type encountered in all_constructors")
                    }
                }
            },

            // Nothing great to return for these
            Constructor::Int(_) | Constructor::Range(..) => Vec::new(),
        }
    }

    fn classify_type(&mut self, typ: TypeId) -> Option<UserDefinedTypeKind> {
        match self.checker.follow_type(typ) {
            Type::Application(constructor, arguments) => match self.checker.follow_type(*constructor) {
                Type::UserDefined(origin) => {
                    let origin = *origin;
                    let arguments = arguments.clone();
                    self.classify_type_origin(origin, &arguments)
                },
                _ => Some(UserDefinedTypeKind::NotUserDefined(typ)),
            },
            Type::UserDefined(origin) => self.classify_type_origin(*origin, &[]),
            _ => Some(UserDefinedTypeKind::NotUserDefined(typ)),
        }
    }

    /// Try to return this type's body (its fields or variants).
    /// If we fail to do so, None is returned and no error is issued.
    fn classify_type_origin(&mut self, origin: Origin, arguments: &[TypeId]) -> Option<UserDefinedTypeKind> {
        // Most times we can immediately grab the origin, but in the case of
        // Origin::TypeResolution we need to grab it from another map. A loop
        // is used here instead of recursion to prevent infinite recursion in the
        // case of a bug elsewhere in the compiler.
        match origin {
            Origin::TopLevelDefinition(top_level_name) => {
                let item = GetItem(top_level_name.top_level_item).get(self.checker.compiler);
                self.classify_top_level_item(&item.0, top_level_name.local_name_id)
            },
            Origin::Local(_) => unreachable!("Origin::Local used in path_to_constructor"),
            Origin::TypeResolution => {
                unreachable!("Origin::TypeResolution encountered in classify_type_origin, should be unreachable in a type position")
            },
            Origin::Builtin(builtin) => {
                let fields = builtin.fields(arguments.to_vec())?;
                // There is no built-in sum type
                Some(UserDefinedTypeKind::Product(fields))
            },
        }
    }

    fn classify_top_level_item(&mut self, item: &TopLevelItem, name: NameId) -> Option<UserDefinedTypeKind> {
        let cst::TopLevelItemKind::TypeDefinition(type_definition) = &item.kind else {
            return None;
        };

        if name != type_definition.name {
            return None;
        }

        let resolve = Resolve(item.id).get(self.checker.compiler);
        match &type_definition.body {
            cst::TypeDefinitionBody::Error => None,
            cst::TypeDefinitionBody::Struct(fields) => {
                let fields = vecmap(fields, |(_, typ)| self.checker.convert_foreign_type(typ, &resolve));
                Some(UserDefinedTypeKind::Product(fields))
            },
            cst::TypeDefinitionBody::Enum(variants) => {
                Some(UserDefinedTypeKind::Sum(vecmap(variants, |(name, args)| {
                    let args = vecmap(args, |arg| self.checker.convert_foreign_type(arg, &resolve));
                    (*name, args)
                })))
            },
            cst::TypeDefinitionBody::Alias(typ) => {
                // TODO: Guard against infinite recursion
                let typ = self.checker.convert_foreign_type(typ, &resolve);
                self.classify_type(typ)
            },
        }
    }
}

/// Not to be confused with the "kind" of a type (type of a type), this
/// is meant to classify user-defined types into product types or sum types.
enum UserDefinedTypeKind {
    NotUserDefined(TypeId),
    Product(Vec<TypeId>),
    Sum(Vec<(NameId, Vec<TypeId>)>),
}
