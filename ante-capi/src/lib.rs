//! A non-final C API made just to expose the minimal API for ADocGen to query the compiler for
//! parsed code, typed signatures, modules, and doc comments. This may expand or be changed completely
//! in the future.
//!
//! Items are currently addressed by (module index, item index). The central object is a [Session] which
//! is the state of a project and is immutable once created.
use std::{
    ffi::{CStr, CString, c_char},
    panic::catch_unwind,
    path::Path,
    ptr,
};

use ante::{
    diagnostics::Location,
    find_files,
    incremental::{
        Db, ExportedDefinitions, ExportedTypes, GetCrateGraph, GetItem, Parse, TargetPointerSize, TypeCheck,
    },
    name_resolution::namespace::{CrateId, SourceFileId},
    parser::{
        context::TopLevelContext,
        cst::{Expr, ItemName, Name, Pattern, TopLevelItem, TopLevelItemKind},
        ids::{IdStore, NameStore, PatternId, TopLevelId},
    },
    paths::stdlib_path,
};

pub struct Session {
    crate_name: CString,
    modules: Vec<Module>,
}

struct Module {
    name: CString,
    path: CString,
    items: Vec<Item>,
}

struct Item {
    kind: ItemKind,
    line: u32,
    exported: bool,
    name: CString,
    signature: CString,
    /// `None` if the item has no doc comment
    doc: Option<CString>,
    /// If the item is a method, this is its object type
    object_type: Option<CString>,
}

#[repr(u32)]
#[derive(Clone, Copy)]
enum ItemKind {
    Definition = 0,
    Type = 1,
    Trait = 2,
    Effect = 3,
    Impl = 4,
    Comptime = 5,
    Implicit = 6,
}

fn c_string(s: String) -> CString {
    c_string_from_bytes(s.into_bytes())
}

fn c_string_from_bytes(mut bytes: Vec<u8>) -> CString {
    bytes.retain(|&byte| byte != 0);
    unsafe { CString::from_vec_unchecked(bytes) }
}

fn kind_of(kind: &TopLevelItemKind) -> ItemKind {
    match kind {
        TopLevelItemKind::Definition(definition) if definition.implicit => ItemKind::Implicit,
        TopLevelItemKind::Definition(_) => ItemKind::Definition,
        TopLevelItemKind::TypeDefinition(_) => ItemKind::Type,
        TopLevelItemKind::TraitDefinition(_) => ItemKind::Trait,
        TopLevelItemKind::EffectDefinition(_) => ItemKind::Effect,
        TopLevelItemKind::TraitImpl(_) => ItemKind::Impl,
        TopLevelItemKind::Comptime(_) => ItemKind::Comptime,
    }
}

fn exported_items(db: &Db, file: SourceFileId) -> Vec<TopLevelId> {
    let definitions = ExportedDefinitions(file).get(db);
    let methods = definitions.methods.values().flat_map(|methods| methods.values());
    let types = ExportedTypes(file).get(db);
    definitions.definitions.values().chain(methods).chain(types.values()).map(|name| name.top_level_item).collect()
}

fn load_module(db: &Db, crate_name: &str, path: &Path, file: SourceFileId) -> Module {
    let parse = Parse(file).get(db);
    let exported = exported_items(db, file);

    let items = parse.cst.top_level_items.iter().filter_map(|item| {
        let context = parse.top_level_data.get(&item.id)?;
        Some(Item {
            kind: kind_of(&item.kind),
            line: name_location(item.kind.name(), context).span.start.line_number,
            exported: exported.contains(&item.id),
            name: c_string(item.kind.name().to_string(context.as_ref())),
            signature: c_string(signature(db, item, context)),
            doc: (!item.comments.is_empty()).then(|| c_string(item.comments.join("\n"))),
            object_type: object_type(&item.kind, context).map(|name| c_string(name.to_string())),
        })
    });
    let items = items.collect();

    let path_bytes = path.as_os_str().as_encoded_bytes().to_vec();
    Module { name: c_string(module_name(crate_name, path)), path: c_string_from_bytes(path_bytes), items }
}

