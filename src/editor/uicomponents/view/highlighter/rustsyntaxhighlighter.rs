use super::{Annotation, AnnotationType, Line, SyntaxHighlighter};
use crate::prelude::*;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

const KEYWORDS: [&str; 57] = [
    "async",
    "await",
    "as",
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "yield",
    "union",
    "macro_rules",
    "include",
    "include_str",
    "option_env",
];

const TYPES: [&str; 22] = [
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char", "Option", "Result", "String", "str", "Vec", "HashMap",
];

const KNOWN_VALUES: [&str; 6] = ["Some", "None", "true", "false", "Ok", "Err"];

#[derive(Default)]
pub struct RustSyntaxHighlighter {
    highlights: HashMap<LineIdx, Vec<Annotation>>,
}

impl SyntaxHighlighter for RustSyntaxHighlighter {
    fn highlight(&mut self, idx: LineIdx, line: &Line) {
        let mut result = Vec::new();
        let mut iterator = line.split_word_bound_indices().peekable();

        while let Some((start_idx, _)) = iterator.next() {
            let remainder = &line[start_idx..];
            if let Some(mut annotation) = annotate_char(remainder)
                .or_else(|| annotate_number(remainder))
                .or_else(|| annotate_keyword(remainder))
                .or_else(|| annotate_type(remainder))
                .or_else(|| annotate_known_value(remainder))
            {
                annotation.shift(start_idx);
                result.push(annotation);

                while let Some(&(next_idx, _)) = iterator.peek() {
                    if next_idx >= annotation.end {
                        break;
                    }
                    iterator.next();
                }
            };
        }
        self.highlights.insert(idx, result);
    }

    fn get_annotations(&self, idx: LineIdx) -> Option<&Vec<Annotation>> {
        self.highlights.get(&idx)
    }
}

fn annotate_next_word<F>(
    string: &str,
    annotation_type: AnnotationType,
    validator: F,
) -> Option<Annotation>
where
    F: Fn(&str) -> bool,
{
    if let Some(word) = string.split_word_bounds().next() {
        if validator(word) {
            return Some(Annotation {
                annotation_type,
                start: 0,
                end: word.len(),
            });
        }
    }
    None
}

fn annotate_number(string: &str) -> Option<Annotation> {
    annotate_next_word(string, AnnotationType::Number, is_valid_number)
}

fn annotate_keyword(string: &str) -> Option<Annotation> {
    annotate_next_word(string, AnnotationType::Keyword, is_keyword)
}

fn annotate_type(string: &str) -> Option<Annotation> {
    annotate_next_word(string, AnnotationType::Type, is_type)
}

fn annotate_known_value(string: &str) -> Option<Annotation> {
    annotate_next_word(string, AnnotationType::KnownValue, is_known_value)
}

fn annotate_char(string: &str) -> Option<Annotation> {
    let mut iter = string.split_word_bound_indices().peekable();
    if let Some((_, "\'")) = iter.next() {
        if let Some((_, "\\")) = iter.peek() {
            iter.next(); //Skip the escape character
        }
        iter.next();
        if let Some((idx, "\'")) = iter.next() {
            return Some(Annotation {
                annotation_type: AnnotationType::Char,
                start: 0,
                end: idx.saturating_add(1), //Include the closing quote in the annotation
            });
        }
    }
    None
}

fn is_valid_number(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }

    if is_numeric_literal(word) {
        return true;
    }

    let mut chars = word.chars();

    // Check the first character
    if let Some(first_char) = chars.next() {
        if !first_char.is_ascii_digit() {
            return false; // Numbers must start with a digit
        }
    }

    let mut seen_dot = false;
    let mut seen_e = false;
    let mut prev_was_digit = true;
    // Iterate over the remaining characters
    for char in chars {
        match char {
            '0'..='9' => {
                prev_was_digit = true;
            }
            '_' => {
                if !prev_was_digit {
                    return false; // Underscores must be between digits
                }
                prev_was_digit = false;
            }
            '.' => {
                if seen_dot || seen_e || !prev_was_digit {
                    return false; // Disallow multiple dots, dots after 'e' or dots not after a digit
                }
                seen_dot = true;
                prev_was_digit = false;
            }
            'e' | 'E' => {
                if seen_e || !prev_was_digit {
                    return false; // Disallow multiple 'e's or 'e' not after a digit
                }
                seen_e = true;
                prev_was_digit = false;
            }
            _ => {
                return false; // Invalid character
            }
        }
    }

    prev_was_digit // Must end with a digit
}

fn is_numeric_literal(word: &str) -> bool {
    if word.len() < 3 {
        //For a literal, we need a leading `0`, a suffix and at least one digit
        return false;
    }
    let mut chars = word.chars();
    if chars.next() != Some('0') {
        // Check the first character for a leading 0
        return false;
    }

    let base = match chars.next() {
        //Check the second character for a proper base
        Some('b' | 'B') => 2,
        Some('o' | 'O') => 8,
        Some('x' | 'X') => 16,
        _ => return false,
    };
    chars.all(|char| char.is_digit(base))
}

fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

fn is_type(word: &str) -> bool {
    TYPES.contains(&word)
}

fn is_known_value(word: &str) -> bool {
    KNOWN_VALUES.contains(&word)
}
