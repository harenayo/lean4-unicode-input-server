use {
    lsp_server::{
        Connection,
        Message,
        Response,
    },
    serde_json::json,
    std::error::Error,
};

fn main() -> Result<(), Box<dyn Error>> {
    let (connection, threads) = Connection::stdio();
    run(connection)?;
    threads.join()?;
    Result::Ok(())
}

fn run(connection: Connection) -> Result<(), Box<dyn Error>> {
    connection.initialize(json!({
        "capabilities": {
            "completionProvider": {
                "triggerCharacters": "\\",
            },
        },
        "serverInfo": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        },
    }))?;

    for message in &connection.receiver {
        if let Message::Request(request) = message {
            if connection.handle_shutdown(&request)? {
                break;
            }

            if request.method == "textDocument/completion" {
                connection.sender.send(Message::Response(Response {
                    id: request.id,
                    result: Option::Some(json!([{
                        "label": "DUMMY",
                    }])),
                    error: Option::None,
                }))?;
            }
        }
    }

    Result::Ok(())
}
