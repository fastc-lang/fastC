//! FastC Language Server implementation

use crate::diagnostics::compile_error_to_diagnostics;
use crate::workspace::Workspace;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

/// Document state stored by the server
pub struct DocumentState {
    pub content: String,
    #[allow(dead_code)]
    pub version: i32,
}

/// FastC Language Server
pub struct FastcLanguageServer {
    client: Client,
    documents: DashMap<Url, DocumentState>,
    workspace: Workspace,
    workspace_root: RwLock<Option<PathBuf>>,
}

impl FastcLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            workspace: Workspace::new(),
            workspace_root: RwLock::new(None),
        }
    }

    /// Validate a document and publish diagnostics
    async fn validate_document(&self, uri: &Url) {
        let Some(doc) = self.documents.get(uri) else {
            return;
        };

        let content = doc.content.clone();
        let filename = uri.path().to_string();
        drop(doc); // Release the lock before async operation

        let diagnostics = match fastc::check(&content, &filename) {
            Ok(()) => vec![],
            Err(e) => compile_error_to_diagnostics(&e, &content),
        };

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    /// Get completions for keywords and builtins
    fn get_keyword_completions(&self) -> Vec<CompletionItem> {
        let keywords = [
            (
                "fn",
                "Function declaration",
                "fn ${1:name}(${2:params}) -> ${3:ReturnType} {\n    $0\n}",
            ),
            (
                "let",
                "Variable declaration",
                "let ${1:name}: ${2:Type} = ${3:value};",
            ),
            ("if", "If statement", "if (${1:condition}) {\n    $0\n}"),
            ("else", "Else branch", "else {\n    $0\n}"),
            ("while", "While loop", "while (${1:condition}) {\n    $0\n}"),
            (
                "for",
                "For loop",
                "for (${1:init}; ${2:cond}; ${3:step}) {\n    $0\n}",
            ),
            ("return", "Return statement", "return ${1:value};"),
            (
                "struct",
                "Struct declaration",
                "struct ${1:Name} {\n    $0\n}",
            ),
            ("enum", "Enum declaration", "enum ${1:Name} {\n    $0\n}"),
            (
                "const",
                "Constant declaration",
                "const ${1:NAME}: ${2:Type} = ${3:value};",
            ),
            ("unsafe", "Unsafe block", "unsafe {\n    $0\n}"),
            ("extern", "Extern block", "extern \"C\" {\n    $0\n}"),
            (
                "switch",
                "Switch statement",
                "switch (${1:expr}) {\n    case ${2:value}:\n        $0\n        break;\n}",
            ),
            ("break", "Break statement", "break;"),
            ("continue", "Continue statement", "continue;"),
            ("defer", "Defer statement", "defer {\n    $0\n}"),
            ("true", "Boolean true", "true"),
            ("false", "Boolean false", "false"),
        ];

        keywords
            .iter()
            .map(|(label, detail, snippet)| CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(detail.to_string()),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
            .collect()
    }

    /// Get completions for primitive types
    fn get_type_completions(&self) -> Vec<CompletionItem> {
        let types = [
            ("i8", "8-bit signed integer"),
            ("i16", "16-bit signed integer"),
            ("i32", "32-bit signed integer"),
            ("i64", "64-bit signed integer"),
            ("u8", "8-bit unsigned integer"),
            ("u16", "16-bit unsigned integer"),
            ("u32", "32-bit unsigned integer"),
            ("u64", "64-bit unsigned integer"),
            ("f32", "32-bit floating point"),
            ("f64", "64-bit floating point"),
            ("bool", "Boolean type"),
            ("usize", "Pointer-sized unsigned integer"),
            ("isize", "Pointer-sized signed integer"),
        ];

        let type_constructors = [
            ("ref", "Non-null immutable reference", "ref(${1:T})"),
            ("mref", "Non-null mutable reference", "mref(${1:T})"),
            ("raw", "Nullable raw pointer (immutable)", "raw(${1:T})"),
            ("rawm", "Nullable raw pointer (mutable)", "rawm(${1:T})"),
            ("own", "Owning pointer", "own(${1:T})"),
            ("slice", "View over contiguous elements", "slice(${1:T})"),
            ("arr", "Fixed-size array", "arr(${1:T}, ${2:N})"),
            ("opt", "Optional value", "opt(${1:T})"),
            ("res", "Result type", "res(${1:T}, ${2:E})"),
        ];

        let mut items: Vec<_> = types
            .iter()
            .map(|(label, detail)| CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some(detail.to_string()),
                ..Default::default()
            })
            .collect();

        items.extend(
            type_constructors
                .iter()
                .map(|(label, detail, snippet)| CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::TYPE_PARAMETER),
                    detail: Some(detail.to_string()),
                    insert_text: Some(snippet.to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                }),
        );

        items
    }

    /// Get completions for builtin functions
    fn get_builtin_completions(&self) -> Vec<CompletionItem> {
        let builtins = [
            ("addr", "Take address of value", "addr(${1:value})"),
            ("deref", "Dereference pointer", "deref(${1:ptr})"),
            ("at", "Array/slice indexing", "at(${1:arr}, ${2:index})"),
            ("cast", "Type cast", "cast(${1:Type}, ${2:value})"),
            ("cstr", "C string literal", "cstr(\"${1:string}\")"),
            ("bytes", "Byte slice literal", "bytes(\"${1:string}\")"),
            ("none", "Empty optional", "none(${1:Type})"),
            ("some", "Wrap in optional", "some(${1:value})"),
            ("ok", "Success result", "ok(${1:value})"),
            ("err", "Error result", "err(${1:value})"),
            ("discard", "Discard value", "discard(${1:value});"),
        ];

        builtins
            .iter()
            .map(|(label, detail, snippet)| CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail.to_string()),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
            .collect()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for FastcLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Extract workspace root for indexing
        if let Some(root_uri) = params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                *self.workspace_root.write().unwrap() = Some(path);
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "fastc-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        // Index workspace files
        let workspace_root = self.workspace_root.read().unwrap().clone();
        if let Some(root) = workspace_root {
            self.workspace.index_workspace(&root);
            self.client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "FastC language server initialized. Indexed workspace at: {}",
                        root.display()
                    ),
                )
                .await;
        } else {
            self.client
                .log_message(MessageType::INFO, "FastC language server initialized")
                .await;
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        // Index file in workspace for cross-file features
        self.workspace.index_file(&uri, &content);

        self.documents
            .insert(uri.clone(), DocumentState { content, version });

        self.validate_document(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        // We use full sync, so there's only one change with the full content
        if let Some(change) = params.content_changes.into_iter().next() {
            let content = change.text;

            // Re-index file in workspace for cross-file features
            self.workspace.index_file(&uri, &content);

            self.documents.insert(
                uri.clone(),
                DocumentState {
                    content,
                    version: params.text_document.version,
                },
            );
        }

        self.validate_document(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
        // Clear diagnostics for closed document
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(doc) = self.documents.get(&uri) else {
            return Ok(None);
        };

        let content = &doc.content;

        // Find the word at the cursor position
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return Ok(None);
        }

        let line = lines[position.line as usize];
        let col = position.character as usize;

        // Extract word at position
        let word_start = line[..col.min(line.len())]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        let word_end = line[col.min(line.len())..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| col + i)
            .unwrap_or(line.len());

        let word = &line[word_start..word_end];

        if word.is_empty() {
            return Ok(None);
        }

        // Look up in workspace index
        if let Some(location) = self.workspace.find_definition(word) {
            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }

        Ok(None)
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut items = self.get_keyword_completions();
        items.extend(self.get_type_completions());
        items.extend(self.get_builtin_completions());

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(doc) = self.documents.get(&uri) else {
            return Ok(None);
        };

        // Find the word at the cursor position
        let content = &doc.content;
        let lines: Vec<&str> = content.lines().collect();

        if position.line as usize >= lines.len() {
            return Ok(None);
        }

        let line = lines[position.line as usize];
        let col = position.character as usize;

        // Extract word at position
        let word_start = line[..col.min(line.len())]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        let word_end = line[col.min(line.len())..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| col + i)
            .unwrap_or(line.len());

        let word = &line[word_start..word_end];

        // Provide hover info for keywords and types
        let hover_text = match word {
            // Types
            "i8" => Some("**i8**\n\n8-bit signed integer (-128 to 127)"),
            "i16" => Some("**i16**\n\n16-bit signed integer (-32,768 to 32,767)"),
            "i32" => Some("**i32**\n\n32-bit signed integer (-2,147,483,648 to 2,147,483,647)"),
            "i64" => Some("**i64**\n\n64-bit signed integer"),
            "u8" => Some("**u8**\n\n8-bit unsigned integer (0 to 255)"),
            "u16" => Some("**u16**\n\n16-bit unsigned integer (0 to 65,535)"),
            "u32" => Some("**u32**\n\n32-bit unsigned integer (0 to 4,294,967,295)"),
            "u64" => Some("**u64**\n\n64-bit unsigned integer"),
            "f32" => Some("**f32**\n\n32-bit IEEE 754 floating point"),
            "f64" => Some("**f64**\n\n64-bit IEEE 754 floating point"),
            "bool" => Some("**bool**\n\nBoolean type (true or false)"),
            "usize" => Some("**usize**\n\nPointer-sized unsigned integer"),
            "isize" => Some("**isize**\n\nPointer-sized signed integer"),

            // Type constructors
            "ref" => Some("**ref(T)**\n\nNon-null immutable reference to T"),
            "mref" => Some("**mref(T)**\n\nNon-null mutable reference to T"),
            "raw" => Some("**raw(T)**\n\nNullable raw pointer (immutable) to T"),
            "rawm" => Some("**rawm(T)**\n\nNullable raw pointer (mutable) to T"),
            "own" => Some("**own(T)**\n\nOwning pointer to T"),
            "slice" => Some("**slice(T)**\n\nView over contiguous elements of type T"),
            "arr" => Some("**arr(T, N)**\n\nFixed-size array of N elements of type T"),
            "opt" => Some("**opt(T)**\n\nOptional value of type T"),
            "res" => Some("**res(T, E)**\n\nResult type: success T or error E"),

            // Keywords
            "fn" => Some("**fn**\n\nFunction declaration keyword"),
            "let" => Some("**let**\n\nVariable declaration keyword"),
            "const" => Some("**const**\n\nConstant declaration keyword"),
            "struct" => Some("**struct**\n\nStruct type declaration keyword"),
            "enum" => Some("**enum**\n\nEnum type declaration keyword"),
            "if" => Some("**if**\n\nConditional statement"),
            "else" => Some("**else**\n\nAlternative branch for if statement"),
            "while" => Some("**while**\n\nWhile loop"),
            "for" => Some("**for**\n\nFor loop"),
            "switch" => Some("**switch**\n\nSwitch statement for pattern matching"),
            "case" => Some("**case**\n\nCase label in switch statement"),
            "default" => Some("**default**\n\nDefault case in switch statement"),
            "return" => Some("**return**\n\nReturn from function"),
            "break" => Some("**break**\n\nBreak out of loop"),
            "continue" => Some("**continue**\n\nContinue to next loop iteration"),
            "defer" => Some("**defer**\n\nDefer execution until scope exit"),
            "unsafe" => Some("**unsafe**\n\nUnsafe block or function marker"),
            "extern" => Some("**extern**\n\nExternal function declaration block"),

            // Builtins
            "addr" => Some("**addr(x)**\n\nTake the address of x"),
            "deref" => Some("**deref(p)**\n\nDereference pointer p"),
            "at" => Some("**at(arr, i)**\n\nIndex into array or slice with bounds checking"),
            "cast" => Some("**cast(T, x)**\n\nCast x to type T"),
            "cstr" => Some("**cstr(\"...\")**\n\nC string literal (null-terminated)"),
            "bytes" => Some("**bytes(\"...\")**\n\nByte slice literal"),
            "none" => Some("**none(T)**\n\nEmpty optional of type T"),
            "some" => Some("**some(x)**\n\nWrap value in optional"),
            "ok" => Some("**ok(x)**\n\nSuccess result with value x"),
            "err" => Some("**err(e)**\n\nError result with error e"),
            "discard" => Some("**discard(x)**\n\nExplicitly discard a value"),
            "true" => Some("**true**\n\nBoolean literal true"),
            "false" => Some("**false**\n\nBoolean literal false"),

            _ => None,
        };

        Ok(hover_text.map(|text| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: text.to_string(),
            }),
            range: Some(Range::new(
                Position::new(position.line, word_start as u32),
                Position::new(position.line, word_end as u32),
            )),
        }))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        let Some(doc) = self.documents.get(&uri) else {
            return Ok(None);
        };

        let content = doc.content.clone();
        let filename = uri.path().to_string();
        drop(doc);

        // Parse the file to extract symbols
        let lexer = fastc::lexer::Lexer::new(&content);
        let tokens: Vec<_> = lexer.collect();
        let mut parser = fastc::parser::Parser::new(&tokens, &content, &filename);

        let ast = match parser.parse_file() {
            Ok(ast) => ast,
            Err(_) => return Ok(None),
        };

        let mut symbols = Vec::new();

        for item in &ast.items {
            match item {
                fastc::ast::Item::Fn(decl) => {
                    let range = Range::new(
                        crate::diagnostics::byte_to_position(&content, decl.span.start),
                        crate::diagnostics::byte_to_position(&content, decl.span.end),
                    );
                    #[allow(deprecated)]
                    symbols.push(DocumentSymbol {
                        name: decl.name.clone(),
                        detail: Some(format_fn_signature(decl)),
                        kind: SymbolKind::FUNCTION,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: None,
                    });
                }
                fastc::ast::Item::Struct(decl) => {
                    let range = Range::new(
                        crate::diagnostics::byte_to_position(&content, decl.span.start),
                        crate::diagnostics::byte_to_position(&content, decl.span.end),
                    );

                    let children: Vec<_> = decl
                        .fields
                        .iter()
                        .map(|field| {
                            let field_range = Range::new(
                                crate::diagnostics::byte_to_position(&content, field.span.start),
                                crate::diagnostics::byte_to_position(&content, field.span.end),
                            );
                            #[allow(deprecated)]
                            DocumentSymbol {
                                name: field.name.clone(),
                                detail: Some(format_type(&field.ty)),
                                kind: SymbolKind::FIELD,
                                tags: None,
                                deprecated: None,
                                range: field_range,
                                selection_range: field_range,
                                children: None,
                            }
                        })
                        .collect();

                    #[allow(deprecated)]
                    symbols.push(DocumentSymbol {
                        name: decl.name.clone(),
                        detail: None,
                        kind: SymbolKind::STRUCT,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: if children.is_empty() {
                            None
                        } else {
                            Some(children)
                        },
                    });
                }
                fastc::ast::Item::Enum(decl) => {
                    let range = Range::new(
                        crate::diagnostics::byte_to_position(&content, decl.span.start),
                        crate::diagnostics::byte_to_position(&content, decl.span.end),
                    );

                    let children: Vec<_> = decl
                        .variants
                        .iter()
                        .map(|variant| {
                            let variant_range = Range::new(
                                crate::diagnostics::byte_to_position(&content, variant.span.start),
                                crate::diagnostics::byte_to_position(&content, variant.span.end),
                            );
                            #[allow(deprecated)]
                            DocumentSymbol {
                                name: variant.name.clone(),
                                detail: None,
                                kind: SymbolKind::ENUM_MEMBER,
                                tags: None,
                                deprecated: None,
                                range: variant_range,
                                selection_range: variant_range,
                                children: None,
                            }
                        })
                        .collect();

                    #[allow(deprecated)]
                    symbols.push(DocumentSymbol {
                        name: decl.name.clone(),
                        detail: None,
                        kind: SymbolKind::ENUM,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: if children.is_empty() {
                            None
                        } else {
                            Some(children)
                        },
                    });
                }
                fastc::ast::Item::Const(decl) => {
                    let range = Range::new(
                        crate::diagnostics::byte_to_position(&content, decl.span.start),
                        crate::diagnostics::byte_to_position(&content, decl.span.end),
                    );
                    #[allow(deprecated)]
                    symbols.push(DocumentSymbol {
                        name: decl.name.clone(),
                        detail: Some(format_type(&decl.ty)),
                        kind: SymbolKind::CONSTANT,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: None,
                    });
                }
                fastc::ast::Item::Opaque(decl) => {
                    let range = Range::new(
                        crate::diagnostics::byte_to_position(&content, decl.span.start),
                        crate::diagnostics::byte_to_position(&content, decl.span.end),
                    );
                    #[allow(deprecated)]
                    symbols.push(DocumentSymbol {
                        name: decl.name.clone(),
                        detail: Some("opaque".to_string()),
                        kind: SymbolKind::TYPE_PARAMETER,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: None,
                    });
                }
                fastc::ast::Item::Extern(block) => {
                    for extern_item in &block.items {
                        match extern_item {
                            fastc::ast::ExternItem::Fn(proto) => {
                                let range = Range::new(
                                    crate::diagnostics::byte_to_position(
                                        &content,
                                        proto.span.start,
                                    ),
                                    crate::diagnostics::byte_to_position(&content, proto.span.end),
                                );
                                #[allow(deprecated)]
                                symbols.push(DocumentSymbol {
                                    name: proto.name.clone(),
                                    detail: Some(format_fn_proto_signature(proto)),
                                    kind: SymbolKind::FUNCTION,
                                    tags: None,
                                    deprecated: None,
                                    range,
                                    selection_range: range,
                                    children: None,
                                });
                            }
                            fastc::ast::ExternItem::Struct(decl) => {
                                let range = Range::new(
                                    crate::diagnostics::byte_to_position(&content, decl.span.start),
                                    crate::diagnostics::byte_to_position(&content, decl.span.end),
                                );
                                #[allow(deprecated)]
                                symbols.push(DocumentSymbol {
                                    name: decl.name.clone(),
                                    detail: Some("extern struct".to_string()),
                                    kind: SymbolKind::STRUCT,
                                    tags: None,
                                    deprecated: None,
                                    range,
                                    selection_range: range,
                                    children: None,
                                });
                            }
                            fastc::ast::ExternItem::Enum(decl) => {
                                let range = Range::new(
                                    crate::diagnostics::byte_to_position(&content, decl.span.start),
                                    crate::diagnostics::byte_to_position(&content, decl.span.end),
                                );
                                #[allow(deprecated)]
                                symbols.push(DocumentSymbol {
                                    name: decl.name.clone(),
                                    detail: Some("extern enum".to_string()),
                                    kind: SymbolKind::ENUM,
                                    tags: None,
                                    deprecated: None,
                                    range,
                                    selection_range: range,
                                    children: None,
                                });
                            }
                            fastc::ast::ExternItem::Opaque(decl) => {
                                let range = Range::new(
                                    crate::diagnostics::byte_to_position(&content, decl.span.start),
                                    crate::diagnostics::byte_to_position(&content, decl.span.end),
                                );
                                #[allow(deprecated)]
                                symbols.push(DocumentSymbol {
                                    name: decl.name.clone(),
                                    detail: Some("extern opaque".to_string()),
                                    kind: SymbolKind::TYPE_PARAMETER,
                                    tags: None,
                                    deprecated: None,
                                    range,
                                    selection_range: range,
                                    children: None,
                                });
                            }
                        }
                    }
                }
                fastc::ast::Item::Use(decl) => {
                    let range = Range::new(
                        crate::diagnostics::byte_to_position(&content, decl.span.start),
                        crate::diagnostics::byte_to_position(&content, decl.span.end),
                    );
                    let name = match &decl.items {
                        fastc::ast::UseItems::Single(item) => {
                            format!("{}::{}", decl.path.join("::"), item)
                        }
                        fastc::ast::UseItems::Multiple(_items) => {
                            format!("{}::{{...}}", decl.path.join("::"))
                        }
                        fastc::ast::UseItems::Glob => {
                            format!("{}::*", decl.path.join("::"))
                        }
                        fastc::ast::UseItems::Module => decl.path.join("::"),
                    };
                    #[allow(deprecated)]
                    symbols.push(DocumentSymbol {
                        name,
                        detail: Some("use".to_string()),
                        kind: SymbolKind::NAMESPACE,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: None,
                    });
                }
                fastc::ast::Item::Mod(decl) => {
                    let range = Range::new(
                        crate::diagnostics::byte_to_position(&content, decl.span.start),
                        crate::diagnostics::byte_to_position(&content, decl.span.end),
                    );
                    #[allow(deprecated)]
                    symbols.push(DocumentSymbol {
                        name: decl.name.clone(),
                        detail: Some(if decl.is_pub { "pub mod" } else { "mod" }.to_string()),
                        kind: SymbolKind::MODULE,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: None,
                    });
                }
            }
        }

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }
}

