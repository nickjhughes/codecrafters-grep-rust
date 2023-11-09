use anyhow::Result;
use std::{env, io, ops::Index, process};

#[derive(Debug)]
struct Regex<'regex> {
    patterns: Vec<Pattern<'regex>>,
}

#[derive(Debug)]
enum Pattern<'regex> {
    Character(char),
    Digit,
    Alphanumeric,
    PositiveGroup(&'regex str),
    NegativeGroup(&'regex str),
}

impl<'regex> Pattern<'regex> {
    fn parse(input: &'regex str) -> Result<(&'regex str, Self)> {
        match input.chars().next().unwrap() {
            '[' => {
                // Character group
                let (rest, is_negative) = if input.chars().nth(1) == Some('^') {
                    (input.index(2..), true)
                } else {
                    (input.index(1..), false)
                };

                let mut i = 0;
                let mut chars = rest.chars();
                loop {
                    match chars.next() {
                        Some(ch) => match ch {
                            ']' => {
                                break;
                            }
                            _ => {
                                i += 1;
                            }
                        },
                        None => {
                            anyhow::bail!("premature end of character group")
                        }
                    }
                }

                if is_negative {
                    Ok((
                        rest.index(i + 1..),
                        Pattern::NegativeGroup(rest.index(0..i)),
                    ))
                } else {
                    Ok((
                        rest.index(i + 1..),
                        Pattern::PositiveGroup(rest.index(0..i)),
                    ))
                }
            }
            '\\' => match input.chars().nth(1) {
                Some('d') => {
                    // Digit character class
                    Ok((input.index(2..), Pattern::Digit))
                }
                Some('w') => {
                    // Alphanumeric character class
                    Ok((input.index(2..), Pattern::Alphanumeric))
                }
                Some('\\') => {
                    // Literal backslash
                    Ok((input.index(2..), Pattern::Character('\\')))
                }
                _ => {
                    anyhow::bail!("unhandled pattern")
                }
            },
            ch => {
                // Single character
                Ok((input.index(1..), Pattern::Character(ch)))
            }
        }
    }

    fn matches(&self, ch: char) -> bool {
        match self {
            Pattern::Character(c) => *c == ch,
            Pattern::Digit => ch.is_ascii_digit(),
            Pattern::Alphanumeric => ch.is_ascii_alphanumeric(),
            Pattern::PositiveGroup(chars) => chars.contains(ch),
            Pattern::NegativeGroup(chars) => !chars.contains(ch),
        }
    }
}

impl<'regex> Regex<'regex> {
    fn parse(input: &'regex str) -> Result<Self> {
        // Only handle ascii patterns for simplicity
        if input.chars().any(|ch| !ch.is_ascii()) {
            anyhow::bail!("non-ascii character in pattern {}", input);
        }

        let mut patterns = Vec::new();
        let mut rest = input;
        while !rest.is_empty() {
            let (remainder, pattern) = Pattern::parse(rest)?;
            rest = remainder;
            patterns.push(pattern);
        }
        Ok(Regex { patterns })
    }

    fn matches(&self, input: &str) -> Result<bool> {
        // Only handle ascii inputs for simplicity
        if input.chars().any(|ch| !ch.is_ascii()) {
            anyhow::bail!("non-ascii character in pattern {}", input);
        }

        if self.patterns.is_empty() {
            return Ok(true);
        }

        let mut patterns = self.patterns.iter();
        let mut pattern = patterns.next().unwrap();
        for in_ch in input.chars() {
            if pattern.matches(in_ch) {
                // Move onto next pattern
                pattern = match patterns.next() {
                    Some(pattern) => pattern,
                    None => {
                        // All patterns matched, so overall input matched
                        return Ok(true);
                    }
                };
            }
        }
        // Ran out of input before matching all paterns, so overall input doesn't match
        Ok(false)
    }
}

fn match_pattern(input_line: &str, regex_str: &str) -> Result<bool> {
    let regex = Regex::parse(regex_str)?;
    regex.matches(input_line)
}

// Usage: echo <input_text> | your_grep.sh -E <pattern>
fn main() -> Result<()> {
    if env::args().nth(1).unwrap() != "-E" {
        println!("expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();

    if match_pattern(&input_line, &pattern)? {
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
        assert!(match_pattern("apple", "a").unwrap());
        assert!(!match_pattern("dog", "a").unwrap());
    }

    #[test]
    fn digit_character_class() {
        assert!(match_pattern("3", "\\d").unwrap());
        assert!(!match_pattern("c", "\\d").unwrap());
    }

    #[test]
    fn alphanumeric_character_class() {
        assert!(match_pattern("foo101", "\\w").unwrap());
        assert!(!match_pattern("$!?", "\\w").unwrap());
    }

    #[test]
    fn positive_character_group() {
        assert!(match_pattern("apple", "[abc]").unwrap());
        assert!(!match_pattern("dog", "[abc]").unwrap());
    }

    #[test]
    fn negative_character_group() {
        assert!(match_pattern("dog", "[^abc]").unwrap());
        assert!(!match_pattern("cab", "[^abc]").unwrap());
    }

    #[test]
    fn combined_classes() {
        assert!(match_pattern("1 apple", "\\d apple").unwrap());
        assert!(!match_pattern("1 orange", "\\d apple").unwrap());

        assert!(match_pattern("100 apples", "\\d\\d\\d apple").unwrap());
        assert!(!match_pattern("1 apple", "\\d\\d\\d apple").unwrap());

        assert!(match_pattern("3 dogs", "\\d \\w\\w\\ws").unwrap());
        assert!(match_pattern("4 cats", "\\d \\w\\w\\ws").unwrap());
        assert!(!match_pattern("1 dog", "\\d \\w\\w\\ws").unwrap());

        assert!(!match_pattern("sally has 12 apples", "\\d\\\\d\\\\d apples").unwrap());
    }
}
