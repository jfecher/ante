//! patterns.rs handles compilation of Match expressions into decision trees
//!
//! This entire file is adapted from https://github.com/yorickpeterse/pattern-matching-in-rust/tree/main/jacobs2021

use std::{borrow::Cow, collections::{BTreeMap, BTreeSet}};

use rustc_hash::FxHashMap;

use crate::{
    diagnostics::{Diagnostic, Location},
    iterator_extensions::{btree_map, try_vecmap, vecmap},
    name_resolution::{builtin::Builtin, Origin},
    parser::{
        cst::{Literal, Pattern},
        ids::{ExprId, NameId, PathId, PatternId},
    },
    type_inference::{
        type_id::TypeId,
        types::{PrimitiveType, Type},
        TypeChecker,
    },
};

const WILDCARD_PATTERN: &str = "_";

struct MatchCompiler<'local, 'db> {
    checker: &'local TypeChecker<'local, 'db>,

    has_missing_cases: bool,

    /// This is a BTreeMap for deterministic iteration later on
    unreachable_cases: BTreeMap<RowBody, Location>,
}

#[derive(PartialEq, Eq)]
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

#[derive(PartialEq, Eq)]
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
#[derive(Debug, Clone)]
enum Pattern {
    /// A pattern checking for a tag and possibly binding variables such as `Some(42)`
    Constructor(Constructor, Vec<Pattern>),

    /// An integer literal pattern such as `4` or `12345`
    /// TODO: Support negative literals
    Int(u64),

    /// A pattern binding a variable such as `a` or `_`
    Variable(PathId),

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
            Constructor::False
            | Constructor::Int(_)
            | Constructor::Unit
            | Constructor::Range(_, _) => 0,
            Constructor::True => 1,
            Constructor::Variant(_, index) => *index,
        }
    }
}

#[derive(Clone)]
struct Column {
    variable_to_match: PathId,
    pattern: PatternId,
}

impl Column {
    fn new(variable_to_match: PathId, pattern: PatternId) -> Self {
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
        &mut self, variable_to_match: PathId, rules: Vec<(PatternId, ExprId)>, pattern_type: TypeId, location: Location,
    ) -> DecisionTree {
        let rows = vecmap(rules, |(pattern, branch)| {
            let pattern_location = self.current_context().pattern_locations[pattern].clone();
            let columns = vec![Column::new(variable_to_match, pattern)];
            let guard = None;
            Row::new(columns, guard, branch, pattern_location)
        });

        MatchCompiler::run(self, rows, pattern_type, location)
    }
}

