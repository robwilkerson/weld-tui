const TAB_WIDTH: usize = 4;

/// Expand tab characters to spaces, respecting tab stops.
pub fn expand_tabs(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut col = 0;
    for ch in s.chars() {
        if ch == '\t' {
            let spaces = TAB_WIDTH - (col % TAB_WIDTH);
            result.extend(std::iter::repeat_n(' ', spaces));
            col += spaces;
        } else {
            result.push(ch);
            col += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_tabs() {
        assert_eq!(expand_tabs("hello"), "hello");
    }

    #[test]
    fn tab_at_start() {
        assert_eq!(expand_tabs("\thello"), "    hello");
    }

    #[test]
    fn tab_mid_word() {
        // "ab" is 2 chars, so tab at col 2 → 2 spaces to reach col 4
        assert_eq!(expand_tabs("ab\tc"), "ab  c");
    }

    #[test]
    fn multiple_tabs() {
        assert_eq!(expand_tabs("\t\t"), "        ");
    }

    #[test]
    fn empty_string() {
        assert_eq!(expand_tabs(""), "");
    }
}
