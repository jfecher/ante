use std::sync::{Arc, OnceLock};

use ante::{incremental::Db, name_resolution::namespace::SourceFileId};

use dashmap::DashMap;
use ropey::Rope;
use tokio::sync::RwLock;
use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer, LspService, Server};

mod auto_import;
mod code_action;
mod completion;
mod definition;
mod diagnostics;
mod hover;
mod util;

use code_action::code_actions_at;
use completion::completions_at;
use definition::definition_at;
use diagnostics::{init_db, rope_for_file, CrateRoots, DiagnosticsWorker, DirtyDocs};
use hover::hover_at;
use util::{byte_range_to_lsp_range, identifier_prefix_before, lsp_range_to_rope_range, position_to_byte_offset};

struct Backend {
    client: Client,
    document_map: Arc<DashMap<Url, Rope>>,
    /// Last seen `didChange` version per document, used to detect out-of-order edits.
    document_versions: DashMap<Url, i32>,
    compiler: Arc<RwLock<Db>>,
    /// Documents edited since the last compile; drained by the [DiagnosticsWorker].
    dirty: Arc<DirtyDocs>,
    /// The crates' canonical `src/` roots, set once in `initialize`.
    crate_roots: OnceLock<Arc<CrateRoots>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.client.log_message(MessageType::LOG, format!("ante-ls initialize: {:?}", params)).await;