/// `Std` and `Foo/Bar.an` give `Std.Foo.Bar`
fn module_name(crate_name: &str, path: &Path) -> String {
    let mut name = crate_name.to_owned();
    let parents = path.parent().into_iter().flatten();
    for segment in parents.chain(path.file_stem()) {
        name.push('.');
        name.push_str(&segment.to_string_lossy());
    }
    name
}

/// Return a string representing the signature of the given item
fn signature(db: &Db, item: &TopLevelItem, context: &TopLevelContext) -> String {
    if is_annotated(&item.kind, context) {
        return item.display_signature(context).to_string();
    }
    let (desugared, _) = GetItem(item.id).get(db);
    let check = TypeCheck(item.id).get(db);
    desugared.display_typed_signature(&check.result.context, db).to_string()
}

/// True if a definition's full type is written in source
fn is_annotated(kind: &TopLevelItemKind, context: &TopLevelContext) -> bool {
    let TopLevelItemKind::Definition(definition) = kind else { return true };
    match context.get_expr(definition.rhs) {
        Expr::Lambda(lambda) => lambda.return_type.is_some(),
        Expr::TypeAnnotation(_) => true,
        _ => matches!(context.get_pattern(definition.pattern), Pattern::TypeAnnotation(..)),
    }
}

fn object_type<'a>(kind: &TopLevelItemKind, context: &'a TopLevelContext) -> Option<&'a Name> {
    let TopLevelItemKind::Definition(definition) = kind else { return None };
    let mut pattern = context.get_pattern(definition.pattern);
    while let Pattern::TypeAnnotation(inner, _) = pattern {
        pattern = context.get_pattern(*inner);
    }
    match pattern {
        Pattern::MethodName { type_name, .. } => Some(context.get_name(*type_name)),
        _ => None,
    }
}

fn name_location<'a>(name: ItemName, context: &'a TopLevelContext) -> &'a Location {
    match name {
        ItemName::Single(name) => &context.name_locations[name],
        ItemName::Pattern(pattern) => pattern_name_location(pattern, context),
        ItemName::None => &context.location,
    }
}

fn pattern_name_location(pattern: PatternId, context: &TopLevelContext) -> &Location {
    match context.get_pattern(pattern) {
        Pattern::Variable(name) | Pattern::Alias(name, _) | Pattern::ConstructorRest(_, _, Some(name)) => {
            &context.name_locations[*name]
        },
        Pattern::MethodName { item_name, .. } => &context.name_locations[*item_name],
        Pattern::TypeAnnotation(inner, _) => pattern_name_location(*inner, context),
        Pattern::Or(alts) if !alts.is_empty() => pattern_name_location(alts[0], context),
        _ => &context.pattern_locations[pattern],
    }
}

fn load_session(root: &Path) -> Option<Session> {
    let mut db = Db::default();
    TargetPointerSize.set(&mut db, 8);
    find_files::populate_crates_and_files(&mut db, root, &[]);

    let is_stdlib = same_directory(root, &stdlib_path());
    let crate_id = if is_stdlib { CrateId::STDLIB } else { CrateId::LOCAL };

    let graph = GetCrateGraph.get(&db);
    let crate_ = graph.get(&crate_id)?;
    let modules = crate_.source_files.iter().map(|(path, file)| load_module(&db, &crate_.name, path, *file)).collect();
    Some(Session { crate_name: c_string(crate_.name.clone()), modules })
}

fn same_directory(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(unix)]
fn path_from_c(path: &CStr) -> Option<&Path> {
    use std::os::unix::ffi::OsStrExt;
    Some(Path::new(std::ffi::OsStr::from_bytes(path.to_bytes())))
}

#[cfg(not(unix))]
fn path_from_c(path: &CStr) -> Option<&Path> {
    path.to_str().ok().map(Path::new)
}

unsafe fn module<'a>(s: *const Session, m: usize) -> Option<&'a Module> {
    unsafe { s.as_ref() }?.modules.get(m)
}

unsafe fn item<'a>(s: *const Session, m: usize, i: usize) -> Option<&'a Item> {
    unsafe { module(s, m) }?.items.get(i)
}

