use regex::Regex;

pub fn glob_to_regex(pattern: &str) -> Result<Regex, regex::Error> {
    let mut regex_str = String::new();
    regex_str.push('^');
    for c in pattern.chars() {
        match c {
            '*' => regex_str.push_str(".*"),
            '?' => regex_str.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex_str.push('\\');
                regex_str.push(c);
            }
            _ => regex_str.push(c),
        }
    }
    regex_str.push('$');
    Regex::new(&regex_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_star_matches_any() {
        let re = glob_to_regex("*.py").unwrap();
        assert!(re.is_match("foo.py"));
        assert!(!re.is_match("foo.rs"));
    }

    #[test]
    fn glob_question_matches_one() {
        let re = glob_to_regex("t?st").unwrap();
        assert!(re.is_match("test"));
        assert!(re.is_match("tast"));
        assert!(!re.is_match("tst"));
    }

    #[test]
    fn glob_escapes_regex_metacharacters() {
        let re = glob_to_regex("a+b").unwrap();
        assert!(re.is_match("a+b"));
        assert!(!re.is_match("aab"));
    }
}
