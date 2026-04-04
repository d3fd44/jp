//! A zero-dependency recursive JSON parser.
//!
//! handle deserialization with minimal overhead and
//! strict UTF-8 compliance.

use colored::Colorize;
use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

/// Represents (almost) all possible failure modes during the lexical scanning and parsing phases.
#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnexpectedCharacter(char),
    UnexpectedEndOfFile,
    InvalidInput,
}

/// The Abstract Syntax Tree (AST) representation of a parsed JSON document.
///
/// This enum recursively maps to the standard JSON data types.
/// works with String buffers, not borrowed (`&str`).
#[derive(Debug, PartialEq)]
pub enum Json {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(HashMap<String, Json>),
}

/// A formatting wrapper that maintains indentation state.
///
/// traits: `std::fmt::Display`
pub struct PrettyJson<'a> {
    json: &'a Json,
    indent: usize,
}

impl Json {
    pub fn pretty(&self) -> PrettyJson<'_> {
        PrettyJson {
            json: self,
            indent: 0,
        }
    }
}

impl std::fmt::Display for PrettyJson<'_> {
    /// Creates a temporary formatting wrapper around the AST for pretty-printing.
    ///
    /// # Examples
    /// ```rust
    /// let ast = parse_value(&mut chars)?;
    /// println!("{}", ast.pretty());
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let spaces = "  ".repeat(self.indent);
        let next_spaces = "  ".repeat(self.indent + 1);

        match self.json {
            Json::Null => write!(f, "{}", "null".purple()),
            Json::Boolean(b) => write!(f, "{}", if *b { "true".blue() } else { "false".red() }),
            Json::Number(n) => write!(f, "{}", n.to_string().green()),
            Json::String(s) => write!(f, "{}{}{}", "\"".yellow(), s, "\"".yellow()),

            Json::Array(arr) => {
                if arr.is_empty() {
                    return write!(f, "{}", "[]".cyan());
                }

                writeln!(f, "{}", "[".cyan())?;

                let len = arr.len();
                for (i, item) in arr.iter().enumerate() {
                    let child_wrapper = PrettyJson {
                        json: item,
                        indent: self.indent + 1,
                    };

                    write!(f, "{}{}", next_spaces, child_wrapper)?;

                    if i < len - 1 {
                        writeln!(f, "{}", ",".cyan())?;
                    } else {
                        writeln!(f)?;
                    }
                }
                write!(f, "{}{}", spaces, "]".cyan())
            }

            Json::Object(object) => {
                if object.is_empty() {
                    return write!(f, "{}", "{}".magenta());
                }

                writeln!(f, "{}", "{".magenta())?;

                let len = object.len();
                for (i, (key, value)) in object.iter().enumerate() {
                    let child_wrapper = PrettyJson {
                        json: value,
                        indent: self.indent + 1,
                    };

                    write!(
                        f,
                        "{}{}{}{}: {}",
                        next_spaces,
                        "\"".yellow(),
                        key,
                        "\"".yellow(),
                        child_wrapper
                    )?;

                    if i < len - 1 {
                        writeln!(f, "{}", ",".magenta())?;
                    } else {
                        writeln!(f)?;
                    }
                }
                write!(f, "{}{}", spaces, "}".magenta())
            }
        }
    }
}

/// Reads a JSON file from the filesystem directly into a UTF-8 `String`.
///
/// # Errors
/// Returns a `std::io::Error` if the file does not exist, cannot be read,
/// or contains invalid UTF-8 data.
pub fn read_file(path: &str) -> Result<String, std::io::Error> {
    let jf = std::fs::read_to_string(path)?;
    Ok(jf)
}

/// Advances the iterator past standard JSON whitespace.
fn skip_whitespace(chars: &mut Peekable<Chars>) {
    while let Some(&c) = chars.peek()
        && c.is_whitespace()
    {
        chars.next();
    }
}

/// The primary recursive entry point for the parser.
///
/// Automatically strips leading whitespace, evaluates the next character, etc.
///
/// # Errors
/// Returns a `ParseError` if the sequence is truncated, or contains Invalid/Unexpected characters.
pub fn parse_value(chars: &mut Peekable<Chars>) -> Result<Json, ParseError> {
    skip_whitespace(chars);
    match chars.peek() {
        Some(&'{') => parse_object(chars),
        Some(&'[') => parse_array(chars),
        Some(&'"') => parse_string(chars),
        Some(&'t') | Some(&'f') => parse_bool(chars),
        Some(&c) if c.is_ascii_digit() || c == '-' => parse_number(chars),
        Some(&'n') => parse_null(chars),
        None => Err(ParseError::UnexpectedEndOfFile),
        _ => Err(ParseError::InvalidInput),
    }
}

/// Helper function to strictly verify and consume a sequence of exact character matches.
fn match_next(chars: &mut Peekable<Chars>, expected: &str) -> Result<(), ParseError> {
    for c in expected.chars() {
        match chars.next() {
            Some(t) if t == c => continue,
            None => return Err(ParseError::UnexpectedEndOfFile),
            Some(x) => return Err(ParseError::UnexpectedCharacter(x)),
        }
    }
    Ok(())
}

