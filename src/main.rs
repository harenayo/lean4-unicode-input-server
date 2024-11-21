use {
    indexmap::map::IndexMap,
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
        de::from_str,
        value::{
            from_value,
            to_value,
        },
    },
    std::convert::identity,
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

    let mut completion: Vec<_> =
        from_str::<Abbreviations>(include_str!(env!("ABBREVIATIONS_JSON")))
            .unwrap()
            .into_iter()
            .map(|(label, text)| CompletionItem {
                label: format!("\\{label}"),
                kind: CompletionItemKind::Snippet,
                insert_text_format: InsertTextFormat::Snippet,
                insert_text_mode: InsertTextMode::AdjustIndentation,
                text_edit: TextEdit {
                    range: Range::default(),
                    new_text: match text.contains("$CURSOR") {
                        true => text.replace("$CURSOR", "$0"),
                        false => text,
                    },
                },
            })
            .collect();

    for message in &connection.receiver {
        if let Message::Request(request) = message {
            if connection.handle_shutdown(&request).unwrap() {
                break;
            }

            let (result, error) = match match request.method.as_str() {
                "textDocument/completion" => (|| {
                    let params = from_value::<CompletionParams>(request.params)?;

                    match (params.context.trigger_kind != CompletionTriggerKind::TriggerCharacter)
                        || (params.context.trigger_character == "\\")
                    {
                        true => {
                            let mut range = Range {
                                start: params.position,
                                end: params.position,
                            };

                            range.start.character -= 1;

                            for item in &mut completion {
                                item.text_edit.range = range;
                            }

                            to_value(&completion)
                        },
                        false => to_value(identity::<[CompletionItem; 0]>([])),
                    }
                })()
                .map_err(|_| ResponseError {
                    code: ErrorCode::InvalidParams as i32,
                    message: "parameters are invalid".to_owned(),
                    data: Option::None,
                }),
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

#[derive(Clone, Copy, Default, Serialize)]
struct Range {
    start: Position,
    end: Position,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TextEdit {
    range: Range,
    new_text: String,
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

#[derive(Deserialize)]
struct CompletionParams {
    position: Position,
    context: CompletionContext,
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "u32")]
enum CompletionTriggerKind {
    Invoked = 1,
    TriggerCharacter = 2,
    TriggerForIncompleteCompletions = 3,
}

impl TryFrom<u32> for CompletionTriggerKind {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            _ if value == Self::Invoked as u32 => Result::Ok(Self::Invoked),
            _ if value == Self::TriggerCharacter as u32 => Result::Ok(Self::TriggerCharacter),
            _ if value == Self::TriggerForIncompleteCompletions as u32 => {
                Result::Ok(Self::TriggerForIncompleteCompletions)
            },
            _ => Result::Err(format!("{value} is not CompletionTriggerKind")),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletionContext {
    trigger_kind: CompletionTriggerKind,
    trigger_character: String,
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
    insert_text_format: InsertTextFormat,
    insert_text_mode: InsertTextMode,
    text_edit: TextEdit,
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

type Abbreviations = IndexMap<String, String>;
