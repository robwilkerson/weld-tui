/// Expand tab characters to spaces, respecting tab stops at multiples of
/// `tab_width`.
pub fn expand_tabs(s: &str, tab_width: usize) -> String {
    let mut result = String::with_capacity(s.len());
    let mut col = 0;
    for ch in s.chars() {
        if ch == '\t' {
            let spaces = tab_width - (col % tab_width);
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
        assert_eq!(expand_tabs("hello", 4), "hello");
    }

    #[test]
    fn tab_at_start() {
        assert_eq!(expand_tabs("\thello", 4), "    hello");
    }

    #[test]
    fn tab_mid_word() {
        // "ab" is 2 chars, so tab at col 2 → 2 spaces to reach col 4
        assert_eq!(expand_tabs("ab\tc", 4), "ab  c");
    }

    #[test]
    fn multiple_tabs() {
        assert_eq!(expand_tabs("\t\t", 4), "        ");
    }

    #[test]
    fn empty_string() {
        assert_eq!(expand_tabs("", 4), "");
    }

    #[test]
    fn honors_custom_tab_width() {
        assert_eq!(expand_tabs("\ta", 2), "  a");
        assert_eq!(expand_tabs("\ta", 8), "        a");
    }
}
