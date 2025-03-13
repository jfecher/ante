use std::{
    collections::HashMap,
    env::set_current_dir,
    path::{Path, PathBuf},
};

use ante::{
    cache::{cached_read, ModuleCache},
    error::{location::Locatable, ErrorType},
    frontend,
    parser::ast::Ast,
    types::typeprinter,
};

use dashmap::DashMap;
use futures::future::join_all;
use ropey::Rope;
use tower_lsp::{
    jsonrpc::{Error, ErrorCode, Result},
    lsp_types::*,
    Client, LanguageServer, LspService, Server,
};

mod util;
use util::{lsp_range_to_rope_range, node_at_index, position_to_index, rope_range_to_lsp_range};

#[derive(Debug)]
struct Backend {
    client: Client,
    document_map: DashMap<Url, Rope>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.client.log_message(MessageType::LOG, format!("ante-ls initialize: {:?}", params)).await;
        if let Some(root_uri) = params.root_uri {
            let root = PathBuf::from(root_uri.path());
            if set_current_dir(&root).is_err() {
                self.client
                    .log_message(MessageType::ERROR, format!("Failed to set root directory to {:?}", root))
                    .await;
            };
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
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

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::LOG, format!("ante-ls did_save: {:?}", params)).await;
        if let Some(text) = params.text {
            let rope = Rope::from_str(&text);
            self.document_map.insert(params.text_document.uri.clone(), rope);
        }
        if let Some(rope) = self.document_map.get(&params.text_document.uri) {
            self.update_diagnostics(params.text_document.uri, &rope).await;
        };
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client.log_message(MessageType::LOG, format!("ante-ls did_open: {:?}", params)).await;
        let rope = Rope::from_str(&params.text_document.text);
        self.document_map.insert(params.text_document.uri.clone(), rope.clone());
        self.update_diagnostics(params.text_document.uri, &rope).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client.log_message(MessageType::LOG, format!("ante_ls did_change: {:?}", params)).await;
        self.document_map.alter(&params.text_document.uri, |_, mut rope| {
            for change in params.content_changes {
                if let Some(range) = change.range {
                    let range = lsp_range_to_rope_range(range, &rope).unwrap();
                    rope.remove(range.clone());
                    rope.insert(range.start, &change.text);
                } else {
                    rope = Rope::from_str(&change.text)
                }
            }
            rope
        });
        if let Some(rope) = self.document_map.get(&params.text_document.uri) {
            self.update_diagnostics(params.text_document.uri, &rope).await;
        };
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.client.log_message(MessageType::LOG, format!("ante-ls hover: {:?}", params)).await;
        let uri = params.text_document_position_params.text_document.uri;
        let rope = match self.document_map.get(&uri) {
            Some(rope) => rope,
            None => return Ok(None),
        };

        let cache = self.create_cache(&uri, &rope);
        let ast = match cache.parse_trees.get_mut(0) {
            Some(ast) => ast,
            None => return Ok(None),
        };

        let index = position_to_index(params.text_document_position_params.position, &rope).map_err(|_| Error {
            code: ErrorCode::InternalError,
            message: "Failed to convert hover position to range".into(),
            data: None,
        })?;
        let hovered_node = node_at_index(ast, index);

        let result = match hovered_node {
            Ast::Variable(v) => {
                let info = match v.definition {
                    Some(definition_id) => &cache[definition_id],
                    _ => return Ok(None),
                };

                let typ = match &info.typ {
                    Some(typ) => typ,
                    None => return Ok(None),
                };

                let name = v.kind.name();

                let value = typeprinter::show_type_and_traits(
                    &name,
                    typ,
                    &info.required_traits,
                    &info.trait_info,
                    &cache,
                    false,
                );

                let location = v.locate();
                let range =
                    Some(rope_range_to_lsp_range(location.start.index..location.end.index, &rope).map_err(|_| {
                        Error {
                            code: ErrorCode::InternalError,
                            message: "Failed to convert range to hover location".into(),
                            data: None,
                        }
                    })?);

                Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::PlainText, value }),
                    range,
                }))
            },
            _ => Ok(None),
        };

        self.save_cache(cache);
        result
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        self.client.log_message(MessageType::LOG, format!("ante-ls goto_definition: {:?}", params)).await;
        let uri = params.text_document_position_params.text_document.uri;
        let rope = match self.document_map.get(&uri) {
            Some(rope) => rope,
            None => return Ok(None),
        };

        let cache = self.create_cache(&uri, &rope);
        let ast = match cache.parse_trees.get_mut(0) {
            Some(ast) => ast,
            None => return Ok(None),
        };

        let index = position_to_index(params.text_document_position_params.position, &rope).map_err(|_| Error {
            code: ErrorCode::InternalError,
            message: "Failed to convert hover position to range".into(),
            data: None,
        })?;
        let hovered_node = node_at_index(ast, index);

        let result = match hovered_node {
            Ast::Variable(v) => {
                let info = match v.definition {
                    Some(definition_id) => &cache[definition_id],
                    _ => return Ok(None),
                };
                let loc = info.location;
                let uri = match Url::from_file_path(loc.filename) {
                    Ok(uri) => uri,
                    Err(_) => {
                        return Err(Error {
                            code: ErrorCode::InternalError,
                            message: "Failed to convert path to uri".into(),
                            data: None,
                        })
                    },
                };
                let rope = match self.document_map.get(&uri) {
                    Some(rope) => rope,
                    None => {
                        let contents = cached_read(&cache.file_cache, loc.filename).unwrap();
                        let rope = Rope::from_str(&contents);
                        self.document_map.insert(uri.clone(), rope);
                        self.document_map.get(&uri).unwrap()
                    },
                };

                let range = loc.start.index..loc.end.index;
                let range = rope_range_to_lsp_range(range, &rope).map_err(|_| Error {
                    code: ErrorCode::InternalError,
                    message: "Failed to convert range to definition location".into(),
                    data: None,
                })?;

                Ok(Some(GotoDefinitionResponse::Scalar(Location { uri, range })))
            },
            _ => Ok(None),
        };

        self.save_cache(cache);
        result
    }
}

