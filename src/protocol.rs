/// Line-based protocol commands sent over TCP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    SetNext(String),         // SET_NEXT <addr>
    Get,                     // GET
    Ring { hops: u32, msg: String }, // RING <hops> <message...>
}

pub fn parse_line(line: &str) -> Result<Command, String> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if let Some(rest) = trimmed.strip_prefix("SET_NEXT ") {
        let addr = rest.trim();
        if addr.is_empty() { return Err("missing address".into()); }
        return Ok(Command::SetNext(addr.to_string()));
    }
    if trimmed == "GET" {
        return Ok(Command::Get);
    }
    if let Some(rest) = trimmed.strip_prefix("RING ") {
        let mut parts = rest.splitn(2, ' ');
        let hops_str = parts.next().unwrap_or("");
        let msg = parts.next().unwrap_or("").trim().to_string();
        let hops = hops_str.parse::<u32>().map_err(|_| "invalid hops")?;
        return Ok(Command::Ring { hops, msg });
    }
    Err("unknown command".into())
}
