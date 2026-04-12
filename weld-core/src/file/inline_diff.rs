use similar::{ChangeTag, TextDiff};

/// A segment of an inline (character-level) diff within a single line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSegment {
    pub kind: InlineKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineKind {
    Equal,
    Changed,
}

/// Character-level diff between two lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineDiff {
    pub left_segments: Vec<InlineSegment>,
    pub right_segments: Vec<InlineSegment>,
}

impl InlineDiff {
    /// Compute character-level diff between two lines.
    pub fn compute(left_line: &str, right_line: &str) -> Self {
        let diff = TextDiff::from_words(left_line, right_line);
        let mut left_segments = Vec::new();
        let mut right_segments = Vec::new();

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    let text = change.value().to_string();
                    left_segments.push(InlineSegment {
                        kind: InlineKind::Equal,
                        text: text.clone(),
                    });
                    right_segments.push(InlineSegment {
                        kind: InlineKind::Equal,
                        text,
                    });
                }
                ChangeTag::Delete => {
                    left_segments.push(InlineSegment {
                        kind: InlineKind::Changed,
                        text: change.value().to_string(),
                    });
                }
                ChangeTag::Insert => {
                    right_segments.push(InlineSegment {
                        kind: InlineKind::Changed,
                        text: change.value().to_string(),
                    });
                }
            }
        }

        InlineDiff {
            left_segments: merge_segments(left_segments),
            right_segments: merge_segments(right_segments),
        }
    }
}

fn merge_segments(segments: Vec<InlineSegment>) -> Vec<InlineSegment> {
    let mut merged: Vec<InlineSegment> = Vec::new();
    for seg in segments {
        if let Some(last) = merged.last_mut()
            && last.kind == seg.kind
        {
            last.text.push_str(&seg.text);
            continue;
        }
        merged.push(seg);
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_lines_produce_single_equal_segment() {
        let result = InlineDiff::compute("hello world", "hello world");
        assert_eq!(result.left_segments.len(), 1);
        assert_eq!(result.left_segments[0].kind, InlineKind::Equal);
        assert_eq!(result.left_segments[0].text, "hello world");
    }

    #[test]
    fn completely_different_lines() {
        let result = InlineDiff::compute("aaa", "bbb");
        assert!(
            result
                .left_segments
                .iter()
                .any(|s| s.kind == InlineKind::Changed)
        );
        assert!(
            result
                .right_segments
                .iter()
                .any(|s| s.kind == InlineKind::Changed)
        );
    }

    #[test]
    fn partial_change_detected() {
        let result = InlineDiff::compute("Host: \"localhost\",", "Host: \"0.0.0.0\",");
        // Word-level: "Host: \"" is equal, the value differs, trailing "\"," is equal.
        assert!(result.left_segments.len() >= 2);
        assert!(
            result
                .left_segments
                .iter()
                .any(|s| s.kind == InlineKind::Equal)
        );
        assert!(
            result
                .left_segments
                .iter()
                .any(|s| s.kind == InlineKind::Changed)
        );
    }

    #[test]
    fn empty_left_line() {
        let result = InlineDiff::compute("", "added text");
        assert!(
            result.left_segments.is_empty()
                || result.left_segments.iter().all(|s| s.text.is_empty())
        );
        assert!(
            result
                .right_segments
                .iter()
                .any(|s| s.kind == InlineKind::Changed)
        );
    }

    #[test]
    fn empty_right_line() {
        let result = InlineDiff::compute("removed text", "");
        assert!(
            result
                .left_segments
                .iter()
                .any(|s| s.kind == InlineKind::Changed)
        );
        assert!(
            result.right_segments.is_empty()
                || result.right_segments.iter().all(|s| s.text.is_empty())
        );
    }
}
