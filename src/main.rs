use anyhow::Result;
use std::{env, io, ops::Index, process};

#[derive(Debug, PartialEq)]
struct Regex<'regex> {
    patterns: Vec<Pattern<'regex>>,
}

#[derive(Debug, PartialEq, Clone)]
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
    Wildcard,
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
                Some('.') => Ok((input.index(2..), Pattern::Character('.'))),
                _ => {
                    anyhow::bail!("unhandled pattern")
                }
            },
            '.' => {
                // Wildcard
                Ok((input.index(1..), Pattern::Wildcard))
            }
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
            Pattern::Wildcard => true,
            _ => unreachable!(),
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

        // dbg!(&self);

        Ok(self.match_(input, 0))
    }

    fn match_(&self, input: &str, pattern_index: usize) -> bool {
        // eprintln!("match_({:?}, {})", input, pattern_index);

        if self.patterns.get(pattern_index) == Some(&Pattern::Start) {
            return self.match_here(input, pattern_index + 1);
        }

        let mut input = input;
        loop {
            if self.match_here(input, pattern_index) {
                return true;
            }
            input = &input[1..];
            if input.is_empty() {
                break;
            }
        }
        false
    }

    fn match_here(&self, input: &str, pattern_index: usize) -> bool {
        // eprintln!("match_here({:?}, {})", input, pattern_index);

        match self.patterns.get(pattern_index) {
            None => true,
            Some(pattern) => match pattern {
                Pattern::OneOrMore(inner_pattern) => {
                    self.match_one_or_more(input, inner_pattern, pattern_index + 1)
                }
                Pattern::ZeroOrOne(inner_pattern) => {
                    self.match_zero_or_one(input, inner_pattern, pattern_index + 1)
                }
                Pattern::End if self.patterns.get(pattern_index + 1).is_none() => input.is_empty(),
                Pattern::Character(ch) if input.chars().next() == Some(*ch) => {
                    self.match_here(&input[1..], pattern_index + 1)
                }
                pattern => {
                    if let Some(ch) = input.chars().next() {
                        if pattern.matches(ch) {
                            self.match_here(&input[1..], pattern_index + 1)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            },
        }
    }

    fn match_one_or_more(
        &self,
        input: &str,
        inner_pattern: &Pattern,
        next_pattern_index: usize,
    ) -> bool {
        // eprintln!(
        //     "match_one_or_more({:?}, {:?}, {})",
        //     input, &inner_pattern, next_pattern_index
        // );

        let mut input = input;
        while !input.is_empty() && inner_pattern.matches(input.chars().next().unwrap()) {
            input = &input[1..];
            if self.match_here(input, next_pattern_index) {
                return true;
            }
        }
        false
    }

    fn match_zero_or_one(
        &self,
        input: &str,
        inner_pattern: &Pattern,
        next_pattern_index: usize,
    ) -> bool {
        // eprintln!(
        //     "match_zero_or_one({:?}, {:?}, {})",
        //     input, &inner_pattern, next_pattern_index
        // );

        if self.match_here(input, next_pattern_index) {
            return true;
        }
        if !input.is_empty() && inner_pattern.matches(input.chars().next().unwrap()) {
            self.match_here(&input[1..], next_pattern_index)
        } else {
            false
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
        assert!(!match_pattern("cag", "ca?t").unwrap());
    }

    #[test]
    fn wildcard() {
        assert!(match_pattern("dog", "d.g").unwrap());
        assert!(!match_pattern("cog", "d.g").unwrap());
    }
}