impl<'local, 'db> MatchCompiler<'local, 'db> {
    fn run(
        checker: &'local TypeChecker<'local, 'db>, rows: Vec<Row>, pattern_type: TypeId, location: Location,
    ) -> DecisionTree {
        let mut compiler = Self {
            checker,
            has_missing_cases: false,
            unreachable_cases: rows.iter().map(|row| (row.body, row.location.clone())).collect(),
        };

        let tree = compiler.compile_rows(rows).unwrap_or_else(|error| {
            checker.compiler.accumulate(error);
            DecisionTree::Failure { missing_case: false }
        });

        if compiler.has_missing_cases {
            compiler.issue_missing_cases_error(&tree, pattern_type, location);
        }

        if !compiler.unreachable_cases.is_empty() {
            compiler.issue_unreachable_cases_warning();
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
        let location = self.parse_context.path_locations[branch_var].clone();

        let definition_type = self.elaborator.interner.definition_type(branch_var);
        match definition_type.follow_bindings_shallow().into_owned() {
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
            Type::Application(constructor, arguments) => match types.get(constructor) {
                Type::Primitive(PrimitiveType::Pair) => {
                    let field_variables = self.fresh_match_variables(arguments.clone(), location);
                    let cases = vec![(Constructor::struct_(TypeId::PAIR), field_variables, Vec::new())];
                    let (cases, fallback) = self.compile_constructor_cases(rows, branch_var, cases)?;
                    Ok(DecisionTree::Switch(branch_var, cases, fallback))
                },
                Type::UserDefined(origin) => {
                    self.compile_userdefined_cases(rows, branch_var, origin, &arguments, location)
                },
                _ => Err(Diagnostic::CannotMatchOnType { typ, location }),
            },
            Type::UserDefined(origin) => self.compile_userdefined_cases(rows, branch_var, origin, &[], location),
            Type::Generic(_) | Type::Variable(_) | Type::Function(_) | Type::Reference(..) => {
                Err(Diagnostic::CannotMatchOnType { typ, location })
            },
        }
    }

    fn compile_userdefined_cases(
        &self, rows: Vec<Row>, branch_var: PathId, origin: Origin, generics: &[TypeId], location: Location,
    ) -> Result<DecisionTree, Diagnostic> {
        let def = type_def.borrow();
        if let Some(variants) = def.get_variants(&generics) {
            drop(def);
            let typ = Type::DataType(type_def, generics);

            let cases = vecmap(variants.iter().enumerate(), |(idx, (_name, args))| {
                let constructor = Constructor::Variant(typ.clone(), idx);
                let args = self.fresh_match_variables(args.clone(), location);
                (constructor, args, Vec::new())
            });

            let (cases, fallback) = self.compile_constructor_cases(rows, branch_var, cases)?;
            Ok(DecisionTree::Switch(branch_var, cases, fallback))
        } else if let Some(fields) = def.get_fields(&generics) {
            drop(def);
            let typ = Type::DataType(type_def, generics);

            let fields = vecmap(fields, |(_name, typ, _)| typ);
            let constructor = Constructor::struct_(typ);
            let field_variables = self.fresh_match_variables(fields, location);
            let cases = vec![(constructor, field_variables, Vec::new())];
            let (cases, fallback) = self.compile_constructor_cases(rows, branch_var, cases)?;
            Ok(DecisionTree::Switch(branch_var, cases, fallback))
        } else {
            drop(def);
            let typ = Type::DataType(type_def, generics);
            Err(Diagnostic::CannotMatchOnType { typ, location })
        }
    }

    fn fresh_match_variables(&mut self, variable_types: Vec<TypeId>, location: Location) -> Vec<PathId> {
        vecmap(variable_types.into_iter().enumerate(), |(index, typ)| {
            self.fresh_match_variable(index, typ, location.clone())
        })
    }

    fn fresh_match_variable(&mut self, index: usize, variable_type: TypeId, location: Location) -> PathId {
        let name = format!("internal_match_variable_{index}");
        let kind = DefinitionKind::Local(None);
        let id = self.elaborator.interner.push_definition(name, false, false, kind, location);
        self.elaborator.interner.push_definition_type(id, variable_type);
        id
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
                let (key, cons) = match &self.parse_context.patterns[col.pattern] {
                    Pattern::Literal(Literal::Integer(val, _kind)) => ((*val, *val), Constructor::Int(*val)),
                    Pattern::Range(start, stop) => ((*start, *end), Constructor::Range(*start, *end)),
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
                if let Pattern::Constructor(constructor, args) = &self.parse_context.patterns[col.pattern] {
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
                if let Pattern::Variable(variable) = &self.parse_context.patterns[col.pattern] {
                    row.body = self.let_binding(*variable, col.variable_to_match, row.body);
                    false
                } else {
                    true
                }
            });
        }
    }