        let root = params
            .root_uri
            .as_ref()
            .and_then(|uri| uri.to_file_path().ok())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Set once and eagerly walk the crate graph so requests never have to do it lazily.
        // Only the call that sets the roots spawns the diagnostics worker.
        if self.crate_roots.get().is_none() {
            let roots = {
                let mut compiler = self.compiler.write().await;
                init_db(&mut compiler, &root);
                Arc::new(CrateRoots::new(&compiler, root))
            };

            if self.crate_roots.set(roots.clone()).is_ok() {
                tokio::spawn(
                    DiagnosticsWorker {
                        client: self.client.clone(),
                        document_map: self.document_map.clone(),
                        compiler: self.compiler.clone(),
                        dirty: self.dirty.clone(),
                        crate_roots: roots,
                    }
                    .run(),
                );
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                    // Ask for the full text on save so `did_save` can repair any divergence
                    // between our rope and the editor's buffer.
                    save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions { include_text: Some(true) })),
                    ..Default::default()
                })),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: None,
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: None,
                }),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                position_encoding: Some(PositionEncodingKind::UTF16),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, params: InitializedParams) {
        self.client.log_message(MessageType::LOG, format!("ante-ls initialized: {:?}", params)).await;
    }

    async fn shutdown(&self) -> Result<()> {
        self.client.log_message(MessageType::LOG, "ante-ls shutdown".to_string()).await;
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        // Mutate the document state before the first await: tower-lsp polls up to 4
        // handlers concurrently (`buffer_unordered`), so an await before the mutation
        // would let a later notification's edit apply first and corrupt the rope.
        let uri = params.text_document.uri;
        let rope = Rope::from_str(&params.text_document.text);
        self.document_map.insert(uri.clone(), rope);
        self.document_versions.insert(uri.clone(), params.text_document.version);
        self.dirty.mark(uri.clone());

        self.client.log_message(MessageType::LOG, format!("ante-ls did_open: {uri:?}")).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // Apply the edit before the first await - see `did_open` for why.
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        // `alter` silently no-ops on a missing key; incremental edits without a base
        // document cannot be applied, so surface the dropped edits instead.
        if !self.document_map.contains_key(&uri) {
            let message = format!("ante-ls did_change: no open document for {uri:?}; dropping edits");
            self.client.log_message(MessageType::ERROR, message).await;
            return;
        }

        // Drop stale edits: a version older than the last seen one would mutate the rope
        // out of order and corrupt it.
        if let Some(previous) = self.document_versions.get(&uri).map(|v| *v) {
            if version < previous {
                let message = format!(
                    "ante-ls did_change: version {version} arrived after {previous} for {uri:?}; dropping out-of-order edits"
                );
                self.client.log_message(MessageType::ERROR, message).await;
                return;
            }
        }

        // `alter`'s closure can't await, so collect any dropped edits and log them afterward.
        let mut dropped_edits = 0;
        self.document_map.alter(&uri, |_, mut rope| {
            for change in params.content_changes {
                if !apply_content_change(&mut rope, change) {
                    dropped_edits += 1;
                }
            }
            rope
        });
        self.document_versions.insert(uri.clone(), version);
        self.dirty.mark(uri.clone());

        if dropped_edits > 0 {
            let message = format!(
                "ante-ls did_change: dropped {dropped_edits} edit(s) for {uri:?} whose range could not be mapped onto the document"
            );
            self.client.log_message(MessageType::ERROR, message).await;
        } else {
            self.client.log_message(MessageType::LOG, format!("ante-ls did_change: {uri:?}")).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Replace the rope with the saved text (the server requests `include_text`)
        // before the first await - see `did_open` for why. This also repairs any
        // divergence between our rope and the editor's buffer.
        let uri = params.text_document.uri;
        if let Some(text) = params.text {
            let rope = Rope::from_str(&text);
            self.document_map.insert(uri.clone(), rope);
        }
        self.dirty.mark(uri.clone());

        self.client.log_message(MessageType::LOG, format!("ante-ls did_save: {uri:?}")).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Remove the document state before the first await - see `did_open` for why.
        // Per the LSP spec, after a close the file's truth reverts to its on-disk
        // contents; marking it dirty makes the worker re-read the file from disk.
        let uri = params.text_document.uri;
        self.document_map.remove(&uri);
        self.document_versions.remove(&uri);
        self.dirty.mark(uri.clone());

        self.client.log_message(MessageType::LOG, format!("ante-ls did_close: {uri:?}")).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let Some(ctx) = self.resolve_position(params.text_document_position_params).await else {
            return Ok(None);
        };

        let hover_text = {
            let compiler = self.compiler.read().await;
            hover_at(&compiler, ctx.file_id, ctx.byte_offset)
        };

        Ok(hover_text.map(|value| Hover {
            contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::PlainText, value }),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let Some(ctx) = self.resolve_position(params.text_document_position).await else {
            return Ok(None);
        };

        let prefix = identifier_prefix_before(&ctx.rope, ctx.byte_offset);
        let compiler = self.compiler.read().await;
        let items = completions_at(&compiler, ctx.file_id, ctx.byte_offset, &ctx.rope, &prefix);
        // incomplete so we can add out-of-scope items when the input is closer to their name
        // instead of adding every item in every library all the time.
        Ok(Some(CompletionResponse::List(CompletionList { is_incomplete: true, items })))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let tdpp =
            TextDocumentPositionParams { text_document: params.text_document.clone(), position: params.range.start };
        let Some(ctx) = self.resolve_position(tdpp).await else {
            return Ok(None);
        };

        // Convert the full LSP range (not just the start) to a byte range so we can
        // match any NameNotInScope diagnostic that overlaps the cursor or selection.
        let Ok(rope_range) = lsp_range_to_rope_range(params.range, &ctx.rope) else {
            return Ok(None);
        };
        let start_byte = ctx.rope.char_to_byte(rope_range.start);
        let end_byte = ctx.rope.char_to_byte(rope_range.end);

        let compiler = self.compiler.read().await;
        let actions = code_actions_at(
            &compiler,
            ctx.file_id,
            start_byte,
            end_byte,
            &ctx.uri,
            &ctx.rope,
            &params.context.diagnostics,
        );

        if actions.is_empty() {
            return Ok(None);
        }
        Ok(Some(actions.into_iter().map(CodeActionOrCommand::CodeAction).collect()))
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let Some(ctx) = self.resolve_position(params.text_document_position_params).await else {
            return Ok(None);
        };

        let compiler = self.compiler.read().await;
        let Some(ante_loc) = definition_at(&compiler, ctx.file_id, ctx.byte_offset) else {
            return Ok(None);
        };
        let source_file = ante_loc.file_id.get(&*compiler);
        let Ok(def_uri) = Url::from_file_path(source_file.path.as_ref()) else {
            self.client
                .log_message(MessageType::ERROR, format!("Definition path is not a valid URI: {:?}", source_file.path))
                .await;
            return Ok(None);
        };
        let def_rope = rope_for_file(&def_uri, &source_file.contents, &self.document_map);
        let Ok(range) =
            byte_range_to_lsp_range(ante_loc.span.start.byte_index, ante_loc.span.end.byte_index, &def_rope)
        else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location { uri: def_uri, range })))
    }
}

