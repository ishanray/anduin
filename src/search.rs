use crate::git::diff::FileStatus;

pub const SEARCH_CONTEXT_RADIUS: usize = 5;
pub const SEARCH_DEBOUNCE_MS: u64 = 300;
pub const SEARCH_LINE_HEIGHT: f32 = 22.0;
pub const SEARCH_FILE_HEADER_HEIGHT: f32 = 32.0;
pub const SEARCH_FILE_SECTION_SPACING: f32 = 16.0;
pub const SEARCH_MATCH_BLOCK_PADDING: f32 = 12.0;
pub const SEARCH_MATCH_BLOCK_SPACING: f32 = 8.0;

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSearchResult {
    pub file_path: String,
    pub file_status: FileStatus,
    pub matches: Vec<MatchContext>,
    pub total_matches: usize,
    pub total_matches_display: String,
    pub estimated_scroll_y: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchContext {
    pub start_line: usize,
    pub end_line: usize,
    pub lines: Vec<ContextLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextLine {
    pub line_number: usize,
    pub line_number_display: String,
    pub text: String,
    pub is_match: bool,
}

pub fn find_case_insensitive(haystack: &str, needle_lower: &str) -> Option<usize> {
    haystack.to_lowercase().find(needle_lower)
}

pub fn find_match_line_indices_with_lower(
    raw_diff: &str,
    raw_diff_lower: Option<&str>,
    query: &str,
    query_lower: Option<&str>,
    case_sensitive: bool,
) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }

    if case_sensitive {
        return raw_diff
            .lines()
            .enumerate()
            .filter_map(|(line_number, line)| line.contains(query).then_some(line_number))
            .collect();
    }

    if let (Some(diff_lower), Some(needle_lower)) = (raw_diff_lower, query_lower) {
        return diff_lower
            .lines()
            .enumerate()
            .filter_map(|(line_number, line)| line.contains(needle_lower).then_some(line_number))
            .collect();
    }

    let needle_lower = query_lower
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| query.to_lowercase());
    let diff_lower = raw_diff.to_lowercase();
    diff_lower
        .lines()
        .enumerate()
        .filter_map(|(line_number, line)| line.contains(&needle_lower).then_some(line_number))
        .collect()
}

pub fn extract_match_contexts(
    raw_diff: &str,
    match_line_indices: &[usize],
    context_radius: usize,
) -> Vec<MatchContext> {
    let lines: Vec<&str> = raw_diff.lines().collect();
    extract_match_contexts_from_lines(&lines, match_line_indices, context_radius)
}

pub fn extract_match_contexts_from_lines(
    lines: &[&str],
    match_line_indices: &[usize],
    context_radius: usize,
) -> Vec<MatchContext> {
    if lines.is_empty() || match_line_indices.is_empty() {
        return Vec::new();
    }

    let mut sorted_matches = match_line_indices.to_vec();
    sorted_matches.sort_unstable();
    sorted_matches.dedup();

    let mut contexts = Vec::new();
    let mut start_line = sorted_matches[0].saturating_sub(context_radius);
    let mut end_line = (sorted_matches[0] + context_radius).min(lines.len().saturating_sub(1));
    let mut current_matches = vec![sorted_matches[0]];

    for &line_number in sorted_matches.iter().skip(1) {
        let candidate_start = line_number.saturating_sub(context_radius);
        let candidate_end = (line_number + context_radius).min(lines.len().saturating_sub(1));

        if candidate_start <= end_line.saturating_add(1) {
            end_line = end_line.max(candidate_end);
            current_matches.push(line_number);
        } else {
            contexts.push(build_match_context(
                lines,
                start_line,
                end_line,
                &current_matches,
            ));
            start_line = candidate_start;
            end_line = candidate_end;
            current_matches.clear();
            current_matches.push(line_number);
        }
    }

    contexts.push(build_match_context(
        lines,
        start_line,
        end_line,
        &current_matches,
    ));
    contexts
}

