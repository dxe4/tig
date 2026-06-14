#[derive(Debug, Clone, PartialEq)]
pub enum CompareTarget {
    Commit(String),
    Range { left: String, right: String },
}

pub fn parse_arg<I>(args: I) -> Option<CompareTarget>
where
    I: IntoIterator<Item = String>,
{
    args.into_iter()
        .find(|a| a != "--")
        .map(|a| {
            if let Some((left, right)) = a.split_once(':') {
                CompareTarget::Range {
                    left: left.to_string(),
                    right: right.to_string(),
                }
            } else {
                CompareTarget::Commit(a)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_no_arg() {
        assert_eq!(parse_arg(Vec::new()), None);
    }

    #[test]
    fn parse_commit_ref() {
        assert_eq!(
            parse_arg(vec!["abc123".to_string()]),
            Some(CompareTarget::Commit("abc123".to_string()))
        );
    }

    #[test]
    fn parse_range() {
        assert_eq!(
            parse_arg(vec!["main:branch".to_string()]),
            Some(CompareTarget::Range {
                left: "main".to_string(),
                right: "branch".to_string(),
            })
        );
    }

    #[test]
    fn parse_range_with_slashes() {
        assert_eq!(
            parse_arg(vec!["HEAD~3:feature/x".to_string()]),
            Some(CompareTarget::Range {
                left: "HEAD~3".to_string(),
                right: "feature/x".to_string(),
            })
        );
    }

    #[test]
    fn parse_skips_double_dash() {
        assert_eq!(
            parse_arg(vec!["--".to_string(), "main:branch".to_string()]),
            Some(CompareTarget::Range {
                left: "main".to_string(),
                right: "branch".to_string(),
            })
        );
    }
}