/// Format a function signature for display
fn format_fn_signature(decl: &fastc::ast::FnDecl) -> String {
    let mut sig = String::new();
    if decl.is_unsafe {
        sig.push_str("unsafe ");
    }
    sig.push_str("fn(");
    for (i, param) in decl.params.iter().enumerate() {
        if i > 0 {
            sig.push_str(", ");
        }
        sig.push_str(&param.name);
        sig.push_str(": ");
        sig.push_str(&format_type(&param.ty));
    }
    sig.push(')');
    if !matches!(decl.return_type, fastc::ast::TypeExpr::Void) {
        sig.push_str(" -> ");
        sig.push_str(&format_type(&decl.return_type));
    }
    sig
}

/// Format a function prototype signature for display
fn format_fn_proto_signature(proto: &fastc::ast::FnProto) -> String {
    let mut sig = String::new();
    if proto.is_unsafe {
        sig.push_str("unsafe ");
    }
    sig.push_str("fn(");
    for (i, param) in proto.params.iter().enumerate() {
        if i > 0 {
            sig.push_str(", ");
        }
        sig.push_str(&param.name);
        sig.push_str(": ");
        sig.push_str(&format_type(&param.ty));
    }
    sig.push(')');
    if !matches!(proto.return_type, fastc::ast::TypeExpr::Void) {
        sig.push_str(" -> ");
        sig.push_str(&format_type(&proto.return_type));
    }
    sig
}