    /// Creates:
    /// `{ let <variable> = <rhs>; <body> }`
    fn let_binding(&mut self, variable: NameId, rhs: PathId, body: ExprId) -> ExprId {
        let location = self.elaborator.interner.definition(rhs).location;

        let r#type = self.elaborator.interner.definition_type(variable);
        let rhs_type = self.elaborator.interner.definition_type(rhs);
        let variable = HirIdent::non_trait_method(variable, location);

        let rhs = HirExpression::Ident(HirIdent::non_trait_method(rhs, location), None);
        let rhs = self.elaborator.interner.push_expr(rhs);
        self.elaborator.interner.push_expr_type(rhs, rhs_type);
        self.elaborator.interner.push_expr_location(rhs, location);

        let let_ = HirStatement::Let(HirLetStatement {
            pattern: HirPattern::Identifier(variable),
            r#type,
            expression: rhs,
            attributes: Vec::new(),
            comptime: false,
            is_global_let: false,
        });

        let body_type = self.elaborator.interner.id_type(body);
        let let_ = self.elaborator.interner.push_stmt(let_);
        let body = self.elaborator.interner.push_stmt(HirStatement::Expression(body));

        self.elaborator.interner.push_stmt_location(let_, location);
        self.elaborator.interner.push_stmt_location(body, location);

        let block = HirExpression::Block(HirBlockExpression { statements: vec![let_, body] });
        let block = self.elaborator.interner.push_expr(block);
        self.elaborator.interner.push_expr_type(block, body_type);
        self.elaborator.interner.push_expr_location(block, location);
        block
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
        self.find_missing_values(tree, &mut Default::default(), &mut cases, starting_id);

        // It's possible to trigger this matching on an empty enum like `enum Void {}`
        if !cases.is_empty() {
            self.checker.compiler.accumulate(Diagnostic::MissingCases { cases, location });
        }
    }

    /// Issue a missing cases error if necessary for the given type, assuming that no
    /// case of the type is covered. This is the case for empty matches `match foo {}`.
    /// Note that this is expected not to error if the given type is an enum with zero variants.
    fn issue_missing_cases_error_for_type(&mut self, type_matched_on: TypeId, location: Location) {
        let typ = self.checker.follow_type(type_matched_on);
        if let Type::UserDefined(shared, generics) = typ {
            if let Some(variants) = shared.borrow().get_variants(generics) {
                let cases: BTreeSet<_> = variants.into_iter().map(|(name, _)| name).collect();
                if !cases.is_empty() {
                    self.checker.compiler.accumulate(Diagnostic::MissingCases { cases, location });
                }
                return;
            }
        }
        let typ = self.checker.type_to_string(type_matched_on);
        self.checker.compiler.accumulate(Diagnostic::MissingManyCases { typ, location });
    }

    fn find_missing_values(
        &self, tree: &DecisionTree, env: &mut FxHashMap<PathId, (String, Vec<PathId>)>,
        missing_cases: &mut BTreeSet<String>, starting_id: PathId,
    ) {
        match tree {
            DecisionTree::Success(_) | DecisionTree::Failure { missing_case: false } => (),
            DecisionTree::Guard { else_, .. } => {
                self.find_missing_values(else_, env, missing_cases, starting_id);
            },
            DecisionTree::Failure { missing_case: true } => {
                let case = Self::construct_missing_case(starting_id, env);
                missing_cases.insert(case);
            },
            DecisionTree::Switch(variable, cases, else_case) => {
                for case in cases {
                    let name = self.constructor_string(&case.constructor).to_string();
                    env.insert(*variable, (name, case.arguments.clone()));
                    self.find_missing_values(&case.body, env, missing_cases, starting_id);
                }

                if let Some(else_case) = else_case {
                    let typ = self.checker.path_types[variable];

                    for case in self.missing_cases(cases, typ) {
                        env.insert(*variable, case);
                        self.find_missing_values(else_case, env, missing_cases, starting_id);
                    }
                }

                env.remove(variable);
            },
        }
    }