fn as_ptr(string: Option<&CStr>) -> *const c_char {
    string.map_or(ptr::null(), CStr::as_ptr)
}

/// Create a session for the Ante project in `root`, which should contain the `src` directory.
/// Returns null if `root` is null or the project could not be loaded.
///
/// # Safety
/// `root` must be null or a valid nul-terminated C-string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_session_new(root: *const c_char) -> *mut Session {
    if root.is_null() {
        return ptr::null_mut();
    }
    let Some(root) = path_from_c(unsafe { CStr::from_ptr(root) }) else { return ptr::null_mut() };

    // Catch any possible panics
    match catch_unwind(|| load_session(root)) {
        Ok(Some(session)) => Box::into_raw(Box::new(session)),
        _ => ptr::null_mut(),
    }
}

/// Free a session and every string it returned. Freeing null does nothing.
///
/// # Safety
/// `s` must be null or an unfreed session from [ante_session_new] which no thread is still using
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_session_free(s: *mut Session) {
    if !s.is_null() {
        drop(unsafe { Box::from_raw(s) });
    }
}

/// The name of the crate being documented, from its `ante.toml` if it has one
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_crate_name(s: *const Session) -> *const c_char {
    as_ptr(unsafe { s.as_ref() }.map(|s| s.crate_name.as_c_str()))
}

/// The number of modules in the session's local project
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_module_count(s: *const Session) -> usize {
    unsafe { s.as_ref() }.map_or(0, |s| s.modules.len())
}

/// The module's full name, e.g. `Std.Vec`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_module_name(s: *const Session, m: usize) -> *const c_char {
    as_ptr(unsafe { module(s, m) }.map(|module| module.name.as_c_str()))
}

/// The module's path relative to its crate's `src` directory, e.g. `Vec.an`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_module_path(s: *const Session, m: usize) -> *const c_char {
    as_ptr(unsafe { module(s, m) }.map(|module| module.path.as_c_str()))
}

/// The number of top-level items in the given module
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_item_count(s: *const Session, m: usize) -> usize {
    unsafe { module(s, m) }.map_or(0, |module| module.items.len())
}

/// Returns:
/// - 0 for definitions
/// - 1 for types
/// - 2 for traits
/// - 3 for effects
/// - 4 for impls
/// - 5 for comptime
/// - 6 for implicit definitions
/// - UINT32_MAX for an invalid index
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_item_kind(s: *const Session, m: usize, i: usize) -> u32 {
    unsafe { item(s, m, i) }.map_or(u32::MAX, |item| item.kind as u32)
}

/// The name of the i'th item in the m'th module of the given session
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_item_name(s: *const Session, m: usize, i: usize) -> *const c_char {
    as_ptr(unsafe { item(s, m, i) }.map(|item| item.name.as_c_str()))
}

/// The item's `///` doc comment lines joined by newlines, or an empty string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_item_doc(s: *const Session, m: usize, i: usize) -> *const c_char {
    as_ptr(unsafe { item(s, m, i) }.map(|item| item.doc.as_deref().unwrap_or(c"")))
}

/// The item's signature mostly as written in source. If the item's type was inferred, it will
/// be included as well.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_item_signature(s: *const Session, m: usize, i: usize) -> *const c_char {
    as_ptr(unsafe { item(s, m, i) }.map(|item| item.signature.as_c_str()))
}

/// The source line an item's name is on
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_item_line(s: *const Session, m: usize, i: usize) -> u32 {
    unsafe { item(s, m, i) }.map_or(0, |item| item.line)
}

/// True if the given item is publically exported
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_item_exported(s: *const Session, m: usize, i: usize) -> bool {
    unsafe { item(s, m, i) }.is_some_and(|item| item.exported)
}

/// The type a method is defined on, or an empty string if this item is not a method
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ante_item_object_type(s: *const Session, m: usize, i: usize) -> *const c_char {
    as_ptr(unsafe { item(s, m, i) }.map(|item| item.object_type.as_deref().unwrap_or(c"")))
}
