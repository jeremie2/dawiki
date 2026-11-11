use ammonia::Builder;

use crate::error::AppError;

pub fn sanitize_page_html(unsafe_html: &str) -> Result<String, AppError> {
    let mut cleaner = Builder::default();

    cleaner
        .add_tags(["details", "summary"])
        .add_generic_attributes(["class", "id", "align", "width", "height"])
        .add_tag_attributes("a", ["target"])
        .add_tag_attributes("img", ["loading", "decoding"]);

    Ok(cleaner.clean(unsafe_html).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_simple_html() {
        let input = "<p>Testo</p>";
        let output = sanitize_page_html(input).unwrap();

        assert_eq!(output, "<p>Testo</p>");
    }

    #[test]
    fn preserves_allowed_tag_and_class() {
        let input = "<div class=\"wiki-alert\">Attenzione</div>";
        let output = sanitize_page_html(input).unwrap();

        assert!(output.contains("class=\"wiki-alert\""));
    }

    #[test]
    fn removes_script_tag() {
        let input = "<p>Sicuro</p><script>alert('x')</script>";
        let output = sanitize_page_html(input).unwrap();

        assert!(!output.contains("<script>"));
        assert!(!output.contains("alert"));
    }

    #[test]
    fn removes_event_handler() {
        let input = "<p onclick=\"alert('x')\">Testo</p>";
        let output = sanitize_page_html(input).unwrap();

        assert!(!output.contains("onclick"));
    }

    #[test]
    fn removes_javascript_url() {
        let input = "<a href=\"javascript:alert('x')\">Link</a>";
        let output = sanitize_page_html(input).unwrap();

        assert!(!output.contains("javascript:"));
    }

    #[test]
    fn preserves_target_attribute() {
        let input = "<a href=\"https://example.com\" target=\"_blank\">Link</a>";
        let output = sanitize_page_html(input).unwrap();

        assert!(output.contains("target=\"_blank\""));
    }

    #[test]
    fn preserves_image_attributes() {
        let input = "<img src=\"https://example.com/image.png\" loading=\"lazy\" decoding=\"async\">";
        let output = sanitize_page_html(input).unwrap();

        assert!(output.contains("loading=\"lazy\""));
        assert!(output.contains("decoding=\"async\""));
    }
}