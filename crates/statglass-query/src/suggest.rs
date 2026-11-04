//! The one did-you-mean helper, shared by the app (hosts, metrics) and the CLI
//! (subcommands), so every suggestion follows the same rule.

/// Up to three candidates within edit distance 2 of `input`, closest first, ties in
/// alphabetical order. Case-insensitive; a transposition counts as one edit (optimal
/// string alignment). Candidates keep their own spelling.
pub fn closest<'a>(input: &str, candidates: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    const MAX_DISTANCE: usize = 2;
    const MAX_SUGGESTIONS: usize = 3;
    let input = input.to_lowercase();
    let mut scored: Vec<(usize, String, &str)> = candidates
        .into_iter()
        .filter_map(|c| {
            let lower = c.to_lowercase();
            let d = strsim::osa_distance(&input, &lower);
            (d <= MAX_DISTANCE).then_some((d, lower, c))
        })
        .collect();
    scored.sort();
    scored.dedup_by(|a, b| a.2 == b.2);
    scored
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, _, c)| c.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::closest;

    #[test]
    fn transposition_counts_once() {
        // "shwo" -> "show" is one OSA edit; plain Levenshtein would call it two.
        assert_eq!(closest("shwo", ["show", "hosts", "now"]), vec!["show"]);
        assert_eq!(closest("hsots", ["hosts"]), vec!["hosts"]);
    }

    #[test]
    fn orders_by_distance_then_alphabetically_ignoring_case() {
        let got = closest("nwo", ["now", "Nwa", "nob", "top", "awo"]);
        // now: 1 (transposition); awo, Nwa: 1 substitution; nob: 2.
        assert_eq!(got, vec!["awo", "now", "Nwa"]);
        assert_eq!(
            closest("m1-LINUX", ["M1-linux", "m2-linux"]),
            vec!["M1-linux", "m2-linux"]
        );
    }

    #[test]
    fn drops_far_candidates_and_caps_at_three() {
        assert!(closest("history", ["top", "now"]).is_empty());
        assert_eq!(closest("ab", ["aa", "ac", "ad", "ae"]).len(), 3);
    }
}
