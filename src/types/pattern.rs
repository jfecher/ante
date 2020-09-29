use crate::cache::{ ModuleCache, DefinitionInfoId };
use crate::error::location::{ Location, Locatable };
use crate::parser::ast::{ self, Ast };
use crate::types::pattern::Constructor::*;
use crate::types::{ Type, TypeInfoBody, TypeInfoId };

use std::iter::repeat;
use std::collections::{ HashMap, HashSet };

#[derive(Debug, Copy, Clone, Eq, Hash)]
pub enum VariantTag {
    True,
    False,
    Unit,
    UserDefined(DefinitionInfoId),

    /// This tag signals pattern matching should give up completeness checking
    /// for this constructor. Integers and floats are most notably translated to
    /// Fail rather than attempting to approximate the types' full ranges.
    Fail,
}

impl PartialEq for VariantTag {
    fn eq(&self, other: &VariantTag) -> bool {
        use VariantTag::*;
        match (self, other) {
            (UserDefined(a), UserDefined(b)) => a == b,
            (True, True) => true,
            (False, False) => true,
            (Unit, Unit) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Constructor {
    MatchAll,
    Variant { tag: VariantTag, fields: PatternStack },
}

impl Constructor {
    fn is_wildcard(&self) -> bool {
        match self {
            MatchAll => true,
            _ => false,
        }
    }

    fn matches(&self, candidate: VariantTag) -> bool {
        match self {
            MatchAll => true,
            Variant { tag, .. } => *tag == candidate,
        }
    }

    fn take_n_fields(self, n: usize) -> Vec<Constructor> {
        match self {
            MatchAll => {
                repeat(MatchAll).take(n).collect()
            }
            Variant { fields, .. } => {
                assert_eq!(fields.0.len(), n);
                fields.0
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PatternStack(pub Vec<Constructor>);

impl IntoIterator for PatternStack {
    type Item = Constructor;
    type IntoIter = std::iter::Rev<std::vec::IntoIter<Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().rev()
    }
}

impl PatternStack {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn from_ast<'c>(ast: &Ast<'c>, cache: &ModuleCache<'c>) -> PatternStack {
        match ast {
            Ast::Variable(variable) => {
                let id = variable.definition.unwrap();

                use ast::VariableKind::TypeConstructor;
                let constructor = match variable.kind {
                    TypeConstructor(_) => Variant { tag: VariantTag::UserDefined(id), fields: PatternStack(vec![]) },
                    _ => MatchAll,
                };
                PatternStack(vec![constructor])
            },
            Ast::Literal(literal) => {
                let fields = PatternStack(vec![]);

                // Only attempt to match bools and unit values. The ranges of all other
                // literal types are too large.
                let tag = match literal.kind {
                    ast::LiteralKind::Bool(b) => if b { VariantTag::True } else { VariantTag::False },
                    ast::LiteralKind::Unit => VariantTag::Unit,
                    _ => VariantTag::Fail,
                };

                PatternStack(vec![Variant { tag, fields }])
            },
            Ast::Tuple(tuple) => {
                let patterns = tuple.elements.iter().rev()
                    .flat_map(|element| PatternStack::from_ast(element, cache))
                    .collect();

                PatternStack(patterns)
            },
            Ast::FunctionCall(call) => {
                match call.function.as_ref() {
                    Ast::Variable(variable) => {
                        let tag = VariantTag::UserDefined(variable.definition.unwrap());
                        let fields = call.args.iter().rev()
                            .flat_map(|arg| PatternStack::from_ast(arg, cache))
                            .collect();

                        let fields = PatternStack(fields);
                        PatternStack(vec![Variant { tag, fields }])
                    },
                    _ => {
                        error!(ast.locate(), "Invalid syntax used in pattern");
                        PatternStack(vec![])
                    }
                }
            },
            _ => {
                error!(ast.locate(), "Invalid syntax used in pattern");
                PatternStack(vec![])
            }
        }
    }

    fn head(&self) -> Option<&Constructor> {
        self.0.last()
    }

    fn specialize_row(&self, tag: VariantTag, arity: usize) -> Option<Self> {
        match self.head() {
            Some(head) if head.matches(tag) => {
                let mut new_stack = self.0.clone();

                match new_stack.pop() {
                    Some(head) => {
                        new_stack.append(&mut head.take_n_fields(arity));
                    }
                    _ => unreachable!("Cannot specialize empty row"),
                }
                
                Some(PatternStack(new_stack))
            }
            _ => None,
        }
    }

    /// Given self = [patternN, ..., pattern2, head]
    ///  Return Some [patternN, ..., pattern2]   if head == MatchAll
    ///         None                             otherwise
    fn default_specialize_row(&self) -> Option<Self> {
        self.head().filter(|constructor| constructor.is_wildcard())
            .map(|_| PatternStack(self.0.iter().take(self.0.len() - 1).cloned().collect()))
    }

    fn all_wildcards(&self) -> bool {
        self.0.iter().all(|constructor| constructor.is_wildcard())
    }
}

fn get_type_info_id(typ: &Type) -> TypeInfoId {
    match typ {
        Type::UserDefinedType(id) => *id,
        Type::TypeApplication(typ, _) => get_type_info_id(typ.as_ref()),
        _ => unreachable!("get_type_info_id called on non-sum-type: {:?}", typ),
    }
}

/// Returns the type that a constructor constructs.
/// Used as a helper function when checking exhaustiveness.
fn get_variant_type_from_constructor<'c>(constructor_id: DefinitionInfoId, cache: &ModuleCache<'c>) -> TypeInfoId {
    let constructor_type = &cache.definition_infos[constructor_id.0].typ;
    match constructor_type {
        Some(Type::ForAll(_, typ)) => {
            match typ.as_ref() {
                Type::Function(_, return_type) => get_type_info_id(return_type.as_ref()),
                typ => get_type_info_id(typ),
            }
        },
        Some(Type::Function(_, return_type)) => get_type_info_id(return_type.as_ref()),
        Some(Type::UserDefinedType(id)) => *id,
        _ => unreachable!("get_variant_type_from_constructor called on invalid constructor of type: {:?}", constructor_type),
    }
}

/// The builtin constructors true, false, and unit don't have DefinitionInfoIds
/// so they must be manually handled here.
fn builtin_is_exhastive(variants: &HashMap<VariantTag, usize>) -> Option<bool> {
    let mut variants_iter = variants.iter().map(|(tag, _)| *tag);
    let (first, second) = (variants_iter.next(), variants_iter.next());

    use VariantTag::*;
    match (first, second) {
        (Some(True), second) => Some(second == Some(False)),
        (Some(False), second) => Some(second == Some(True)),
        (Some(Unit), _) => Some(true),
        (Some(Fail), _) => Some(false),
        _ => None,
    }
}

fn get_covered_constructors(variants: &HashMap<VariantTag, usize>) -> HashSet<DefinitionInfoId> {
    variants.iter().filter_map(|(tag, _)| match tag {
        VariantTag::UserDefined(id) => Some(*id),

        // All constructors in a well-formed program should be user-defined.
        // To give better errors in the presense of previous type errors though, its
        // possible we do completeness checking with the cases false | None for example.
        // In these cases the builtin variant tags are filtered out here.
        _ => None,
    }).collect()
}

/// Given a hashmap from variant tag -> arity,
/// return true if the hashmap covers all constructors for its type.
fn is_exhaustive<'c>(variants: &HashMap<VariantTag, usize>, cache: &ModuleCache<'c>) -> bool {
    use VariantTag::*;

    if let Some(result) = builtin_is_exhastive(variants) {
        return result;
    }

    match variants.iter().nth(0).map(|(tag, _)| *tag).unwrap() {
        True | False | Unit | Fail =>
            unreachable!("Found builtin constructor not covered by builtin_is_exhastive"),

        UserDefined(id) => {
            let type_id = get_variant_type_from_constructor(id, cache);
            match &cache.type_infos[type_id.0].body {
                TypeInfoBody::Union(constructors) => {
                    let all_constructors = constructors.iter().map(|constructor| constructor.id).collect();
                    let covered_constructors = get_covered_constructors(variants);
                    covered_constructors == all_constructors
                },

                // Structs only have one constructor anyway, so if
                // we have a constructor its always exhaustive.
                TypeInfoBody::Struct(_) => true,
                TypeInfoBody::Alias(_) => unimplemented!("Pattern matching on aliased types is unimplemented"),
                TypeInfoBody::Unknown => unreachable!("Cannot pattern match on unknown type constructor"),
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct PatternMatrix {
    /// Each row holds the pattern stack of the pattern for a particular branch as
    /// well as the index of the branch that the pattern leads to in the source if matched.
    rows: Vec<(PatternStack, usize)>,
}

impl PatternMatrix {
    pub fn from_ast<'c>(match_expr: &ast::Match<'c>, cache: &ModuleCache<'c>) -> PatternMatrix {
        let rows = match_expr.branches.iter().enumerate()
            .map(|(branch_index, (pattern, _))| (PatternStack::from_ast(pattern, cache), branch_index))
            .collect();

        PatternMatrix { rows }
    }

    /// This function corresponds to S(c, P -> A) in "Compiling
    /// Pattern Matching to Good Decision Trees"
    ///
    /// Remove all rows of the matrix whose first pattern is not the
    /// given construct or a catch-all pattern. For the rows that are
    /// retained, pop the first pattern off the stack for that row. E.g.
    ///
    /// Matrix =
    ///     []      _      -> 1
    ///     _       []     -> 2
    ///     (_::_)  (_::_) -> 3
    ///
    /// specialize(Matrix, (::)) =
    ///     _ _ []      -> 2
    ///     _ _ (_::_)  -> 3
    ///
    fn specialize(&self, tag: VariantTag, arity: usize) -> Self {
        let mut matrix = PatternMatrix::default();

        for (row, branch) in self.rows.iter() {
            match row.specialize_row(tag, arity) {
                Some(row) => matrix.rows.push((row, *branch)),
                None => (),
            }
        }

        matrix
    }

    /// This function corresponds to D(P -> A) in "Compiling
    /// Pattern Matching to Good Decision Trees"
    ///
    /// Remove all rows of the matrix whose first pattern is
    /// not a catch-all. For the rows that are retained, pop
    /// the catch-all off the stack for that row. E.g.
    ///
    /// Matrix =
    ///     [] _  -> 1
    ///     _  [] -> 2
    ///     _  _  -> 3
    ///
    /// default_specialize(Matrix) =
    ///     [] -> 2
    ///     _  -> 3
    ///
    fn default_specialize(&self) -> Self {
        let mut matrix = PatternMatrix::default();

        for (row, branch) in self.rows.iter() {
            match row.default_specialize_row() {
                Some(row) => matrix.rows.push((row, *branch)),
                None => (),
            }
        }

        matrix
    }

    /// Generate a Switch branch covering each case of the top pattern on the stack.
    /// Handles exhaustiveness checking for the union internally.
    fn switch_on_pattern<'c>(&self, cache: &ModuleCache<'c>, location: Location<'c>) -> DecisionTree {
        // Generate the set of constructors appearing in the column
        let mut matched_variants = HashMap::new();
        for (row, _) in self.rows.iter() {
            if let Some(Variant { tag, fields }) = row.head() {
                matched_variants.insert(*tag, fields.0.len());
            }
        }

        let exhaustive = is_exhaustive(&matched_variants, cache);

        let mut cases: Vec<_> = matched_variants.into_iter().map(|(tag, arity)| {
            let mut branch = self.specialize(tag, arity);
            let fields = PatternStack(repeat(MatchAll).take(arity).collect());

            (Variant { tag, fields }, branch.compile(cache, location))
        }).collect();

        // If we don't have an exhaustive match, generate a default matrix
        if !exhaustive {
            cases.push((MatchAll, self.default_specialize().compile(cache, location)));
        }

        DecisionTree::Switch(cases)
    }

    fn find_first_non_default_column(&self) -> Option<usize> {
        let len = self.rows[0].0.len();

        for col in (1 .. len).rev() {
            for (row, _) in self.rows.iter() {
                match row.0.get(col) {
                    Some(MatchAll) => continue,
                    _ => return Some(col),
                }
            }
        }
        None
    }

    fn swap_column(&mut self, column: usize) -> &mut Self {
        for (row, _) in self.rows.iter_mut() {
            row.0.swap(0, column);
        }
        self
    }

    pub fn compile<'c>(&mut self, cache: &ModuleCache<'c>, location: Location<'c>) -> DecisionTree {
        if self.rows.is_empty() {
            // We have an in-exhaustive case expression
            error!(location, "Match is non-exhaustive");
            DecisionTree::Fail
        } else if self.rows.get(0).map_or(false, |(row, _)| row.all_wildcards()) {
            // If every pattern in the first row is a wildcard it must match.
            DecisionTree::Leaf(self.rows[0].1)
        } else {
            // There's at least one non-wild pattern in the matrix somewhere
            for (row, _) in self.rows.iter() {
                match row.head() {
                    Some(Variant { .. }) => return self.switch_on_pattern(cache, location),
                    Some(MatchAll) => continue,
                    None => unreachable!("PatternMatrix rows cannot be empty"),
                }
            }

            // The first column contains only wildcard patterns. Search the
            // matrix until we find a column that has a non-wildcard pattern,
            // and swap columns with column 0
            match self.find_first_non_default_column() {
                Some(column) => self.swap_column(column).compile(cache, location),
                None => self.default_specialize().compile(cache, location),
            }
        }
    }
}

#[derive(Debug)]
pub enum DecisionTree {
    /// Success! run the code at the given numerical branch
    Leaf(usize),

    /// The pattern failed to match anything,
    /// if this is constructed we issue a compile time error
    /// that the match is non-exhaustive.
    Fail,

    /// Multi-way test
    Switch(Vec<(Constructor, DecisionTree)>),
}