/// Apply a single `didChange` content change to `rope` in place. A change with
/// `range = None` is a full document replace, otherwise it is an incremental edit.
/// Returns true if the change was successfully applied.
fn apply_content_change(rope: &mut Rope, change: TextDocumentContentChangeEvent) -> bool {
    match change.range {
        None => {
            *rope = Rope::from_str(&change.text);
            true
        },
        Some(range) => match lsp_range_to_rope_range(range, rope) {
            Ok(range) => {
                rope.remove(range.clone());
                rope.insert(range.start, &change.text);
                true
            },
            Err(_) => false,
        },
    }
}

/// Everything `hover` and `goto_definition` need to share: the document's rope,
/// the byte offset under the cursor, and the `SourceFileId` to look it up in the Db.
struct RequestContext {
    uri: Url,
    rope: Rope,
    byte_offset: usize,
    file_id: SourceFileId,
}

impl Backend {
    /// Derive the per-request context from a `TextDocumentPositionParams`. Returns `None`
    /// (and logs the unexpected failures) for any of the early-bailout conditions that
    /// every position-based request shares.
    async fn resolve_position(&self, params: TextDocumentPositionParams) -> Option<RequestContext> {
        let uri = params.text_document.uri;
        let position = params.position;

        let rope = self.document_map.get(&uri).map(|r| r.clone())?;
        let byte_offset = position_to_byte_offset(position, &rope)?;

        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                self.client.log_message(MessageType::ERROR, format!("URI is not a file path: {uri}")).await;
                return None;
            },
        };

        let roots = match self.crate_roots.get() {
            Some(roots) => roots,
            None => {
                self.client
                    .log_message(MessageType::ERROR, "resolve_position called before initialize()".to_string())
                    .await;
                return None;
            },
        };
        // Match the path against the crate graph so ids agree with `set_file_content`
        // (e.g. a stdlib file resolves to the stdlib crate's id, not a local-crate one).
        let file_id = diagnostics::file_id_for_path(roots, &path);

        Some(RequestContext { uri, rope, byte_offset, file_id })
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client,
        document_map: Arc::new(DashMap::new()),
        document_versions: DashMap::new(),
        compiler: Arc::new(RwLock::new(Db::default())),
        dirty: Arc::new(DirtyDocs::default()),
        crate_roots: OnceLock::new(),
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn incremental_change(range: Range, text: &str) -> TextDocumentContentChangeEvent {
        TextDocumentContentChangeEvent { range: Some(range), range_length: None, text: text.to_string() }
    }

    #[test]
    fn full_replace_change_replaces_the_whole_document() {
        let mut rope = Rope::from_str("hello world\n");
        let change = TextDocumentContentChangeEvent { range: None, range_length: None, text: "new".to_string() };
        assert!(apply_content_change(&mut rope, change));
        assert_eq!(rope.to_string(), "new");
    }

    #[test]
    fn incremental_change_edits_in_place() {
        let mut rope = Rope::from_str("hello world\n");
        let range = Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 5 } };
        assert!(apply_content_change(&mut rope, incremental_change(range, "goodbye")));
        assert_eq!(rope.to_string(), "goodbye world\n");
    }

    #[test]
    fn out_of_bounds_incremental_change_is_dropped_not_applied() {
        let mut rope = Rope::from_str("hello world\n");
        let range = Range { start: Position { line: 99, character: 0 }, end: Position { line: 99, character: 1 } };
        assert!(!apply_content_change(&mut rope, incremental_change(range, "x")));
        assert_eq!(rope.to_string(), "hello world\n", "the document must be left untouched");
    }
}