pub fn estimate_result_height(result: &ProjectSearchResult) -> f32 {
    let mut height = SEARCH_FILE_HEADER_HEIGHT + SEARCH_FILE_SECTION_SPACING;

    for context in &result.matches {
        height += SEARCH_MATCH_BLOCK_PADDING * 2.0;
        height += context.lines.len() as f32 * SEARCH_LINE_HEIGHT;
        height += SEARCH_MATCH_BLOCK_SPACING;
    }

    height
}

fn build_match_context(
    lines: &[&str],
    start_line: usize,
    end_line: usize,
    match_line_indices: &[usize],
) -> MatchContext {
    let lines = (start_line..=end_line)
        .map(|line_number| ContextLine {
            line_number,
            line_number_display: format!("{:>4}", line_number + 1),
            text: lines[line_number].to_owned(),
            is_match: match_line_indices.binary_search(&line_number).is_ok(),
        })
        .collect();

    MatchContext {
        start_line,
        end_line,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff::FileStatus;

    #[test]
    fn finds_case_insensitive_matches() {
        let diff = "@@ -1 +1 @@\n-Foo\n+bar foo\n context";
        let matches = find_match_line_indices_with_lower(diff, None, "foo", Some("foo"), false);

        assert_eq!(matches, vec![1, 2]);
    }

    #[test]
    fn finds_case_sensitive_matches() {
        let diff = "@@ -1 +1 @@\n-Foo\n+foo\n+FOO";
        let matches = find_match_line_indices_with_lower(diff, None, "foo", None, true);

        assert_eq!(matches, vec![2]);
    }

    #[test]
    fn merges_overlapping_match_contexts() {
        let lines = vec!["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
        let contexts = extract_match_contexts_from_lines(&lines, &[2, 4], 2);

        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].start_line, 0);
        assert_eq!(contexts[0].end_line, 6);
        assert!(contexts[0].lines[2].is_match);
        assert!(contexts[0].lines[4].is_match);
    }

    #[test]
    fn splits_distant_match_contexts() {
        let lines = vec!["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"];
        let contexts = extract_match_contexts_from_lines(&lines, &[1, 10], 1);

        assert_eq!(contexts.len(), 2);
        assert_eq!(contexts[0].start_line, 0);
        assert_eq!(contexts[0].end_line, 2);
        assert_eq!(contexts[1].start_line, 9);
        assert_eq!(contexts[1].end_line, 11);
    }

    #[test]
    fn deduplicates_match_line_indices_in_contexts() {
        let lines = vec!["0", "1", "2", "3", "4"];
        let contexts = extract_match_contexts_from_lines(&lines, &[2, 2, 2], 1);

        assert_eq!(contexts.len(), 1);
        let match_lines: Vec<_> = contexts[0]
            .lines
            .iter()
            .filter(|line| line.is_match)
            .map(|line| line.line_number)
            .collect();
        assert_eq!(match_lines, vec![2]);
    }

    #[test]
    fn estimates_height_includes_header_and_blocks() {
        let result = ProjectSearchResult {
            file_path: "src/main.rs".into(),
            file_status: FileStatus::Modified,
            total_matches_display: "2".into(),
            total_matches: 2,
            estimated_scroll_y: 0.0,
            matches: vec![MatchContext {
                start_line: 0,
                end_line: 2,
                lines: vec![
                    ContextLine {
                        line_number: 0,
                        line_number_display: "   1".into(),
                        text: "a".into(),
                        is_match: false,
                    },
                    ContextLine {
                        line_number: 1,
                        line_number_display: "   2".into(),
                        text: "b".into(),
                        is_match: true,
                    },
                    ContextLine {
                        line_number: 2,
                        line_number_display: "   3".into(),
                        text: "c".into(),
                        is_match: false,
                    },
                ],
            }],
        };

        let height = estimate_result_height(&result);
        assert!(height > SEARCH_FILE_HEADER_HEIGHT);
    }
}
