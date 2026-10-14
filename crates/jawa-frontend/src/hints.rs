//! Hints for JavaScript and TypeScript habits.
//!
//! Jawa's syntax follows JavaScript and TypeScript, but its words are
//! Javanese. When a name is not found, or a statement starts with a
//! JavaScript keyword, these hints say what Jawa calls it.

/// The Jawa spelling of a JavaScript or TypeScript keyword, type, value or
/// common function.
pub fn jawa_for(word: &str) -> Option<&'static str> {
    Some(match word {
        "let" | "var" => "ayo",
        "const" => "tetep",
        "function" => "gawe",
        "return" => "bali",
        "if" => "yen",
        "else" => "liyane",
        "while" => "sajrone",
        "for" => "kanggo",
        "of" | "in" => "ing",
        "break" => "mandheg",
        "continue" => "terusna",
        "true" => "bener",
        "false" => "salah",
        "null" | "undefined" => "kosong",
        "class" | "interface" | "struct" | "type" => "wujud",
        "number" => "wilangan` or `pecahan",
        "string" => "ukara",
        "boolean" => "logika",
        "void" => "suwung",
        "print" | "println" | "log" => "tulis",
        "len" | "length" => "dawane",
        "push" => "tambah",
        "throw" | "panic" => "gagal",
        _ => return None,
    })
}

/// A note saying how Jawa spells `word`, if it is a JavaScript habit.
pub fn note_for(word: &str) -> Option<String> {
    jawa_for(word).map(|jawa| format!("Jawa writes `{word}` as `{jawa}`"))
}

/// JavaScript keywords that start a statement.
pub fn starts_javascript_statement(word: &str) -> bool {
    matches!(
        word,
        "let"
            | "var"
            | "const"
            | "function"
            | "return"
            | "if"
            | "else"
            | "while"
            | "for"
            | "break"
            | "continue"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes() {
        assert_eq!(
            note_for("false").as_deref(),
            Some("Jawa writes `false` as `salah`")
        );
        assert_eq!(
            note_for("number").as_deref(),
            Some("Jawa writes `number` as `wilangan` or `pecahan`")
        );
        assert_eq!(note_for("jeneng"), None);
    }
}
