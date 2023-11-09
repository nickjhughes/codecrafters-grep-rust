use anyhow::Result;
use std::{env, io, ops::Index, process};

#[derive(Debug, PartialEq)]
struct Regex<'regex> {
    patterns: Vec<Pattern<'regex>>,
}

#[derive(Debug, PartialEq)]
enum Pattern<'regex> {
    Character(char),
    Digit,
    Alphanumeric,
    PositiveGroup(&'regex str),
    NegativeGroup(&'regex str),
    Start,
    End,
    OneOrMore(Box<Pattern<'regex>>),
    ZeroOrOne(Box<Pattern<'regex>>),
}

impl<'regex> Pattern<'regex> {
    fn parse(input: &'regex str) -> Result<(&'regex str, Self)> {
        match input.chars().next().unwrap() {
            '^' => {
                // Start of string anchor
                Ok((input.index(1..), Pattern::Start))
            }
            '$' => {
                // End of string anchor
                Ok((input.index(1..), Pattern::End))
            }
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

                let (rest, inner_pattern) = if is_negative {
                    (
                        rest.index(i + 1..),
                        Pattern::NegativeGroup(rest.index(0..i)),
                    )
                } else {
                    (
                        rest.index(i + 1..),
                        Pattern::PositiveGroup(rest.index(0..i)),
                    )
                };

                if rest.chars().next() == Some('+') {
                    Ok((rest.index(1..), Pattern::OneOrMore(Box::new(inner_pattern))))
                } else if rest.chars().next() == Some('?') {
                    Ok((rest.index(1..), Pattern::ZeroOrOne(Box::new(inner_pattern))))
                } else {
                    Ok((rest, inner_pattern))
                }
            }
            '\\' => match input.chars().nth(1) {
                Some('d') => {
                    // Digit character class
                    if input.chars().nth(2) == Some('+') {
                        Ok((
                            input.index(3..),
                            Pattern::OneOrMore(Box::new(Pattern::Digit)),
                        ))
                    } else if input.chars().nth(2) == Some('?') {
                        Ok((
                            input.index(3..),
                            Pattern::ZeroOrOne(Box::new(Pattern::Digit)),
                        ))
                    } else {
                        Ok((input.index(2..), Pattern::Digit))
                    }
                    // Ok((input.index(2..), Pattern::Digit))
                }
                Some('w') => {
                    // Alphanumeric character class
                    if input.chars().nth(2) == Some('+') {
                        Ok((
                            input.index(3..),
                            Pattern::OneOrMore(Box::new(Pattern::Alphanumeric)),
                        ))
                    } else if input.chars().nth(2) == Some('?') {
                        Ok((
                            input.index(3..),
                            Pattern::ZeroOrOne(Box::new(Pattern::Alphanumeric)),
                        ))
                    } else {
                        Ok((input.index(2..), Pattern::Alphanumeric))
                    }
                }
                Some('\\') => Ok((input.index(2..), Pattern::Character('\\'))),
                Some('$') => Ok((input.index(2..), Pattern::Character('$'))),
                Some('^') => Ok((input.index(2..), Pattern::Character('^'))),
                Some('+') => Ok((input.index(2..), Pattern::Character('+'))),
                Some('?') => Ok((input.index(2..), Pattern::Character('?'))),
                _ => {
                    anyhow::bail!("unhandled pattern")
                }
            },
            ch => {
                // Single character
                if input.chars().nth(1) == Some('+') {
                    Ok((
                        input.index(2..),
                        Pattern::OneOrMore(Box::new(Pattern::Character(ch))),
                    ))
                } else if input.chars().nth(1) == Some('?') {
                    Ok((
                        input.index(2..),
                        Pattern::ZeroOrOne(Box::new(Pattern::Character(ch))),
                    ))
                } else {
                    Ok((input.index(1..), Pattern::Character(ch)))
                }
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
            Pattern::End => false,
            _ => unreachable!(),
        }
    }

    fn must_match(&self) -> bool {
        matches!(self, Pattern::End)
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

        let mut next_must_match = if matches!(pattern, Pattern::Start) {
            pattern = patterns.next().unwrap();
            true
        } else {
            false
        };

        let mut input_chars = input.chars().peekable();
        while let Some(in_ch) = input_chars.next() {
            match pattern {
                Pattern::OneOrMore(inner_pattern) => {
                    if !inner_pattern.matches(in_ch) {
                        if next_must_match {
                            return Ok(false);
                        } else {
                            // Keep going until we find a match
                            loop {
                                if let Some(in_ch) = input_chars.next() {
                                    if inner_pattern.matches(in_ch) {
                                        break;
                                    }
                                } else {
                                    // Ran out of input before finding a match
                                    return Ok(false);
                                }
                            }
                        }
                    }

                    while let Some(in_ch) = input_chars.peek() {
                        if !inner_pattern.matches(*in_ch) {
                            pattern = match patterns.next() {
                                Some(pattern) => pattern,
                                None => {
                                    // All patterns matched, so overall input matched
                                    return Ok(true);
                                }
                            };
                            break;
                        } else {
                            input_chars.next();
                        }
                    }
                }
                _ => {
                    if pattern.matches(in_ch) {
                        // Move onto next pattern
                        pattern = match patterns.next() {
                            Some(pattern) => pattern,
                            None => {
                                // All patterns matched, so overall input matched
                                return Ok(true);
                            }
                        };
                        next_must_match = false;
                    } else if next_must_match || pattern.must_match() {
                        return Ok(false);
                    }
                }
            }
        }
        if matches!(pattern, Pattern::End) {
            Ok(true)
        } else {
            // Ran out of input before matching all paterns, so overall input doesn't match
            Ok(false)
        }
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
    use super::{match_pattern, Pattern, Regex};

    #[test]
    fn parse() {
        let regex = Regex::parse("^[^abc]\\w?f+oo\\d+[bar]+$").unwrap();
        assert_eq!(
            regex,
            Regex {
                patterns: vec![
                    Pattern::Start,
                    Pattern::NegativeGroup("abc"),
                    Pattern::ZeroOrOne(Box::new(Pattern::Alphanumeric)),
                    Pattern::OneOrMore(Box::new(Pattern::Character('f'))),
                    Pattern::Character('o'),
                    Pattern::Character('o'),
                    Pattern::OneOrMore(Box::new(Pattern::Digit)),
                    Pattern::OneOrMore(Box::new(Pattern::PositiveGroup("bar"))),
                    Pattern::End
                ]
            }
        )
    }

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

    #[test]
    fn start_anchor() {
        assert!(match_pattern("log", "^log").unwrap());
        assert!(!match_pattern("slog", "^log").unwrap());
    }

    #[test]
    fn end_anchor() {
        assert!(match_pattern("dog", "dog$").unwrap());
        assert!(!match_pattern("dogs", "dog$").unwrap());
    }

    #[test]
    fn one_or_more() {
        assert!(match_pattern("apple", "a+").unwrap());
        assert!(match_pattern("SaaS", "a+").unwrap());
        assert!(!match_pattern("dog", "a+").unwrap());
    }

    #[test]
    fn zero_or_one() {
        assert!(match_pattern("dogs", "dogs?").unwrap());
        assert!(match_pattern("dog", "dogs?").unwrap());
        assert!(!match_pattern("cat", "dogs?").unwrap());
    }
}
