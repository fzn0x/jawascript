//! Diagnostics: errors and warnings with a source span, rendered with the
//! offending line and a caret underline.

use crate::source::{SourceFile, Span};
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            span,
            notes: Vec::new(),
        }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            message: message.into(),
            span,
            notes: Vec::new(),
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Renders the diagnostic in the familiar compiler layout:
    ///
    /// ```text
    /// error: unknown variable `y`
    ///  --> main.jw:3:10
    ///   |
    /// 3 |     tulis(y)
    ///   |           ^
    /// ```
    pub fn render(&self, file: &SourceFile, color: bool) -> String {
        let paint = |code: &str, text: &str| {
            if color {
                format!("\x1b[{code}m{text}\x1b[0m")
            } else {
                text.to_string()
            }
        };
        let (label, label_color) = match self.severity {
            Severity::Error => ("error", "1;31"),
            Severity::Warning => ("warning", "1;33"),
        };

        let start = file.line_col(self.span.start);
        let line_text = file.line_text(start.line);
        let gutter = start.line.to_string().len();
        let pad = " ".repeat(gutter);

        // Underline at least one column, and never past the end of the line.
        let line_chars = line_text.chars().count() as u32;
        let end = file.line_col(self.span.end);
        let width = if end.line == start.line {
            end.col.saturating_sub(start.col).max(1)
        } else {
            line_chars.saturating_sub(start.col - 1).max(1)
        };
        let lead: String = line_text
            .chars()
            .take(start.col as usize - 1)
            .map(|c| if c == '\t' { '\t' } else { ' ' })
            .collect();

        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}{}",
            paint(label_color, label),
            paint("1", &format!(": {}", self.message))
        );
        let _ = writeln!(
            out,
            "{pad}{} {}:{}",
            paint("1;34", "-->"),
            file.name(),
            start
        );
        let _ = writeln!(out, "{pad} {}", paint("1;34", "|"));
        let _ = writeln!(
            out,
            "{} {} {line_text}",
            paint("1;34", &start.line.to_string()),
            paint("1;34", "|")
        );
        let _ = writeln!(
            out,
            "{pad} {} {lead}{}",
            paint("1;34", "|"),
            paint(label_color, &"^".repeat(width as usize))
        );
        for note in &self.notes {
            let _ = writeln!(out, "{pad} {} note: {note}", paint("1;34", "="));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_line_and_caret() {
        let file = SourceFile::new("main.jw", "gawe utama() {\n    tulis(y)\n}\n");
        let diag = Diagnostic::error(Span::new(25, 26), "unknown variable `y`");
        let text = diag.render(&file, false);
        assert_eq!(
            text,
            "error: unknown variable `y`\n \
             --> main.jw:2:11\n  \
             |\n\
             2 |     tulis(y)\n  \
             |           ^\n"
        );
    }

    #[test]
    fn notes_follow_the_snippet() {
        let file = SourceFile::new("main.jw", "bali 1\n");
        let diag = Diagnostic::error(Span::new(0, 4), "`bali` outside a function")
            .with_note("move it into a `gawe` body");
        let text = diag.render(&file, false);
        assert!(text.ends_with("  = note: move it into a `gawe` body\n"));
        assert!(text.contains("| ^^^^\n"));
    }
}