    fn missing_cases(&self, cases: &[Case], typ: TypeId) -> Vec<(String, Vec<PathId>)> {
        // We expect `cases` to come from a `Switch` which should always have
        // at least 2 cases, otherwise it should be a Success or Failure node.
        let first = &cases[0];

        if matches!(&first.constructor, Constructor::Int(_) | Constructor::Range(..)) {
            return self.missing_integer_cases(cases, typ);
        }

        let all_constructors = self.all_constructors(&first.constructor);
        let mut all_constructors = btree_map(all_constructors, |(constructor, arg_count)| (constructor, arg_count));

        for case in cases {
            all_constructors.remove(&case.constructor);
        }

        vecmap(all_constructors, |(constructor, arg_count)| {
            // Safety: this id should only be used in `env` of `find_missing_values` which
            //         only uses it for display and defaults to "_" on unknown ids.
            let args = vecmap(0..arg_count, |_| PathId::dummy_id());
            (self.constructor_string(&constructor), args)
        })
    }

    fn missing_integer_cases(&self, cases: &[Case], typ: TypeId) -> Vec<(String, Vec<PathId>)> {
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

    fn construct_missing_case(starting_id: PathId, env: &FxHashMap<PathId, (String, Vec<PathId>)>) -> String {
        let Some((constructor, arguments)) = env.get(&starting_id) else {
            return WILDCARD_PATTERN.to_string();
        };

        let no_arguments = arguments.is_empty();

        let args = vecmap(arguments, |arg| Self::construct_missing_case(*arg, env)).join(", ");

        if no_arguments {
            constructor.clone()
        } else {
            format!("{constructor}({args})")
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
        } else {
            unreachable!("Non-struct or enum datatype")
        }
    }

    /// Return all the constructors of the result type of the given constructor. Intended to be used
    /// for error reporting in cases where there are at least 2 constructors.
    pub(crate) fn all_constructors(&self, constructor: &Constructor) -> Vec<(Constructor, /*arg count:*/ usize)> {
        match constructor {
            Constructor::True | Constructor::False => {
                vec![(Constructor::True, 0), (Constructor::False, 0)]
            }
            Constructor::Unit => vec![(Constructor::Unit, 0)],
            Constructor::Variant(typ, _) => {
                let typ = self.checker.follow_type(*typ);
                let Type::UserDefined(origin) = &typ else {
                    unreachable!(
                        "Constructor::Variant should have a DataType type, but found {typ:?}"
                    );
                };

                let def_ref = def.borrow();
                if let Some(variants) = def_ref.get_variants(generics) {
                    vecmap(variants.into_iter().enumerate(), |(i, (_, fields))| {
                        (Constructor::Variant(typ.clone(), i), fields.len())
                    })
                } else
                /* def is a struct */
                {
                    let field_count = def_ref.fields_raw().map(|fields| fields.len()).unwrap_or(0);
                    vec![(Constructor::Variant(typ.clone(), 0), field_count)]
                }
            }

            // Nothing great to return for these
            Constructor::Int(_) | Constructor::Range(..) => Vec::new(),
        }
    }

    fn classify_type(&self, typ: TypeId) -> UserDefinedTypeKind {
        match self.checker.follow_type(typ) {
            other @ Type::Application(constructor, arguments) => {
                match self.checker.follow_type(*constructor) {
                    Type::UserDefined(origin) => self.classify_user_defined(origin, arguments),
                    _ => UserDefinedTypeKind::NotUserDefined(other),
                }
            },
            Type::UserDefined(origin) => self.classify_user_defined(origin, &[]),
            other => UserDefinedTypeKind::NotUserDefined(other),
        }
    }

    fn classify_user_defined(&self, origin: &Origin, arguments: &[TypeId]) -> UserDefinedTypeKind {
        match origin {
            Origin::TopLevelDefinition(top_level_name) => todo!(),
            Origin::Builtin(builtin) => todo!(),
            Origin::Local(_) => unreachable!(),
            Origin::TypeResolution => unreachable!(),
        }
    }
}

/// Not to be confused with the "kind" of a type (type of a type), this
/// is meant to classify user-defined types into product types or sum types.
enum UserDefinedTypeKind<'a> {
    NotUserDefined(&'a Type),
    Product,
    Sum,
}
