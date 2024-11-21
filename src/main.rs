use {
    lsp_server::{
        Connection,
        Message,
        ProtocolError,
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

fn run(connection: Connection) -> Result<(), ProtocolError> {
    connection.initialize(json!({
        "capabilities": {},
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
        }
    }

    Result::Ok(())
}