/// Parses a literal `null` and returns the corresponding variant.
fn parse_null(chars: &mut Peekable<Chars>) -> Result<Json, ParseError> {
    match_next(chars, "null")?;
    Ok(Json::Null)
}

/// Parses a literal `true` or `false` and returns the corresponding Variant.
fn parse_bool(chars: &mut Peekable<Chars>) -> Result<Json, ParseError> {
    match chars.peek() {
        Some(&'t') => {
            match_next(chars, "true")?;
            Ok(Json::Boolean(true))
        }
        Some(&'f') => {
            match_next(chars, "false")?;
            Ok(Json::Boolean(false))
        }
        _ => Err(ParseError::InvalidInput),
    }
}

/// Scans a contiguous block of numeric characters and parses them into a standard 64-bit floating-point value.
fn parse_number(chars: &mut Peekable<Chars>) -> Result<Json, ParseError> {
    let mut buf = String::new();
    while let Some(&c) = chars.peek() {
        match c {
            '+' | '-' | '.' | 'e' | 'E' | '0'..='9' => {
                buf.push(c);
                chars.next();
            }
            _ => break,
        }
    }

    if buf.is_empty() {
        return Err(ParseError::UnexpectedEndOfFile);
    }

    match buf.parse::<f64>() {
        Ok(number) => Ok(Json::Number(number)),
        Err(_) => Err(ParseError::InvalidInput),
    }
}

/// Parses a strictly formatted UTF-8 string, starting with '"'.
fn parse_string(chars: &mut Peekable<Chars>) -> Result<Json, ParseError> {
    chars.next();

    let mut buf = String::new();
    while let Some(&c) = chars.peek() {
        match c {
            '"' => {
                chars.next();
                return Ok(Json::String(buf));
            }
            '\\' => {
                chars.next();
                match chars.next() {
                    Some('n') => buf.push('\n'),
                    Some('t') => buf.push('\t'),
                    Some('r') => buf.push('\r'),
                    Some('"') => buf.push('\"'),
                    Some('\\') => buf.push('\\'),
                    Some(x) => return Err(ParseError::UnexpectedCharacter(x)),
                    None => return Err(ParseError::UnexpectedEndOfFile),
                }
            }
            _ => {
                buf.push(c);
                chars.next();
            }
        }
    }
    Err(ParseError::UnexpectedEndOfFile)
}

/// Recursively evaluates a comma-separated list of valid JSON values.
fn parse_array(chars: &mut Peekable<Chars>) -> Result<Json, ParseError> {
    let mut elements: Vec<Json> = Vec::new();
    chars.next();
    loop {
        skip_whitespace(chars);
        if let Some(']') = chars.peek() {
            chars.next();
            return Ok(Json::Array(elements));
        }

        let value = parse_value(chars)?;
        elements.push(value);

        skip_whitespace(chars);
        match chars.next() {
            Some(',') => {
                skip_whitespace(chars);
                if let Some(']') = chars.peek() {
                    return Err(ParseError::UnexpectedCharacter(','));
                }
                continue;
            }
            Some(']') => return Ok(Json::Array(elements)),
            Some(x) => return Err(ParseError::UnexpectedCharacter(x)),
            None => return Err(ParseError::UnexpectedEndOfFile),
        }
    }
}

/// Recursively evaluates a comma-separated list of string-keyed JSON values.
fn parse_object(chars: &mut Peekable<Chars>) -> Result<Json, ParseError> {
    chars.next();

    let mut object: HashMap<String, Json> = HashMap::new();

    loop {
        skip_whitespace(chars);

        if let Some('}') = chars.peek() {
            chars.next();
            return Ok(Json::Object(object));
        }

        let key = if let Json::String(k) = parse_string(chars)? {
            k
        } else {
            return Err(ParseError::InvalidInput);
        };

        skip_whitespace(chars);
        match chars.next() {
            Some(':') => {}
            Some(x) => return Err(ParseError::UnexpectedCharacter(x)),
            None => return Err(ParseError::UnexpectedEndOfFile),
        };

        skip_whitespace(chars);
        let value = parse_value(chars)?;
        object.insert(key, value);

        skip_whitespace(chars);
        match chars.next() {
            Some(',') => {
                skip_whitespace(chars);
                if let Some('}') = chars.peek() {
                    return Err(ParseError::UnexpectedCharacter(','));
                }
                continue;
            }
            Some('}') => return Ok(Json::Object(object)),
            Some(x) => return Err(ParseError::UnexpectedCharacter(x)),
            None => return Err(ParseError::UnexpectedEndOfFile),
        }
    }
}

#[cfg(test)]
mod parsing {
    use super::*;

