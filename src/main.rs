use {
    lsp_server::{
        Connection,
        ErrorCode,
        Message,
        Response,
        ResponseError,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    serde_json::{
        from_str,
        to_value,
    },
    std::collections::HashMap,
};

fn main() {
    let (connection, threads) = Connection::stdio();

    connection
        .initialize_finish(
            connection.initialize_start().unwrap().0,
            to_value(InitializeResult {
                capabilities: ServerCapabilities {
                    completion_provider: CompletionOptions {
                        trigger_characters: ["\\"],
                    },
                },
                server_info: ServerInfo {
                    name: env!("CARGO_PKG_NAME"),
                    version: env!("CARGO_PKG_VERSION"),
                },
            })
            .unwrap(),
        )
        .unwrap();

    let completion = to_value(
        from_str::<Abbreviations>(include_str!(env!("ABBREVIATIONS_JSON")))
            .unwrap()
            .into_iter()
            .map(|(label, text)| CompletionItem {
                label: format!("\\{label}"),
                kind: CompletionItemKind::Snippet,
                insert_text: match text.contains("$CURSOR") {
                    true => text.replace("$CURSOR", "$0"),
                    false => text,
                },
                insert_text_format: InsertTextFormat::Snippet,
                insert_text_mode: InsertTextMode::AdjustIndentation,
                commit_characters: [" ", "\n"],
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();

    for message in &connection.receiver {
        if let Message::Request(request) = message {
            if connection.handle_shutdown(&request).unwrap() {
                break;
            }

            let (result, error) = match match request.method.as_str() {
                "textDocument/completion" => Result::Ok(completion.clone()),
                _ => Result::Err(ResponseError {
                    code: ErrorCode::MethodNotFound as i32,
                    message: "a method is not found".to_owned(),
                    data: Option::None,
                }),
            } {
                Result::Ok(result) => (Option::Some(result), Option::None),
                Result::Err(error) => (Option::None, Option::Some(error)),
            };

            connection
                .sender
                .send(Message::Response(Response {
                    id: request.id,
                    result,
                    error,
                }))
                .unwrap();
        }
    }

    threads.join().unwrap();
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
struct Position {
    line: u32,
    character: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitializeResult {
    capabilities: ServerCapabilities,
    server_info: ServerInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerCapabilities {
    completion_provider: CompletionOptions,
}

#[derive(Serialize)]
struct ServerInfo {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompletionOptions {
    trigger_characters: [&'static str; 1],
}

#[derive(Clone, Serialize)]
#[serde(into = "u32")]
enum InsertTextFormat {
    Snippet = 2,
}

impl From<InsertTextFormat> for u32 {
    fn from(value: InsertTextFormat) -> Self {
        value as Self
    }
}

#[derive(Clone, Serialize)]
#[serde(into = "u32")]
enum InsertTextMode {
    AdjustIndentation = 2,
}

impl From<InsertTextMode> for u32 {
    fn from(value: InsertTextMode) -> Self {
        value as Self
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompletionItem {
    label: String,
    kind: CompletionItemKind,
    insert_text: String,
    insert_text_format: InsertTextFormat,
    insert_text_mode: InsertTextMode,
    commit_characters: [&'static str; 2],
}

#[derive(Clone, Serialize)]
#[serde(into = "u32")]
enum CompletionItemKind {
    Snippet = 15,
}

impl From<CompletionItemKind> for u32 {
    fn from(value: CompletionItemKind) -> Self {
        value as Self
    }
}

type Abbreviations = HashMap<String, String>;
