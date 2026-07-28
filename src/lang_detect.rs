#[derive(Debug, PartialEq)]
pub enum Language {
    Python,
    Node,
    Unknown,
}

pub fn detect(cmd: &[String]) -> Language {
    if cmd.is_empty() {
        return Language::Unknown;
    }
    let prog = cmd[0].to_lowercase();
    if prog.contains("python") {
        return Language::Python;
    }
    if prog.contains("node") {
        return Language::Node;
    }
    Language::Unknown
}