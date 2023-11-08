use std::{env, io, process};

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    if pattern.chars().count() == 1 {
        // Single character
        return input_line.contains(pattern);
    } else if pattern == "\\d" {
        // Digit character class
        input_line.chars().any(|ch| ch.is_ascii_digit())
    } else if pattern == "\\w" {
        // Alphanumeric character class
        input_line.chars().any(|ch| ch.is_ascii_alphanumeric())
    } else {
        panic!("unhandled pattern: {}", pattern)
    }
}

// Usage: echo <input_text> | your_grep.sh -E <pattern>
fn main() {
    if env::args().nth(1).unwrap() != "-E" {
        println!("expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();

    if match_pattern(&input_line, &pattern) {
        process::exit(0)
    } else {
        process::exit(1)
    }
}

#[cfg(test)]
mod tests {
    use super::match_pattern;

    #[test]
    fn single_character() {
        assert!(match_pattern("apple", "a"));
        assert!(!match_pattern("foo", "a"));
    }

    #[test]
    fn digit_character_class() {
        assert!(match_pattern("apple123", "\\d"));
        assert!(!match_pattern("foo", "\\d"));
    }

    #[test]
    fn alphanumeric_character_class() {
        assert!(match_pattern("foo101", "\\w"));
        assert!(!match_pattern("$!?", "\\w"));
    }
}
