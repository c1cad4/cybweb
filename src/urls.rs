//! URL extraction and source ranking for web search results.

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

        let mut score = 0i32;

        // Primary official sources first.
        if url_lower.contains("publication.pravo.gov.ru") {
            score += 150;
        }

        if url_lower.contains("pravo.gov.ru") {
            score += 130;
        }

        if url_lower.contains("government.ru/docs/all/") {
            score += 120;
        } else if url_lower.contains("government.ru") {
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
        if url_lower.contains("consultant.ru") || url_lower.contains("garant.ru") {
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
        if url_lower == "https://government.ru/docs" || url_lower == "https://government.ru/docs/" {
            score -= 150;
        }

        if url_lower.contains("government.ru/docs?") {
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
}
