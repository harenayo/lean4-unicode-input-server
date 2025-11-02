use {
    ::indexmap::map::IndexMap,
    ::lsp_server::{
        Connection,
        ErrorCode,
        Message,
        Response,
        ResponseError,
    },
    ::lsp_types::{
        CompletionItem,
        CompletionItemKind,
        CompletionOptions,
        CompletionParams,
        CompletionTextEdit,
        CompletionTriggerKind,
        InitializeResult,
        InsertTextFormat,
        InsertTextMode,
        Range,
        ServerCapabilities,
        ServerInfo,
        TextEdit,
    },
    ::rancor::{
        BoxedError,
        ResultExt as _,
    },
    ::serde_json::{
        de::from_str,
        value::{
            Value,
            from_value,
            to_value,
        },
    },
    ::std::convert::identity,
};

fn main() -> Result<(), BoxedError> {
    let (connection, threads) = Connection::stdio();

    connection
        .initialize_finish(
            connection.initialize_start().into_error()?.0,
            to_value(InitializeResult {
                capabilities: ServerCapabilities {
                    completion_provider: Option::Some(CompletionOptions {
                        trigger_characters: Option::Some([r"\".to_owned()].to_vec()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                server_info: Option::Some(ServerInfo {
                    name: env!("CARGO_PKG_NAME").to_owned(),
                    version: Option::Some(env!("CARGO_PKG_VERSION").to_owned()),
                }),
            })
            .into_error()?,
        )
        .into_error()?;

    let mut completion: Vec<_> =
        from_str::<IndexMap<String, String>>(include_str!("../abbreviations.json"))
            .into_error()?
            .into_iter()
            .map(|(label, text)| CompletionItem {
                label: format!("\\{label}"),
                kind: Option::Some(CompletionItemKind::SNIPPET),
                insert_text_format: Option::Some(InsertTextFormat::SNIPPET),
                insert_text_mode: Option::Some(InsertTextMode::ADJUST_INDENTATION),
                text_edit: Option::Some(CompletionTextEdit::Edit(TextEdit {
                    range: Default::default(),
                    new_text: match text.contains("$CURSOR") {
                        true => text.replace("$CURSOR", "$0"),
                        false => text,
                    },
                })),
                ..Default::default()
            })
            .collect();

    for message in &connection.receiver {
        let Message::Request(request) = message else {
            continue;
        };

        if connection.handle_shutdown(&request).into_error()? {
            break;
        }

        let result = 'result: {
            if request.method != "textDocument/completion" {
                break 'result Result::Err(ResponseError {
                    code: ErrorCode::MethodNotFound as i32,
                    message: format!("'{}' is not found", request.method),
                    data: Option::None,
                });
            }

            let params = match from_value::<CompletionParams>(request.params) {
                Result::Ok(params) => params,
                Result::Err(error) => {
                    break 'result Result::Err(ResponseError {
                        code: ErrorCode::InvalidParams as i32,
                        message: "parameters are invalid".to_owned(),
                        data: Option::Some(Value::String(error.to_string())),
                    });
                },
            };

            Result::Ok(
                match params.context {
                    Option::Some(context)
                        if context.trigger_kind != CompletionTriggerKind::TRIGGER_CHARACTER
                            || context
                                .trigger_character
                                .as_ref()
                                .is_some_and(|s| s == r"\") =>
                    {
                        let position = params.text_document_position.position;

                        let mut range = Range {
                            start: position,
                            end: position,
                        };

                        range.start.character -= 1;

                        for item in &mut completion {
                            if let Option::Some(CompletionTextEdit::Edit(text_edit)) =
                                &mut item.text_edit
                            {
                                text_edit.range = range;
                            }
                        }

                        to_value(&completion)
                    },
                    _ => to_value(identity::<[CompletionItem; 0]>([])),
                }
                .into_error()?,
            )
        };

        let (result, error) = match result {
            Result::Ok(value) => (Option::Some(value), Option::None),
            Result::Err(error) => (Option::None, Option::Some(error)),
        };

        connection
            .sender
            .send(Message::Response(Response {
                id: request.id,
                result,
                error,
            }))
            .into_error()?;
    }

    threads.join().into_error()?;
    Result::Ok(())
}
