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

/// True for a canonically spelled roman numeral in 1..=3999 ("III", "XIV").
/// Round-tripping through the canonical spelling rejects letter soup that
/// merely uses roman characters ("CIVIL", "MILL", "DIM").
fn is_roman_numeral(s: &str) -> bool {
    const VALUES: [(char, u32); 7] = [
        ('I', 1), ('V', 5), ('X', 10), ('L', 50), ('C', 100), ('D', 500), ('M', 1000),
    ];
    let value_of = |c: char| VALUES.iter().find(|(ch, _)| *ch == c).map(|(_, v)| *v);

    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return false;
    }
    // Signed accumulator: a leading subtractive pair can dip below zero
    // ("IM"), which is not a numeral but must not overflow on the way out.
    let mut total: i64 = 0;
    for (i, &c) in chars.iter().enumerate() {
        let Some(v) = value_of(c) else { return false };
        let next = chars.get(i + 1).copied().and_then(value_of);
        if next.is_some_and(|n| n > v) {
            total -= i64::from(v);
        } else {
            total += i64::from(v);
        }
    }
    u32::try_from(total).ok()
        .and_then(to_roman)
        .is_some_and(|canonical| canonical == s)
}

/// Canonical roman spelling of `n`, or `None` outside 1..=3999.
fn to_roman(n: u32) -> Option<String> {
    const TABLE: [(u32, &str); 13] = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
        (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    if n == 0 || n > 3999 {
        return None;
    }
    let mut out = String::new();
    let mut rem = n;
    for (value, numeral) in TABLE {
        while rem >= value {
            out.push_str(numeral);
            rem -= value;
        }
    }
    Some(out)
}

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
        // The whitelist exists so lowercase particles don't trip Rule 1; an
        // uppercase variant ("VAN") is not a particle spelling and still gets
        // the all-caps check.
        if PARTICLES.contains(&token) {
            continue;
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
        // Rule 3: ALL-CAPS token of length >= 3 (e.g. "SMITH"), excluding
        // generational suffixes. Only well-formed numerals are excused —
        // merely being spelled from roman letters is not enough, or names like
        // "CIVIL" and "MILL" would slip through.
        if token == token.to_uppercase()
            && token.chars().count() >= 3
            && !is_roman_numeral(token)
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
    fn roman_lookalikes_still_warn() {
        // Spelled from roman letters, but not numerals — must not be excused.
        // ("MIX" is left out on purpose: it really is 1009.)
        for n in ["CIVIL", "MILL", "DIM", "LIL"] {
            assert!(unusual(n), "{n} should warn");
        }
    }

    #[test]
    fn uppercase_particle_is_not_whitelisted() {
        // The whitelist covers lowercase spellings only; "VAN" is all-caps.
        assert!(unusual("Ludwig VAN Beethoven"));
    }

    #[test]
    fn roman_numeral_validator() {
        use super::is_roman_numeral;
        for ok in ["I", "III", "IV", "VIII", "XIV", "XL", "MCMXCIV", "MMXXVI"] {
            assert!(is_roman_numeral(ok), "{ok} should be a numeral");
        }
        for bad in ["", "IIII", "VV", "IM", "CIVIL", "MILL", "ABC", "XIIX"] {
            assert!(!is_roman_numeral(bad), "{bad} should not be a numeral");
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
