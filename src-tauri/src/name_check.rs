//! Heuristic check for likely name-entry typos, used by warning W-8.
//!
//! Ported from Mac SQBS 2.0.1 `+[Tournament nameHasUnusualCapitalization:]`.
//! Deliberately conservative: nobiliary particles, Mc/Mac/O'/D'/L' patterns and
//! roman-numeral suffixes are whitelisted so the warning stays quiet on real
//! names and fires on things like "smith", "SMITH", or "SMith".

/// Lowercase nobiliary / particle words that legitimately stay lowercase.
const PARTICLES: &[&str] = &[
    "de", "del", "della", "di", "da", "van", "von", "der", "den", "la", "le",
    "el", "al", "bin", "ibn", "ten", "ter", "of", "the",
];

/// Punctuation stripped from both ends of a token before checking, so
/// pseudonyms and grades like "(Wesley)" or "(12)" don't trip the heuristic.
const WRAPPING: &[char] = &['(', ')', '[', ']', '{', '}', '.', ','];

const ROMAN: &[char] = &['I', 'V', 'X', 'L', 'C', 'D', 'M'];

/// Returns true when `name` looks like a capitalization typo.
#[must_use]
pub fn name_has_unusual_capitalization(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    for raw_token in trimmed.split_whitespace() {
        let token = raw_token.trim_matches(WRAPPING);
        if token.is_empty() || !token.chars().any(char::is_alphabetic) {
            continue; // empty or non-letter token (numbers, symbols)
        }
        if PARTICLES.contains(&token.to_lowercase().as_str()) {
            continue; // whitelisted lowercase particle
        }
        // Whitelist Mc / Mac / O' / D' / L' name patterns with internal caps.
        let mc_mac_o = token.chars().count() >= 3
            && ["Mc", "Mac", "O'", "D'", "L'"].iter().any(|p| token.starts_with(p));

        // Rule 1: leading lowercase letter (and not a whitelisted particle).
        if token.chars().next().is_some_and(char::is_lowercase) {
            return true;
        }
        // Rule 2: 2+ consecutive capitals followed by a lowercase, e.g. "SMith".
        if !mc_mac_o {
            let mut cap_run = 0_usize;
            for c in token.chars() {
                if c.is_uppercase() {
                    cap_run += 1;
                } else {
                    if cap_run >= 2 && c.is_lowercase() {
                        return true;
                    }
                    cap_run = 0;
                }
            }
        }
        // Rule 3: ALL-CAPS token of length >= 3 (e.g. "SMITH"), excluding roman numerals.
        if token == token.to_uppercase()
            && token.chars().count() >= 3
            && !token.trim_matches(ROMAN).is_empty()
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::name_has_unusual_capitalization as unusual;

    #[test]
    fn ordinary_names_are_quiet() {
        for n in ["Alice Smith", "Bob", "Mary-Jane O'Connor", "Jean-Luc Picard"] {
            assert!(!unusual(n), "{n} should not warn");
        }
    }

    #[test]
    fn leading_lowercase_warns() {
        assert!(unusual("alice Smith"));
        assert!(unusual("Alice smith"));
    }

    #[test]
    fn all_caps_warns() {
        assert!(unusual("SMITH"));
        assert!(unusual("Alice SMITH"));
    }

    #[test]
    fn internal_caps_warn() {
        assert!(unusual("SMith"));
    }

    #[test]
    fn whitelisted_patterns_are_quiet() {
        for n in [
            "Ludwig van Beethoven",
            "Vincent van Gogh",
            "Oscar de la Renta",
            "Ronald McDonald",
            "Peter MacArthur",
            "Sean O'Brien",
            "John Smith III",
            "Henry VIII",
        ] {
            assert!(!unusual(n), "{n} should not warn");
        }
    }

    #[test]
    fn short_all_caps_and_non_letters_are_quiet() {
        assert!(!unusual("JD"));       // initials, under the length-3 rule
        assert!(!unusual("Bob (12)")); // grade in parens
        assert!(!unusual(""));
        assert!(!unusual("   "));
    }
}