    fn get_iter(input: &str) -> Peekable<Chars<'_>> {
        input.chars().peekable()
    }

    #[test]
    fn test_null() {
        let mut valid = get_iter("null");
        let mut invalid = get_iter("nulk");

        assert_eq!(parse_null(&mut valid).unwrap(), Json::Null);
        assert_eq!(
            parse_null(&mut invalid),
            Err(ParseError::UnexpectedCharacter('k'))
        );
    }

    #[test]
    fn test_bool() {
        let mut valid_true = get_iter("true");
        let mut valid_false = get_iter("false");
        let mut invalid_true = get_iter("True");
        let mut invalid_true2 = get_iter("txue");
        let mut invalid = get_iter("");

        assert_eq!(parse_bool(&mut valid_true).unwrap(), Json::Boolean(true));
        assert_eq!(parse_bool(&mut valid_false).unwrap(), Json::Boolean(false));
        assert_eq!(parse_bool(&mut invalid_true), Err(ParseError::InvalidInput));
        assert_eq!(
            parse_bool(&mut invalid_true2),
            Err(ParseError::UnexpectedCharacter('x'))
        );
        assert_eq!(parse_bool(&mut invalid), Err(ParseError::InvalidInput));
    }

    #[test]
    fn test_number() {
        let mut valid1 = get_iter("12312735");
        let mut valid2 = get_iter("1231.2735");
        let mut valid3 = get_iter("-11.2");
        let mut valid4 = get_iter("-11e2");
        let mut valid5 = get_iter("-9.1e62");
        let mut invalid = get_iter("-9.1e6,2");

        assert_eq!(parse_number(&mut valid1).unwrap(), Json::Number(12312735.0));
        assert_eq!(parse_number(&mut valid2).unwrap(), Json::Number(1231.2735));
        assert_eq!(parse_number(&mut valid3).unwrap(), Json::Number(-11.2));
        assert_eq!(parse_number(&mut valid4).unwrap(), Json::Number(-11e2));
        assert_eq!(parse_number(&mut valid5).unwrap(), Json::Number(-9.1e62));
        assert_eq!(parse_number(&mut invalid).unwrap(), Json::Number(-9.1e6));
        assert_eq!(invalid.next(), Some(','));
    }

    #[test]
    fn test_string() {
        let mut valid1 = get_iter("\"momen\"");
        let mut valid2 = get_iter("\"Momen\\nDefdaa\\t(\\\"was here XD\\\")\"");
        let mut valid3 = get_iter("\"\"");
        let mut invalid = get_iter("\"\\invalid\"");

        assert_eq!(
            parse_string(&mut valid1).unwrap(),
            Json::String("momen".to_string())
        );
        assert_eq!(
            parse_string(&mut valid2).unwrap(),
            Json::String("Momen\nDefdaa\t(\"was here XD\")".to_string())
        );
        assert_eq!(
            parse_string(&mut valid3).unwrap(),
            Json::String("".to_string())
        );
        assert_eq!(
            parse_string(&mut invalid),
            Err(ParseError::UnexpectedCharacter('i'))
        );
    }

    #[test]
    fn test_array() {
        let mut valid1 = get_iter("[]");
        let mut valid2 = get_iter("[ \"momen\", 24, [\"games\", \"programming\", 8.5]]");
        let mut invalid = get_iter("[1, 2, 3, ]");

        assert_eq!(parse_array(&mut valid1).unwrap(), Json::Array(vec![]));
        assert_eq!(
            parse_array(&mut valid2).unwrap(),
            Json::Array(vec![
                Json::String("momen".to_string()),
                Json::Number(24.0),
                Json::Array(vec![
                    Json::String("games".to_string()),
                    Json::String("programming".to_string()),
                    Json::Number(8.5)
                ])
            ])
        );
        assert_eq!(
            parse_array(&mut invalid),
            Err(ParseError::UnexpectedCharacter(','))
        );
    }

    #[test]
    fn test_object() {
        let mut valid1 = get_iter(
            "{\"name\" : [  \"Mohammad\", \"Momen\"   , \"Defdaa\"], \"skills\": { \"gaming\"      :10, \"programming\": 6, \"handsome\": 7.5 }}",
        );
        let mut valid2 = get_iter("{}");
        let mut invalid = get_iter("{\"name\": \"momen\", }");

        let mut object1 = HashMap::<String, Json>::new();
        let mut object1p2 = HashMap::<String, Json>::new();
        let mut _object2 = HashMap::<String, Json>::new();

        object1.insert(
            "name".to_string(),
            Json::Array(vec![
                Json::String("Mohammad".to_string()),
                Json::String("Momen".to_string()),
                Json::String("Defdaa".to_string()),
            ]),
        );
        object1p2.insert("gaming".to_string(), Json::Number(10.0));
        object1p2.insert("programming".to_string(), Json::Number(6.0));
        object1p2.insert("handsome".to_string(), Json::Number(7.5));
        object1.insert("skills".to_string(), Json::Object(object1p2));

        assert_eq!(parse_object(&mut valid1).unwrap(), Json::Object(object1));
        assert_eq!(parse_object(&mut valid2).unwrap(), Json::Object(_object2));
        assert_eq!(
            parse_object(&mut invalid),
            Err(ParseError::UnexpectedCharacter(','))
        );
    }
}

// JSON PARSER //
