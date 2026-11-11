use std::fmt;

#[derive(Debug)]
pub enum PagePathError {
    Empty,
    TooLong,
    InvalidSegment(String),
}

impl fmt::Display for PagePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PagePathError::Empty => write!(formatter, "Il percorso della pagina è vuoto."),
            PagePathError::TooLong => write!(
                formatter,
                "Il percorso della pagina è troppo lungo."
            ),
            PagePathError::InvalidSegment(segment) => write!(
                formatter,
                "Segmento di percorso non valido: “{segment}”."
            ),
        }
    }
}

pub fn normalize_page_path(raw_path: &str) -> Result<String, PagePathError> {
    const MAXIMUM_PATH_LENGTH: usize = 255;

    let normalized = raw_path.trim();

    if normalized.is_empty() {
        return Err(PagePathError::Empty);
    }

    if normalized.len() > MAXIMUM_PATH_LENGTH {
        return Err(PagePathError::TooLong);
    }

    let mut segments = Vec::new();

    for raw_segment in normalized.split('/') {
        let segment = raw_segment.trim();

        if segment.is_empty() {
            continue;
        }

        if segment == "." || segment == ".." {
            return Err(PagePathError::InvalidSegment(segment.to_string()));
        }

        let cleaned_segment = normalize_segment(segment);

        if cleaned_segment.is_empty() {
            return Err(PagePathError::InvalidSegment(segment.to_string()));
        }

        segments.push(cleaned_segment);
    }

    if segments.is_empty() {
        return Err(PagePathError::Empty);
    }

    Ok(segments.join("/"))
}

fn normalize_segment(raw_segment: &str) -> String {
    raw_segment
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else if character == '-' || character == '_' {
                character
            } else if character.is_whitespace() {
                '-'
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn page_title_from_path(page_path: &str) -> String {
    page_path
        .split('/')
        .last()
        .unwrap_or("Pagina")
        .to_string()
}

pub fn breadcrumb_items_from_path(page_path: &str) -> Vec<(String, String)> {
    let segments: Vec<&str> = page_path.split('/').collect();
    let mut items = Vec::new();

    for index in 0..segments.len() {
        let path = segments[..=index].join("/");
        let title = page_title_from_path(&path);

        items.push((title, path));
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_simple_path() {
        let result = normalize_page_path("Hardware/Video/Amd");

        assert_eq!(result.unwrap(), "Hardware/Video/Amd");
    }

    #[test]
    fn removes_leading_and_trailing_slashes() {
        let result = normalize_page_path("/Hardware/Video/Amd/");

        assert_eq!(result.unwrap(), "Hardware/Video/Amd");
    }

    #[test]
    fn removes_duplicate_slashes() {
        let result = normalize_page_path("Hardware//Video///Amd");

        assert_eq!(result.unwrap(), "Hardware/Video/Amd");
    }

    #[test]
    fn replaces_spaces_with_hyphens() {
        let result = normalize_page_path("Hardware/Video Card/Amd");

        assert_eq!(result.unwrap(), "Hardware/Video-Card/Amd");
    }

    #[test]
    fn rejects_parent_directory_segment() {
        let result = normalize_page_path("Hardware/../Amd");

        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty_path() {
        let result = normalize_page_path("   ");

        assert!(result.is_err());
    }

    #[test]
    fn extracts_last_segment_as_title() {
        let title = page_title_from_path("Hardware/Video/Amd");

        assert_eq!(title, "Amd");
    }

    #[test]
    fn creates_breadcrumb_items() {
        let breadcrumb = breadcrumb_items_from_path("Hardware/Video/Amd");

        assert_eq!(breadcrumb.len(), 3);
        assert_eq!(breadcrumb[0].0, "Hardware");
        assert_eq!(breadcrumb[1].0, "Video");
        assert_eq!(breadcrumb[2].0, "Amd");
    }
}