impl Backend {
    fn create_cache<'a>(&self, uri: &'a Url, rope: &Rope) -> ModuleCache<'a> {
        // Urls always contain ablsoute canonical paths, so there's no need to canonicalize them.
        let filename = Path::new(uri.path());
        let cache_root = filename.parent().unwrap();

        let file_cache =
            self.document_map.iter().map(|item| (PathBuf::from(item.key().path()), item.value().to_string())).collect();
        let mut cache = ModuleCache::new(cache_root, file_cache);

        let _ = frontend::check(filename, rope.to_string(), &mut cache, frontend::FrontendPhase::TypeCheck, false);

        cache
    }

    fn save_cache(&self, cache: ModuleCache) {
        for (path, content) in cache.file_cache {
            let uri = Url::from_file_path(path).unwrap();
            if self.document_map.get(&uri).is_none() {
                self.document_map.insert(uri.clone(), Rope::from_str(&content));
            }
        }
    }

    async fn update_diagnostics(&self, uri: Url, rope: &Rope) {
        let cache = self.create_cache(&uri, rope);

        // Diagnostics for a document get cleared only when an empty list is sent for it's Uri.
        // This presents an issue, as when we have files A and B, where file A imports the file B,
        // and we provide a diagnostic for file A about incorrect usage of a function in file B,
        // the diagnostic will not be cleared when we update  file B, as the compiler currently
        // has no way of knowing that file A imports file B. Because of this, we're initialising
        // the diagnostics with an empty list only for the current file, and not for all files,
        // as we don't want to clear the diagnostics for errors unrelated to changes we made.
        // The diagnostics for file A will only be updated when the function is ran against that file,
        // ie. when it's saved or reopened. Once ante gets a way of defining projects, and there's a way
        // to generate a list of files in one, we could run the compiler on the root of the project.
        // That should provide an exhaustive list of diagnostics, and allow us to clear all diagnostics
        // for files that had none in the new list.
        let mut diagnostics = HashMap::from([(uri.clone(), Vec::new())]);

        for diagnostic in cache.get_diagnostics() {
            let severity = Some(match diagnostic.error_type() {
                ErrorType::Note => DiagnosticSeverity::HINT,
                ErrorType::Warning => DiagnosticSeverity::WARNING,
                ErrorType::Error => DiagnosticSeverity::ERROR,
            });

            let loc = diagnostic.locate();
            let uri = Url::from_file_path(loc.filename).unwrap();

            let rope = match self.document_map.get(&uri) {
                Some(rope) => rope,
                None => {
                    let contents = cached_read(&cache.file_cache, loc.filename).unwrap();
                    let rope = Rope::from_str(&contents);
                    self.document_map.insert(uri.clone(), rope);
                    self.document_map.get(&uri).unwrap()
                },
            };

            let range = match rope_range_to_lsp_range(loc.start.index..loc.end.index, &rope) {
                Ok(range) => range,
                Err(e) => {
                    self.client.log_message(MessageType::ERROR, format!("Failed to convert range: {:?}", e)).await;
                    return;
                },
            };

            let diagnostic = Diagnostic {
                code: None,
                code_description: None,
                data: None,
                message: diagnostic.msg().to_string(),
                range,
                related_information: None,
                severity,
                source: Some(String::from("ante-ls")),
                tags: None,
            };

            match diagnostics.get_mut(&uri) {
                Some(diagnostics) => diagnostics.push(diagnostic),
                None => {
                    diagnostics.insert(uri, vec![diagnostic]);
                },
            };
        }

        let handle = join_all(
            diagnostics.into_iter().map(|(uri, diagnostics)| self.client.publish_diagnostics(uri, diagnostics, None)),
        );

        self.save_cache(cache);

        handle.await;
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend { client, document_map: DashMap::new() }).finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
