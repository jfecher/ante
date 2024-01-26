use std::{
    collections::HashMap,
    env::set_current_dir,
    path::{Path, PathBuf},
};

use ante::{
    cache::{cached_read, ModuleCache},
    error::{location::Locatable, ErrorType},
    lexer::Lexer,
    nameresolution::NameResolver,
    parser::parse,
    types::typechecker,
};

use dashmap::DashMap;
use futures::future::join_all;
use ropey::Rope;
use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer, LspService, Server};

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
                    let range = lsp_range_to_rope_range(range, &rope);
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
}

fn lsp_range_to_rope_range(range: Range, rope: &Rope) -> std::ops::Range<usize> {
    let start_line = range.start.line as usize;
    let start_line = rope.line_to_char(start_line);

    let start_char = range.start.character as usize;
    let start_char = start_line + start_char;

    let end_line = range.end.line as usize;
    let end_line = rope.line_to_char(end_line);

    let end_char = range.end.character as usize;
    let end_char = end_line + end_char;

    start_char..end_char
}

fn rope_range_to_lsp_range(range: std::ops::Range<usize>, rope: &Rope) -> Range {
    let start_line = rope.char_to_line(range.start);
    let start_char = rope.line_to_char(start_line);
    let start_char = (range.start - start_char) as u32;
    let start_line = start_line as u32;

    let end_line = rope.char_to_line(range.end);
    let end_char = rope.line_to_char(end_line);
    let end_char = (range.end - end_char) as u32;
    let end_line = end_line as u32;

    Range {
        start: Position { line: start_line, character: start_char },
        end: Position { line: end_line, character: end_char },
    }
}

impl Backend {
    async fn update_diagnostics(&self, uri: Url, rope: &Rope) {
        // Urls always contain ablsoute canonical paths, so there's no need to canonicalize them.
        let filename = Path::new(uri.path());
        let cache_root = filename.parent().unwrap();

        let (paths, contents) =
            self.document_map.iter().fold((Vec::new(), Vec::new()), |(mut paths, mut contents), item| {
                paths.push(PathBuf::from(item.key().path()));
                contents.push(item.value().to_string());
                (paths, contents)
            });
        let file_cache = paths.iter().zip(contents.into_iter()).map(|(p, c)| (p.as_path(), c)).collect();
        let mut cache = ModuleCache::new(cache_root, file_cache);

        let tokens = Lexer::new(filename, &rope.to_string()).collect::<Vec<_>>();
        match parse(&tokens) {
            Ok(ast) => {
                NameResolver::start(ast, &mut cache);
                if cache.error_count() == 0 {
                    let ast = cache.parse_trees.get_mut(0).unwrap();
                    typechecker::infer_ast(ast, &mut cache);
                }
            },
            Err(err) => {
                cache.push_full_diagnostic(err.into_diagnostic());
            },
        };

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

            let range = rope_range_to_lsp_range(loc.start.index..loc.end.index, &rope);
            let message = format!("{}", diagnostic.display(&cache));

            let diagnostic = Diagnostic {
                code: None,
                code_description: None,
                data: None,
                message,
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

        for (path, content) in cache.file_cache {
            let uri = Url::from_file_path(path).unwrap();
            if self.document_map.get(&uri).is_none() {
                self.document_map.insert(uri.clone(), Rope::from_str(&content));
            }
        }

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
