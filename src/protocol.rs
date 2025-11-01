//! Line-based text protocol for the ring server.
//!
//! Commands (one per line, newline-terminated):
//!   - "SET_NEXT <addr>"
//!   - "GET"
//!   - "RING <ttl> <message...>"
//!   - "WALK"                              (client -> start node)
//!   - "WALK HOP <token> <start> <hist>"   (node -> node; single line)
//!   - "WALK DONE <token> <hist>"          (last node -> start)
//!
//! File transfer (line header + exact number of binary bytes):
//!   - "PUSH_FILE <size> <name>"                         (client -> start)
//!   - "FILE HOP <token> <start_addr> <size> <name>"     (node -> node)
//!
//! IMPORTANT: the protocol is line-delimited. Any binary payload *follows*
//! the header line and is exactly <size> bytes long.

/// Parsed representation of a command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // Existing verbs
    SetNext(String),                   // SET_NEXT <addr>
    Get,                               // GET
    Ring { ttl: u32, msg: String },   // RING <ttl> <message...>

    // WALK verbs
    WalkStart,                                             // "WALK"
    WalkHop { token: String, start_addr: String, history: String }, // "WALK HOP ..."
    WalkDone { token: String, history: String },           // "WALK DONE ..."

    // FILE verbs
    /// Header for a client-initiated file push. The binary body of exactly `size`
    /// bytes follows this header on the same connection.
    PushFile { size: u64, name: String },
    /// Header for a node-to-node hop carrying the file body.
    FileHop { token: String, start_addr: String, size: u64, name: String },
}

/// Parse one incoming line from the wire into a Command.
/// Returns an error string if the command is unknown or malformed.
pub fn parse_line(line: &str) -> Result<Command, String> {
    // 1) Trim typical line endings
    let trimmed = line.trim_end_matches(['\r', '\n']);

    // 2) SET_NEXT <addr>
    if let Some(rest) = trimmed.strip_prefix("SET_NEXT ") {
        let addr = rest.trim();
        if addr.is_empty() { return Err("missing address".into()); }
        return Ok(Command::SetNext(addr.to_string()));
    }

    // 3) GET
    if trimmed == "GET" {
        return Ok(Command::Get);
    }

    // 4) RING <ttl> <message...>
    if let Some(rest) = trimmed.strip_prefix("RING ") {
        let mut parts = rest.splitn(2, ' ');
        let ttl_str = parts.next().unwrap_or("");
        let msg = parts.next().unwrap_or("").trim().to_string();
        let ttl = ttl_str.parse::<u32>().map_err(|_| "invalid ttl")?;
        return Ok(Command::Ring { ttl, msg });
    }

    // 5) WALK (client -> start)
    if trimmed == "WALK" {
        return Ok(Command::WalkStart);
    }

    // 6) WALK HOP <token> <start_addr> <history>
    // Use splitn(3, ' ') to preserve spaces inside <history> (even though we use ';')
    if let Some(rest) = trimmed.strip_prefix("WALK HOP ") {
        let mut parts = rest.splitn(3, ' ');
        let token = parts.next().unwrap_or("").trim();
        let start_addr = parts.next().unwrap_or("").trim();
        let history = parts.next().unwrap_or("").to_string();
        if token.is_empty() || start_addr.is_empty() {
            return Err("malformed WALK HOP".into());
        }
        return Ok(Command::WalkHop {
            token: token.to_string(),
            start_addr: start_addr.to_string(),
            history,
        });
    }

    // 7) WALK DONE <token> <history>
    if let Some(rest) = trimmed.strip_prefix("WALK DONE ") {
        let mut parts = rest.splitn(2, ' ');
        let token = parts.next().unwrap_or("").trim();
        let history = parts.next().unwrap_or("").to_string();
        if token.is_empty() {
            return Err("malformed WALK DONE".into());
        }
        return Ok(Command::WalkDone { token: token.to_string(), history });
    }

    // 8) PUSH_FILE <size> <name>
    // Keep <name> as-is (may contain spaces) using splitn(2, ' ')
    if let Some(rest) = trimmed.strip_prefix("PUSH_FILE ") {
        let mut parts = rest.splitn(2, ' ');
        let size_str = parts.next().unwrap_or("").trim();
        let name = parts.next().unwrap_or("").to_string();
        if name.is_empty() { return Err("missing file name".into()); }
        let size = size_str.parse::<u64>().map_err(|_| "invalid size")?;
        return Ok(Command::PushFile { size, name });
    }

    // 9) FILE HOP <token> <start_addr> <size> <name>
    if let Some(rest) = trimmed.strip_prefix("FILE HOP ") {
        let mut parts = rest.splitn(4, ' ');
        let token = parts.next().unwrap_or("").trim();
        let start_addr = parts.next().unwrap_or("").trim();
        let size_str = parts.next().unwrap_or("").trim();
        let name = parts.next().unwrap_or("").to_string();
        if token.is_empty() || start_addr.is_empty() || name.is_empty() {
            return Err("malformed FILE HOP".into());
        }
        let size = size_str.parse::<u64>().map_err(|_| "invalid size")?;
        return Ok(Command::FileHop { token: token.to_string(), start_addr: start_addr.to_string(), size, name });
    }

    // 10) Unknown verb
    Err("unknown command".into())
}
