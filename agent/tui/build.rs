use std::{env, fs, path::PathBuf};

fn parse_hex(block: &str, name: &str) -> (u8, u8, u8) {
    let marker = format!("--{name}:");
    let value = block
        .split(&marker)
        .nth(1)
        .and_then(|rest| rest.split(';').next())
        .map(str::trim)
        .filter(|value| value.len() == 7 && value.starts_with('#'))
        .unwrap_or_else(|| panic!("missing or invalid {marker} in design-tokens.css"));
    let byte = |range| u8::from_str_radix(&value[range], 16).expect("validated hex token");
    (byte(1..3), byte(3..5), byte(5..7))
}

fn emit(out: &mut String, rust_name: &str, block: &str, css_name: &str) {
    let (r, g, b) = parse_hex(block, css_name);
    out.push_str(&format!(
        "pub const {rust_name}: ratatui::style::Color = ratatui::style::Color::Rgb({r}, {g}, {b});\n"
    ));
}

fn main() {
    let token_path =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../design-tokens.css");
    println!("cargo:rerun-if-changed={}", token_path.display());
    let css = fs::read_to_string(&token_path).expect("read canonical design-tokens.css");
    let dark_start = css.find("[data-theme=\"dark\"]").expect("dark token block");
    let light = &css[..dark_start];
    let dark = &css[dark_start..];
    let mut generated = String::from("// Generated from root design-tokens.css.\n");
    for (rust_name, css_name) in [
        ("COLOR_BRAND", "ag-brand"),
        ("COLOR_ALLOW", "ag-allow"),
        ("COLOR_ASK", "ag-ask"),
        ("COLOR_DENY", "ag-deny"),
        ("COLOR_MUTED", "ag-text-muted"),
        ("COLOR_BORDER", "ag-border"),
        ("COLOR_TEXT", "ag-text"),
        ("COLOR_BG", "ag-bg"),
    ] {
        emit(&mut generated, rust_name, light, css_name);
        emit(&mut generated, &format!("{rust_name}_DARK"), dark, css_name);
    }
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("design_tokens.rs"),
        generated,
    )
    .expect("write generated design tokens");
}
