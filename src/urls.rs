//! URL extraction and source ranking for web search results.

// Only a bounded authority hostname can confer official-source priority.
// This is ranking, not URL normalization or browser navigation validation.
fn hostname(url: &str) -> Option<&str> {
    let authority = url.split_once("://")?.1.split(['/', '?', '#']).next()?;
    if authority.contains('\\')
        || authority
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
    {
        return None;
    }
    let authority = authority.rsplit('@').next()?;
    // IPv6 addresses cannot match the domain names used by this ranker.
    if authority.starts_with('[') {
        return None;
    }
    let host = if let Some((host, port)) = authority.split_once(':') {
        port.parse::<u16>().ok()?;
        host
    } else {
        authority
    };
    let host = host.strip_suffix('.').unwrap_or(host);
    if host.is_empty() { None } else { Some(host) }
}

fn domain_matches(host: Option<&str>, domain: &str) -> bool {
    host.is_some_and(|host| {
        host == domain
            || host
                .strip_suffix(domain)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

fn url_path(url: &str) -> &str {
    let Some((_, rest)) = url.split_once("://") else {
        return "";
    };
    let boundary = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let suffix = &rest[boundary..];
    if suffix.starts_with('/') {
        suffix.split(['?', '#']).next().unwrap_or("")
    } else {
        ""
    }
}

pub fn web_urls_from_result(result: &str) -> Vec<String> {
    let mut scored: Vec<(i32, String)> = Vec::new();
    let mut current_title = String::new();

    for line in result.lines() {
        let line = line.trim();

        if let Some((prefix, rest)) = line.split_once(". ") {
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                current_title = rest.trim().to_string();
                continue;
            }
        }

        let Some(raw_url) = line.strip_prefix("URL: ") else {
            continue;
        };

        let url = raw_url.trim();

        if !(url.starts_with("http://") || url.starts_with("https://")) {
            continue;
        }

        let title = current_title.to_lowercase();
        let url_lower = url.to_lowercase();

        let host = hostname(&url_lower);
        let path = url_path(&url_lower);
        let government = domain_matches(host, "government.ru");
        let mut score = 0i32;

        // Primary official sources first.
        if domain_matches(host, "publication.pravo.gov.ru") {
            score += 150;
        }

        if domain_matches(host, "pravo.gov.ru") {
            score += 130;
        }

        if government && path.starts_with("/docs/all/") {
            score += 120;
        } else if government {
            score += 90;
        }

        // Direct legal-document pages.
        if url_lower.contains("/document/")
            || url_lower.contains("cons_doc_")
            || url_lower.contains("/doc/")
            || url_lower.ends_with(".pdf")
        {
            score += 40;
        }

        // Legal databases.
        if domain_matches(host, "consultant.ru") || domain_matches(host, "garant.ru") {
            score += 50;
        }

        // Document language in title.
        if title.contains("постановлен")
            || title.contains("закон")
            || title.contains("указ")
            || title.contains("приказ")
            || title.contains("распоряжен")
            || title.contains("regulation")
            || title.contains("decree")
            || title.contains("law")
        {
            score += 20;
        }

        // Exact number.
        if title.contains("№ 87")
            || title.contains("№87")
            || title.contains("n 87")
            || title.contains("n. 87")
        {
            score += 25;
        }

        // Generic catalog/index pages must lose.
        if government && matches!(path, "/docs" | "/docs/") && !url_lower.contains('?') {
            score -= 150;
        }

        if government && path == "/docs" && url_lower.contains('?') {
            score -= 80;
        }

        scored.push((score, url.to_string()));
        current_title.clear();
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0));

    let mut urls = Vec::new();

    for (_, url) in scored {
        if !urls.contains(&url) {
            urls.push(url);
        }
    }

    urls
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranks_documents_deduplicates_and_rejects_non_http() {
        let input = "1. Catalog\nURL: https://government.ru/docs\n2. Law\nURL: https://publication.pravo.gov.ru/document/123\n3. Duplicate\nURL: https://publication.pravo.gov.ru/document/123\n4. Invalid\nURL: javascript:alert(1)";
        assert_eq!(
            web_urls_from_result(input),
            vec![
                "https://publication.pravo.gov.ru/document/123",
                "https://government.ru/docs"
            ]
        );
    }
    #[test]
    fn official_priority_uses_hostname_not_url_substrings() {
        for fake in [
            "https://government.ru.evil.test/docs/all/123",
            "https://evilgovernment.ru/docs/all/123",
            "https://evil.test\\@government.ru/docs/all/123",
            "https://evil.test/government.ru/docs/all/123",
            "https://evil.test/?source=publication.pravo.gov.ru",
            "https://publication.pravo.gov.ru@evil.test/document/123",
            "https://evil.test/?source=consultant.ru&backup=garant.ru",
        ] {
            let input = format!(
                "1. Untrusted\nURL: {fake}\n2. Official\nURL: https://government.ru/news/123"
            );
            assert_eq!(
                web_urls_from_result(&input)[0],
                "https://government.ru/news/123",
                "{fake}"
            );
        }
    }

    #[test]
    fn official_hosts_support_case_ports_subdomains_and_dns_root_dot() {
        for official in [
            "https://PUBLICATION.PRAVO.GOV.RU/document/123",
            "https://publication.pravo.gov.ru:443/document/123",
            "https://publication.pravo.gov.ru./document/123",
            "https://news.government.ru/docs/all/123",
        ] {
            let input = format!(
                "1. Document\nURL: https://example.test/doc/123\n2. Official\nURL: {official}"
            );
            assert_eq!(web_urls_from_result(&input)[0], official);
        }
    }
}
