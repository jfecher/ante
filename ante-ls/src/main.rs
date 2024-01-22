use std::{
    collections::HashMap,
    env::{current_dir, set_current_dir},
    path::{Path, PathBuf},
};

use ante::{
    cache::ModuleCache,
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

    // The diagnostics can't be updated on change, because the content of the file in the file system
    // is not guaranteed to be the same as the content of the file in the editor. This will result in
    // a panic when running Diagnostic::format, and the column are lengths different than expected.
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
    }
}

fn relative_path<P1: AsRef<Path>, P2: AsRef<Path>>(root: P1, path: P2) -> Option<PathBuf> {
    let path = path.as_ref();
    if let Ok(path) = path.strip_prefix(&root) {
        return Some(path.to_path_buf());
    }

    let mut acc = PathBuf::new();
    let mut root = root.as_ref();
    loop {
        if let Ok(path) = path.strip_prefix(root) {
            acc.push(path);
            return Some(acc);
        } else {
            acc.push("..");
            root = root.parent()?;
        }
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
        let root = match current_dir() {
            Ok(root) => root,
            Err(_) => {
                self.client.log_message(MessageType::ERROR, "Failed to get current directory".to_string()).await;
                return;
            },
        };
        // We want the filename to be relative to the root for nicer error messages.
        // This could fail on windows when the root is on a different drive than the file.
        let filename = Path::new(uri.path());
        let filename = relative_path(&root, filename).unwrap();

        let mut cache = ModuleCache::new(filename.parent().unwrap());
        let tokens = Lexer::new(&filename, &rope.to_string()).collect::<Vec<_>>();
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
        let mut diagnostics = HashMap::from([(uri, Vec::new())]);

        for diagnostic in cache.get_diagnostics() {
            let severity = Some(match diagnostic.error_type() {
                ErrorType::Note => DiagnosticSeverity::HINT,
                ErrorType::Warning => DiagnosticSeverity::WARNING,
                ErrorType::Error => DiagnosticSeverity::ERROR,
            });

            let loc = diagnostic.locate();
            let filename = root.join(loc.filename);
            let filename = match filename.canonicalize() {
                Ok(filename) => filename,
                Err(_) => {
                    self.client
                        .log_message(
                            MessageType::ERROR,
                            format!("Diagnostics for file {filename:?}, but its path could not be canonicalized"),
                        )
                        .await;
                    continue;
                },
            };
            let uri = Url::from_file_path(filename).unwrap();

            let rope = match self.document_map.get(&uri) {
                Some(rope) => rope,
                None => {
                    // Can we somehow retrieve the file from the compiler rather than reading it again?
                    // Or have the compiler go through the lsp server file buffer instead of reading it from the file system?
                    let rope = Rope::from_str(&std::fs::read_to_string(uri.path()).unwrap());
                    self.document_map.insert(uri.clone(), rope.clone());
                    self.document_map.get(&uri).unwrap()
                },
            };
            let range = rope_range_to_lsp_range(loc.start.index..loc.end.index, &rope);

            let message = format!("{}", diagnostic.display());

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

        join_all(
            diagnostics.into_iter().map(|(uri, diagnostics)| self.client.publish_diagnostics(uri, diagnostics, None)),
        )
        .await;
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
