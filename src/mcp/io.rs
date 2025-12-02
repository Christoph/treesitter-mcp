use std::io::{BufRead, Write};
use eyre::{Result, WrapErr};
use crate::mcp::json_rpc::{Message, Response};

/// Read a JSON-RPC message from a buffered reader
///
/// Messages are expected to be line-delimited (one message per line).
/// The MCP protocol uses stdio transport with newline-delimited JSON messages.
///
/// # Errors
/// Returns an error if:
/// - Reading from input fails (I/O error)
/// - The line is not valid JSON (parse error)
/// - The JSON doesn't match the Message schema (deserialization error)
///
/// # Example
/// ```no_run
/// use std::io::BufReader;
/// use treesitter_cli::mcp::io::read_message;
///
/// let stdin = std::io::stdin();
/// let mut reader = BufReader::new(stdin.lock());
/// let message = read_message(&mut reader)?;
/// # Ok::<(), eyre::Report>(())
/// ```
pub fn read_message<R: BufRead>(reader: &mut R) -> Result<Message> {
    let mut line = String::new();
    reader.read_line(&mut line)
        .wrap_err("Failed to read line from input")?;

    if line.is_empty() {
        eyre::bail!("Reached end of input stream");
    }

    let line = line.trim();
    let msg = serde_json::from_str(line)
        .wrap_err_with(|| format!("Failed to parse JSON-RPC message: {line}"))?;

    Ok(msg)
}

/// Write a JSON-RPC response to an output stream
///
/// The response is serialized as compact JSON (no whitespace) followed by a newline.
/// This format is required by the MCP protocol for token efficiency.
///
/// The output is flushed after writing to ensure immediate delivery.
///
/// # Errors
/// Returns an error if:
/// - Serialization fails (should not happen with valid Response)
/// - Writing to the output stream fails (I/O error)
/// - Flushing the output fails
///
/// # Example
/// ```no_run
/// use serde_json::json;
/// use treesitter_cli::mcp::json_rpc::Response;
/// use treesitter_cli::mcp::io::write_message;
///
/// let response = Response {
///     jsonrpc: "2.0".to_string(),
///     id: json!(1),
///     result: Some(json!({"status": "ok"})),
///     error: None,
/// };
///
/// let stdout = std::io::stdout();
/// let mut writer = stdout.lock();
/// write_message(&mut writer, &response)?;
/// # Ok::<(), eyre::Report>(())
/// ```
pub fn write_message<W: Write>(writer: &mut W, response: &Response) -> Result<()> {
    // Serialize to compact JSON (no whitespace)
    let json = serde_json::to_string(response)
        .wrap_err("Failed to serialize response to JSON")?;

    // Write with newline delimiter
    writeln!(writer, "{json}")
        .wrap_err("Failed to write message to output")?;

    // Flush immediately to ensure message is sent
    writer.flush()
        .wrap_err("Failed to flush output stream")?;

    Ok(())
}
