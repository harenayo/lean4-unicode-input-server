use {
    lsp_server::{
        Connection,
        ErrorCode,
        Message,
        Response,
        ResponseError,
    },
    serde_json::json,
    std::collections::HashMap,
};

fn main() {
    let (connection, threads) = Connection::stdio();

    connection
        .initialize_finish(
            connection.initialize_start().unwrap().0,
            json!({
                "capabilities": {
                    "positionEncoding": "utf-8",
                    "textDocumentSync": 1,
                    "completionProvider": {
                        "triggerCharacters": ["\\"],
                    },
                },
                "serverInfo": {
                    "name": env!("CARGO_PKG_NAME"),
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }),
        )
        .unwrap();

    let mut files: HashMap<_, Vec<String>> = HashMap::new();

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request).unwrap() {
                    break;
                }

                let (result, error) = match match request.method.as_str() {
                    "textDocument/completion" => match (|| {
                        let params = request.params.as_object()?;
                        let position = params.get("position")?.as_object()?;

                        Option::Some((
                            params
                                .get("textDocument")?
                                .as_object()?
                                .get("uri")?
                                .as_str()?,
                            position.get("line")?.as_u64()?,
                            position.get("character")?.as_u64()?,
                        ))
                    })() {
                        Option::Some((uri, line, character)) => {
                            let _str = &files[uri][line as usize][character as usize..];

                            Result::Ok(json!([
                                {
                                    "label": "{}",
                                    "insertText": "{$0}",
                                    "insertTextFormat": 2,
                                    "insertTextMode": 2,
                                },
                                {
                                    "label": "{}_",
                                    "insertText": "{$0}_",
                                    "insertTextFormat": 2,
                                    "insertTextMode": 1,
                                },
                                {
                                    "label": "{{}}",
                                    "insertText": "⦃$1⦄",
                                    "insertTextFormat": 2,
                                    "insertTextMode": 2,
                                },
                                {
                                    "label": "\\",
                                    "insertText": "\\",
                                    "insertTextFormat": 1,
                                },
                                {
                                    "label": "a",
                                    "kind": 1,
                                    "insertText": "α",
                                    "insertTextFormat": 1,
                                },
                                {
                                    "label": "b",
                                    "kind": 6,
                                    "insertText": "β",
                                    "insertTextFormat": 1,
                                },
                                {
                                    "label": "c",
                                    "kind": 15,
                                    "insertText": "χ",
                                    "insertTextFormat": 1,
                                },
                            ]))
                        },
                        Option::None => Result::Err(ResponseError {
                            code: ErrorCode::InvalidParams as i32,
                            message: "parameters are invalid".to_owned(),
                            data: Option::None,
                        }),
                    },
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
            },
            Message::Notification(notification) => (|| {
                let (uri, text) = match notification.method.as_str() {
                    "textDocument/didOpen" => {
                        let document = notification
                            .params
                            .as_object()?
                            .get("textDocument")?
                            .as_object()?;

                        Option::Some((
                            document.get("uri")?.as_str()?,
                            Option::Some(document.get("text")?.as_str()?),
                        ))
                    },
                    "textDocument/didChange" => {
                        let params = notification.params.as_object()?;

                        Option::Some((
                            params
                                .get("textDocument")?
                                .as_object()?
                                .get("uri")?
                                .as_str()?,
                            Option::Some(
                                params
                                    .get("contentChanges")?
                                    .as_array()?
                                    .last()?
                                    .as_object()?
                                    .get("text")?
                                    .as_str()?,
                            ),
                        ))
                    },
                    "textDocument/didClose" => Option::Some((
                        notification
                            .params
                            .as_object()?
                            .get("textDocument")?
                            .as_object()?
                            .get("uri")?
                            .as_str()?,
                        Option::None,
                    )),
                    _ => Option::None,
                }?;

                match text {
                    Option::Some(text) => {
                        files.insert(
                            uri.to_owned(),
                            text.lines().map(str::to_owned).collect::<Vec<_>>(),
                        );
                    },
                    Option::None => {
                        files.remove(uri);
                    },
                }

                Option::None
            })()
            .unwrap_or_default(),
            _ => (),
        }
    }

    threads.join().unwrap();
}
