#[cfg(feature = "logging")]
use alloc::string::{String, ToString};

#[derive(Debug, PartialEq, Clone)]
pub enum Tree {
    Scalar(Vec<u8>),
    Sequence(Vec<Tree>),
    Mapping(Vec<(Vec<u8>, Tree)>),
    Null,
}

#[cfg(feature = "logging")]
pub fn message(class: &str, fn_name: &str, args: &[&str]) -> String {
    let mut s = String::from(class);
    s.push_str("::");
    s.push_str(fn_name);
    s.push('(');
    for (i, arg) in args.iter().enumerate() {
        if i > 0 { s.push_str(", "); }
        s.push_str(arg);
    }
    s.push(')');
    s
}

#[cfg(feature = "logging")]
pub fn format_arg(value: &Tree) -> String {
    match value {
        Tree::Scalar(b) => {
            let s = String::from_utf8_lossy(b);
            if s.len() > 50 {
                let mut out = String::from("'");
                out.push_str(&s[..47]);
                out.push_str("'...");
                out
            } else {
                let mut out = String::from("'");
                out.push_str(&s);
                out.push('\'');
                out
            }
        }
        Tree::Sequence(arr) if arr.is_empty() => String::from("[]"),
        Tree::Sequence(arr) => {
            let mut s = String::from("[");
            s.push_str(&arr.len().to_string());
            s.push_str(" items]");
            s
        }
        Tree::Mapping(obj) if obj.is_empty() => String::from("{}"),
        Tree::Mapping(obj) => {
            let mut s = String::from("{");
            s.push_str(&obj.len().to_string());
            s.push_str(" fields}");
            s
        }
        Tree::Null => String::from("null"),
    }
}

#[cfg(feature = "logging")]
pub fn format_str_arg(s: &str) -> String {
    if s.len() > 50 {
        let mut out = String::from("'");
        out.push_str(&s[..47]);
        out.push_str("'...");
        out
    } else {
        let mut out = String::from("'");
        out.push_str(s);
        out.push('\'');
        out
    }
}

#[macro_export]
macro_rules! debug_log {
    ($class:expr, $fun:expr $(, $arg:expr)*) => {{
        #[cfg(feature = "logging")]
        {
            let formatted: $crate::Vec<$crate::String> = $crate::vec![
                $( $crate::debug_log::format_str_arg($arg), )*
            ];
            let refs: $crate::Vec<&str> = formatted.iter().map(|s| s.as_str()).collect();
            log::debug!("{}", $crate::debug_log::message($class, $fun, &refs));
        }
    }};
}

#[cfg(all(test, feature = "logging"))]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn message_multiple_args() {
        let result = message("State", "get", &["'cache.user'", "null"]);
        assert_eq!(result, "State::get('cache.user', null)");
    }

    #[test]
    fn format_arg_variants() {
        assert_eq!(format_arg(&Tree::Scalar(b"text".to_vec())), "'text'");
        assert_eq!(format_arg(&Tree::Null), "null");
        assert_eq!(format_arg(&Tree::Sequence(vec![])), "[]");
        assert_eq!(format_arg(&Tree::Mapping(vec![])), "{}");
        assert_eq!(format_arg(&Tree::Sequence(vec![Tree::Null, Tree::Null, Tree::Null])), "[3 items]");
        assert_eq!(format_arg(&Tree::Mapping(vec![(b"a".to_vec(), Tree::Null)])), "{1 fields}");
    }

    #[test]
    fn format_arg_long_string_truncated() {
        let long_str = "a".repeat(60);
        let result = format_arg(&Tree::Scalar(long_str.into_bytes()));
        assert!(result.starts_with("'aaa"));
        assert!(result.ends_with("'..."));
        assert_eq!(result.len(), 52);
    }

    #[test]
    fn format_str_arg_short_and_long() {
        assert_eq!(format_str_arg("key"), "'key'");
        let long_str = "a".repeat(60);
        let result = format_str_arg(&long_str);
        assert!(result.starts_with("'aaa"));
        assert!(result.ends_with("'..."));
    }
}
