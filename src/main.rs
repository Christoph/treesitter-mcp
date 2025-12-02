use color_eyre::Result;
use std::io::{stdin, stdout, BufReader};
use treesitter_mcp::mcp::io::{read_message, write_message};
use treesitter_mcp::mcp::server::McpServer;

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    log::info!("Tree-sitter MCP Server starting");

    let mut server = McpServer::new();
    let stdin = stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = stdout();
    let mut writer = stdout.lock();

    loop {
        // Read message from stdin
        let message = match read_message(&mut reader) {
            Ok(msg) => msg,
            Err(e) => {
                // Check for EOF
                if e.to_string().contains("Reached end of input stream") {
                    log::info!("Client disconnected (EOF)");
                    break;
                }
                log::error!("Failed to read message: {}", e);
                continue;
            }
        };

        // Process message and send response if needed
        match server.process_message(message) {
            Ok(Some(response)) => {
                if let Err(e) = write_message(&mut writer, &response) {
                    log::error!("Failed to write response: {}", e);
                }
            }
            Ok(None) => {
                // Notification handling, no response required
            }
            Err(e) => {
                log::error!("Internal error processing message: {}", e);
            }
        }
    }

    log::info!("Tree-sitter MCP Server stopping");

    Ok(())
}
