const EXACT_HTTPS_DESTINATIONS: &[&str] = &[
    "https://heardright.app",
    "https://heardright.app/pricing",
    "https://heardright.app/voice-commands",
    "https://heardright.app/formatting",
    "https://creativecommons.org/licenses/by/4.0/",
    "https://instagram.com/bogusyogi",
    "https://console.groq.com/login",
    "https://console.groq.com/keys",
    "https://openrouter.ai/sign-up",
    "https://openrouter.ai/settings/keys",
];

/// Return whether a renderer-authored link is one of HeardRight's approved
/// HTTPS destinations. Every destination is an exact current browser link;
/// userinfo, ports, alternate paths, query/fragment suffixes, and encoded
/// delimiter tricks cannot pass without adding a URL parser dependency.
pub fn is_allowed_external_url(url: &str) -> bool {
    if url.is_empty()
        || url
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return false;
    }

    EXACT_HTTPS_DESTINATIONS.contains(&url)
}

#[cfg(test)]
mod tests {
    use super::is_allowed_external_url;

    #[test]
    fn allows_current_app_authored_https_destinations() {
        for url in [
            "https://heardright.app",
            "https://heardright.app/pricing",
            "https://heardright.app/formatting",
            "https://heardright.app/voice-commands",
            "https://creativecommons.org/licenses/by/4.0/",
            "https://instagram.com/bogusyogi",
            "https://console.groq.com/login",
            "https://console.groq.com/keys",
            "https://openrouter.ai/sign-up",
            "https://openrouter.ai/settings/keys",
        ] {
            assert!(is_allowed_external_url(url), "expected allowed: {url}");
        }
    }

    #[test]
    fn rejects_wrong_scheme_authority_and_unrelated_destinations() {
        for url in [
            "http://heardright.app/pricing",
            "https://heardright.app.evil.example/pricing",
            "https://heardright.app@evil.example/pricing",
            "https://heardright.app:443/pricing",
            "https://evil.example/https://heardright.app/pricing",
            "https://heardright.app/",
            "https://heardright.app/account",
            "https://heardright.app/pricing/",
            "https://heardright.app/pricing?source=app",
            "https://heardright.app/pricing#buy",
            "https://heardright.app/%70ricing",
            "https://heardright.app/pricing%00ignored",
            "https://heardright.app/pricing%0d%0ahttps://evil.example",
            "https://heardright.app/pricing/%2e%2e/account",
            "https://heardright.app/voice-commands%2fextra",
            "https://heardright.app/formatting%5cextra",
            "https://heardright.app//evil.example",
            "https://creativecommons.org/licenses/by/4.0/extra",
            "https://instagram.com/bogusyogi/extra",
            "https://console.groq.com/login/extra",
            "https://openrouter.ai/settings/keys/extra",
            "https://example.com/",
        ] {
            assert!(!is_allowed_external_url(url), "expected rejected: {url}");
        }
    }

    #[test]
    fn rejects_nuls_and_ambiguous_whitespace() {
        for url in [
            "https://heardright.app/pricing\0https://evil.example",
            "https://heardright.app/\r\nhttps://evil.example",
            "https://heardright.app/price\ttracking",
            " https://heardright.app/pricing",
            "https://heardright.app/pricing ",
            "",
        ] {
            assert!(!is_allowed_external_url(url), "expected rejected: {url:?}");
        }
    }
}