/// Format a type expression for display
fn format_type(ty: &fastc::ast::TypeExpr) -> String {
    match ty {
        fastc::ast::TypeExpr::Primitive(p) => match p {
            fastc::ast::PrimitiveType::I8 => "i8",
            fastc::ast::PrimitiveType::I16 => "i16",
            fastc::ast::PrimitiveType::I32 => "i32",
            fastc::ast::PrimitiveType::I64 => "i64",
            fastc::ast::PrimitiveType::U8 => "u8",
            fastc::ast::PrimitiveType::U16 => "u16",
            fastc::ast::PrimitiveType::U32 => "u32",
            fastc::ast::PrimitiveType::U64 => "u64",
            fastc::ast::PrimitiveType::F32 => "f32",
            fastc::ast::PrimitiveType::F64 => "f64",
            fastc::ast::PrimitiveType::Bool => "bool",
            fastc::ast::PrimitiveType::Usize => "usize",
            fastc::ast::PrimitiveType::Isize => "isize",
        }
        .to_string(),
        fastc::ast::TypeExpr::Named(name) => name.clone(),
        fastc::ast::TypeExpr::Ref(inner) => format!("ref({})", format_type(inner)),
        fastc::ast::TypeExpr::Mref(inner) => format!("mref({})", format_type(inner)),
        fastc::ast::TypeExpr::Raw(inner) => format!("raw({})", format_type(inner)),
        fastc::ast::TypeExpr::Rawm(inner) => format!("rawm({})", format_type(inner)),
        fastc::ast::TypeExpr::Own(inner) => format!("own({})", format_type(inner)),
        fastc::ast::TypeExpr::Slice(inner) => format!("slice({})", format_type(inner)),
        fastc::ast::TypeExpr::Arr(inner, _) => format!("arr({}, N)", format_type(inner)),
        fastc::ast::TypeExpr::Opt(inner) => format!("opt({})", format_type(inner)),
        fastc::ast::TypeExpr::Res(ok, err) => {
            format!("res({}, {})", format_type(ok), format_type(err))
        }
        fastc::ast::TypeExpr::Fn {
            is_unsafe,
            params,
            ret,
        } => {
            let mut sig = String::new();
            if *is_unsafe {
                sig.push_str("unsafe ");
            }
            sig.push_str("fn(");
            for (i, param) in params.iter().enumerate() {
                if i > 0 {
                    sig.push_str(", ");
                }
                sig.push_str(&format_type(param));
            }
            sig.push(')');
            if !matches!(ret.as_ref(), fastc::ast::TypeExpr::Void) {
                sig.push_str(" -> ");
                sig.push_str(&format_type(ret));
            }
            sig
        }
        fastc::ast::TypeExpr::Void => "void".to_string(),
    }
}
