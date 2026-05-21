use std::collections::HashMap;

use ante::{
    diagnostics::Diagnostic as AnteDiagnostic,
    incremental::{CheckAll, Db, Parse},
    name_resolution::namespace::SourceFileId,
};
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, Diagnostic as LspDiagnostic, Url, WorkspaceEdit,
};

use crate::auto_import::{build_import_edit, exports_index};

/// Build the list of code actions available at `[start_byte, end_byte]` in `file_id`.
///
/// The only kind we offer today: for each `NameNotInScope` diagnostic whose span
/// overlaps the cursor or selection, suggest an `import` for every module in any
/// crate that exports that name. If an `import` line for the matching module
/// already exists, the action extends it in place; otherwise a new import line
/// is inserted at the top of the file (after any pre-existing imports).
pub fn code_actions_at(
    compiler: &Db,
    file_id: SourceFileId,
    start_byte: usize,
    end_byte: usize,
    uri: &Url,
    rope: &ropey::Rope,
    client_diagnostics: &[LspDiagnostic],
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Pull the canonical compiler diagnostics so we work off the same names the
    // user is seeing red squiggles for, rather than guessing at the identifier
    // under the cursor.
    let diagnostics = compiler.get_accumulated(CheckAll);
    let mut missing: Vec<(String, Option<LspDiagnostic>)> = Vec::new();
    for diag in diagnostics.iter() {
        if let AnteDiagnostic::NameNotInScope { name, location } = diag {
            if location.file_id != file_id {
                continue;
            }
            if !range_overlaps(
                start_byte,
                end_byte,
                location.span.start.byte_index,
                location.span.end.byte_index,
            ) {
                continue;
            }
            // Try to link this action to the LSP diagnostic the client knows
            // about so editors can offer it as a quickfix for the squiggle.
            let lsp_diag = find_matching_lsp_diagnostic(client_diagnostics, name.as_str());
            missing.push((name.to_string(), lsp_diag));
        }
    }
    if missing.is_empty() {
        return actions;
    }

    // One pass over the crate graph builds `name -> Vec<Candidate>`, then each
    // missing name is an O(log M) lookup. Previously each name triggered its
    // own full walk.
    let index = exports_index(compiler, file_id);

    let parse = Parse(file_id).get(compiler);
    let existing_imports = &parse.cst.imports;

    for (name, lsp_diag) in &missing {
        let Some(candidates) = index.get(name.as_str()) else { continue };
        let only_one = candidates.len() == 1;
        for cand in candidates {
            let Some(text_edit) = build_import_edit(Some(name), cand, existing_imports, rope) else {
                continue;
            };
            let mut changes = HashMap::new();
            changes.insert(uri.clone(), vec![text_edit]);
            let action = CodeAction {
                title: format!("Import `{name}` from {}", cand.dotted_display),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: lsp_diag.clone().map(|d| vec![d]),
                edit: Some(WorkspaceEdit { changes: Some(changes), ..Default::default() }),
                is_preferred: Some(only_one),
                ..Default::default()
            };
            actions.push(action);
        }
    }

    actions
}

fn range_overlaps(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    // Inclusive overlap so a cursor positioned at either end of an identifier
    // still counts as "on" it.
    a_start <= b_end && b_start <= a_end
}

fn find_matching_lsp_diagnostic(diagnostics: &[LspDiagnostic], name: &str) -> Option<LspDiagnostic> {
    let needle = format!("`{name}`");
    diagnostics
        .iter()
        .find(|d| d.source.as_deref() == Some("ante-ls") && d.message.contains(&needle))
        .cloned()
}
