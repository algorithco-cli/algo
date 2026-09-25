use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCommand {
    pub bin: String,
    pub args: Vec<String>,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCmd {
    pub commands: Vec<ParsedCommand>,
    pub redirects: Vec<String>,
    pub pipes: usize,
    pub subshells: usize,
    pub env_assigns: Vec<String>,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseFail: {}", self.0)
    }
}
impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<ParsedCmd, ParseError> {
    // Guard: tree-sitter's C parser can SEGV on null bytes or lone surrogates
    // (fuzz crash bd621...: "ppp\x00..."). Fail-safe → Err (caller maps to ASK).
    if input.as_bytes().contains(&0) {
        return Err(ParseError("input contains null".into()));
    }
    // Empty input is not a command — Err so the caller maps to ASK.
    if input.trim().is_empty() {
        return Err(ParseError("empty input".into()));
    }
    let mut parser = Parser::new();
    let lang: tree_sitter::Language = tree_sitter_bash::LANGUAGE.into();
    parser
        .set_language(&lang)
        .map_err(|e| ParseError(format!("lang: {:?}", e)))?;
    let tree = parser
        .parse(input, None)
        .ok_or_else(|| ParseError("no tree".into()))?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(ParseError("tree has error".into()));
    }

    let mut commands = Vec::new();
    let mut redirects = Vec::new();
    let mut pipes = 0;
    let mut subshells = 0;
    let mut env_assigns = Vec::new();

    // Walk the tree and collect command nodes
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "command" => {
                if let Some(cmd) = extract_command(&node, input) {
                    commands.push(cmd);
                }
            }
            "redirected_statement" => {
                // Count redirects
                redirects.push(node.utf8_text(input.as_bytes()).unwrap_or("").to_string());
            }
            "pipeline" => {
                pipes += 1;
            }
            "subshell" | "command_substitution" => {
                subshells += 1;
            }
            "variable_assignment" => {
                env_assigns.push(node.utf8_text(input.as_bytes()).unwrap_or("").to_string());
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    // The DFS stack above visits siblings in reverse — restore source order
    // so tree-order checks (pipe-to-shell: net before shell) are correct.
    commands.reverse();

    // Fallback: if no commands found but input is not empty, try to extract via simple split
    // (for cases where tree-sitter didn't produce a command node due to incomplete parse)
    if commands.is_empty() && !input.trim().is_empty() {
        if let Some(cmd) = extract_command_fallback(input) {
            commands.push(cmd);
        } else {
            return Err(ParseError("no command found".into()));
        }
    }

    Ok(ParsedCmd {
        commands,
        redirects,
        pipes,
        subshells,
        env_assigns,
        raw: input.to_string(),
    })
}

fn extract_command(node: &Node, input: &str) -> Option<ParsedCommand> {
    // Use the node's text and split — more robust than kind-specific extraction
    let text = node
        .utf8_text(input.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
    if text.is_empty() {
        return None;
    }
    // For a command like "ls -la", split into parts
    // But keep quoted strings together — simple split on whitespace for now
    // (tree-sitter already handled quoting, but we use the raw text for simplicity)
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let bin = parts[0].to_string();
    let args = parts[1..].iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let flags = args
        .iter()
        .filter(|a| a.starts_with('-'))
        .cloned()
        .collect();
    Some(ParsedCommand { bin, args, flags })
}

fn extract_command_fallback(input: &str) -> Option<ParsedCommand> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let bin = parts[0].to_string();
    let args = parts[1..].iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let flags = args
        .iter()
        .filter(|a| a.starts_with('-'))
        .cloned()
        .collect();
    Some(ParsedCommand { bin, args, flags })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_ls() {
        let p = parse("ls -la").unwrap();
        assert_eq!(p.commands[0].bin, "ls");
        assert!(p.commands[0].flags.contains(&"-la".to_string()));
    }

    #[test]
    fn proves_ask_on_parse_fail() {
        // Fail-safe: unparseable input is Err (caller maps to ASK, never ALLOW).
        // Empty / whitespace-only can never be a command.
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());
        // Broken redirections are a tree error → Err, not fallback-Ok.
        assert!(parse("<<<>>>").is_err());
        // Null bytes would SEGV the C parser — guarded to Err.
        assert!(parse("ls\x00 -la").is_err());
    }

    #[test]
    fn counts_pipes_and_subshells() {
        let p = parse("ls | grep foo").unwrap();
        assert!(p.pipes >= 1);
        let p2 = parse("echo $(ls)").unwrap();
        assert!(p2.subshells >= 1);
    }
}
