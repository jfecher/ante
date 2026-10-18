use std::collections::HashMap;

use ante::{
    diagnostics::{Diagnostic as AnteDiagnostic, Span},
    incremental::{CheckAll, Db, Parse},
    name_resolution::namespace::SourceFileId,
};
use tower_lsp::lsp_types::{CodeAction, CodeActionKind, Diagnostic as LspDiagnostic, TextEdit, Url, WorkspaceEdit};

use crate::auto_import::{build_import_edit, candidate_for_file, exports_index};
use crate::util::byte_range_to_lsp_range;

/// Build the list of code actions available at `[start_byte, end_byte]` in `file_id`.
///
/// Two kinds of quick-fix are offered:
///
/// 1. For each `NameNotInScope` diagnostic whose span overlaps the cursor or
///    selection, suggest an `import` for every module in any crate that exports
///    that name.
/// 2. For each `NoImplicitFound` diagnostic whose span overlaps the cursor,
///    surface the compiler's pre-computed `suggestions` list as one action per
///    candidate import path.
///
/// In both cases, if an `import` line for the matching module already exists
/// the edit extends it in place; otherwise a new line is inserted at the top
/// of the file (after any pre-existing imports).
pub fn code_actions_at(
    compiler: &Db, file_id: SourceFileId, start_byte: usize, end_byte: usize, uri: &Url, rope: &ropey::Rope,
    client_diagnostics: &[LspDiagnostic],
) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    let diagnostics = compiler.get_accumulated(CheckAll);
    let parse = Parse(file_id).get(compiler);
    let existing_imports = &parse.cst.imports;

    // `NameNotInScope` actions need a crate-graph-wide `exports_index`, so we
    // collect those misses and emit their actions in a second pass once we know
    // the index is worth computing. `NoImplicitFound` already carries its
    // suggestions, so we handle those inline as we iterate.
    let mut missing: Vec<(String, Option<LspDiagnostic>)> = Vec::new();

    for diag in diagnostics.iter() {
        let location = match diag {
            AnteDiagnostic::NameNotInScope { location, .. } | AnteDiagnostic::NoImplicitFound { location, .. } => {
                location
            },
            _ => continue,
        };
        if location.file_id != file_id {
            continue;
        }
        if !range_overlaps(start_byte, end_byte, location.span.start.byte_index, location.span.end.byte_index) {
            continue;
        }
        match diag {
            AnteDiagnostic::NameNotInScope { name, .. } => {
                let lsp_diag = find_matching_lsp_diagnostic(client_diagnostics, name.as_str());
                missing.push((name.to_string(), lsp_diag));
            },
            AnteDiagnostic::NoImplicitFound { suggestions, .. } => {
                if suggestions.is_empty() {
                    continue;
                }
                let link_diag = find_lsp_diagnostic_by_span(client_diagnostics, &location.span, rope);
                let only_one = suggestions.len() == 1;
                for sugg in suggestions {
                    let item_name = sugg.qualified_path.rsplit('.').next().unwrap_or("");
                    if item_name.is_empty() {
                        continue;
                    }
                    let Some(cand) = candidate_for_file(compiler, sugg.location.file_id) else { continue };
                    let Some(edit) = build_import_edit(Some(item_name), &cand, existing_imports, rope) else {
                        continue;
                    };
                    actions.push(import_quick_fix(
                        uri,
                        format!("Import `{}`", sugg.qualified_path),
                        edit,
                        link_diag.clone(),
                        only_one,
                    ));
                }
            },
            _ => {},
        }
    }

    if let Some(action) = crate::exports::add_to_exports_action(compiler, file_id, start_byte, end_byte, uri, rope) {
        actions.push(action);
    }

    if !missing.is_empty() {
        // One pass over the crate graph builds `name -> Vec<Candidate>`, then each
        // missing name is an O(log M) lookup. Previously each name triggered its
        // own full walk.
        let index = exports_index(compiler, file_id);
        for (name, lsp_diag) in &missing {
            let Some(candidates) = index.get(name.as_str()) else { continue };
            let only_one = candidates.len() == 1;
            for cand in candidates {
                let Some(edit) = build_import_edit(Some(name), cand, existing_imports, rope) else {
                    continue;
                };
                actions.push(import_quick_fix(
                    uri,
                    format!("Import `{name}` from {}", cand.dotted_display),
                    edit,
                    lsp_diag.clone(),
                    only_one,
                ));
            }
        }
    }

    actions
}

fn import_quick_fix(
    uri: &Url, title: String, edit: TextEdit, link_diag: Option<LspDiagnostic>, only_one: bool,
) -> CodeAction {
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);
    CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: link_diag.map(|d| vec![d]),
        edit: Some(WorkspaceEdit { changes: Some(changes), ..Default::default() }),
        is_preferred: Some(only_one),
        ..Default::default()
    }
}

fn range_overlaps(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    // Inclusive overlap so a cursor positioned at either end of an identifier
    // still counts as "on" it.
    a_start <= b_end && b_start <= a_end
}

fn find_matching_lsp_diagnostic(diagnostics: &[LspDiagnostic], name: &str) -> Option<LspDiagnostic> {
    let needle = format!("`{name}`");
    diagnostics.iter().find(|d| d.source.as_deref() == Some("ante-ls") && d.message.contains(&needle)).cloned()
}

/// Find the client-side LSP diagnostic whose range matches `span`. Used by
/// `NoImplicitFound` (whose rendered message has no single name in backticks,
/// so the name-based heuristic does not apply).
fn find_lsp_diagnostic_by_span(
    diagnostics: &[LspDiagnostic], span: &Span, rope: &ropey::Rope,
) -> Option<LspDiagnostic> {
    let range = byte_range_to_lsp_range(span.start.byte_index, span.end.byte_index, rope).ok()?;
    diagnostics.iter().find(|d| d.source.as_deref() == Some("ante-ls") && d.range == range).cloned()
}
