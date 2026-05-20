use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use ante::{incremental::Db, name_resolution::namespace::SourceFileId};

use dashmap::DashMap;
use ropey::Rope;
use tokio::sync::RwLock;
use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer, LspService, Server};

mod definition;
mod diagnostics;
mod hover;
mod util;

use definition::definition_at;
use diagnostics::{init_db, rope_for_file};
use hover::hover_at;
use util::{byte_range_to_lsp_range, lsp_range_to_rope_range, position_to_byte_offset};

struct Backend {
    client: Client,
    document_map: DashMap<Url, Rope>,
    compiler: RwLock<Db>,
    local_crate_root: OnceLock<PathBuf>,
}

impl Backend {
    fn root(&self) -> Option<&Path> {
        self.local_crate_root.get().map(PathBuf::as_path)
    }
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
        let _ = self.local_crate_root.set(root.clone());
        {
            let mut compiler = self.compiler.write().await;
            init_db(&mut compiler, &root);
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                position_encoding: Some(PositionEncodingKind::UTF8),
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
        self.client.log_message(MessageType::LOG, format!("ante-ls did_open: {:?}", params.text_document.uri)).await;
        let rope = Rope::from_str(&params.text_document.text);
        self.document_map.insert(params.text_document.uri.clone(), rope.clone());
        self.update_diagnostics(params.text_document.uri, &rope).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client.log_message(MessageType::LOG, format!("ante-ls did_change: {:?}", params.text_document.uri)).await;
        self.document_map.alter(&params.text_document.uri, |_, mut rope| {
            for change in params.content_changes {
                // `range = None` means a full-document replace
                match change.range.and_then(|r| lsp_range_to_rope_range(r, &rope).ok()) {
                    Some(range) => {
                        rope.remove(range.clone());
                        rope.insert(range.start, &change.text);
                    },
                    None => rope = Rope::from_str(&change.text),
                }
            }
            rope
        });
        if let Some(rope) = self.document_map.get(&params.text_document.uri) {
            self.update_diagnostics(params.text_document.uri, &rope).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::LOG, format!("ante-ls did_save: {:?}", params.text_document.uri)).await;
        if let Some(text) = params.text {
            let rope = Rope::from_str(&text);
            self.document_map.insert(params.text_document.uri.clone(), rope);
        }
        if let Some(rope) = self.document_map.get(&params.text_document.uri) {
            self.update_diagnostics(params.text_document.uri, &rope).await;
        }
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
        let def_rope = rope_for_file(&def_uri, &source_file.contents, &ctx.uri, &ctx.rope, &self.document_map);
        let Ok(range) =
            byte_range_to_lsp_range(ante_loc.span.start.byte_index, ante_loc.span.end.byte_index, &def_rope)
        else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location { uri: def_uri, range })))
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

        let root = match self.root() {
            Some(r) => r,
            None => {
                self.client
                    .log_message(MessageType::ERROR, "resolve_position called before initialize()".to_string())
                    .await;
                return None;
            },
        };
        let file_id = SourceFileId::for_local_path(root, &path);

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
        document_map: DashMap::new(),
        compiler: RwLock::new(Db::default()),
        local_crate_root: OnceLock::new(),
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
