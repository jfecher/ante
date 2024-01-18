use std::path::Path;

use ante::lexer::Lexer;
use ante::parser::{error::ParseError, parse};
use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    document_map: DashMap<Url, Rope>,
}

/// Lets you skip writing the `Type::` part in `..Type::default()` calls
fn default<T: Default>() -> T {
    T::default()
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.client.log_message(MessageType::LOG, format!("ante-ls initialize: {:?}", params)).await;
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL)),
                ..default()
            },
            ..default()
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

fn parser_error_diagnostic(err: ParseError<'_>) -> (&std::path::Path, std::ops::Range<usize>, String) {
    match err {
        ParseError::Fatal(e) => parser_error_diagnostic(*e),
        ParseError::InRule(rule, loc) => {
            (loc.filename, loc.start.index..loc.end.index, format!("failed trying to parse a {}", rule))
        },
        ParseError::LexerError(e, loc) => (loc.filename, loc.start.index..loc.end.index, e.to_string()),
        ParseError::Expected(tokens, loc) => {
            let message = tokens.into_iter().map(|t| format!("\t - {t}")).collect::<Vec<_>>().join("\n");
            (loc.filename, loc.start.index..loc.end.index, format!("expected one of:\n {}", message))
        },
    }
}

impl Backend {
    async fn update_diagnostics(&self, uri: Url, rope: &Rope) {
        let filename = Path::new(uri.path());
        let tokens = Lexer::new(filename, &rope.to_string()).collect::<Vec<_>>();
        match parse(&tokens) {
            Ok(_) => {
                self.client.publish_diagnostics(uri, Vec::new(), None).await;
            },
            Err(err) => {
                let (path, range, message) = parser_error_diagnostic(err);
                let uri = Url::from_file_path(path).unwrap();
                let range = rope_range_to_lsp_range(range, rope);
                self.client
                    .publish_diagnostics(
                        uri,
                        vec![Diagnostic {
                            range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: None,
                            code_description: None,
                            source: Some(String::from("ante-ls")),
                            message,
                            related_information: None,
                            tags: None,
                            data: None,
                        }],
                        None,
                    )
                    .await;
            },
        };
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
