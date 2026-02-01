//! Markdown code block extraction.

use regex::Regex;
use std::sync::OnceLock;

/// A parsed code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub content: String,
}

/// Static regex for code block extraction.
fn code_block_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"```(\w*)\n([\s\S]*?)```").ok())
        .as_ref()
}

/// Extract all fenced code blocks from Markdown text.
#[must_use]
pub fn extract_code_blocks(text: &str) -> Vec<CodeBlock> {
    // Match fenced code blocks: ```lang\ncontent\n```
    // The regex handles:
    // - Optional language tag after opening ```
    // - Content between fences (non-greedy)
    // - Closing ```
    let Some(re) = code_block_regex() else {
        return Vec::new();
    };

    re.captures_iter(text)
        .map(|cap| {
            let language = cap.get(1).map(|m| m.as_str()).filter(|s| !s.is_empty());
            let content = cap.get(2).map_or("", |m| m.as_str());

            CodeBlock {
                language: language.map(ToString::to_string),
                content: content.to_string(),
            }
        })
        .collect()
}

/// Extract the first code block with a specific language tag.
/// If `language` is None, returns the first code block regardless of language.
#[must_use]
pub fn extract_code_block(text: &str, language: Option<&str>) -> Option<CodeBlock> {
    let blocks = extract_code_blocks(text);

    match language {
        Some(lang) => blocks
            .into_iter()
            .find(|b| b.language.as_deref() == Some(lang)),
        None => blocks.into_iter().next(),
    }
}

/// Get file extension for a language tag.
#[must_use]
pub fn language_to_extension(language: &str) -> &str {
    match language.to_lowercase().as_str() {
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "python" | "py" => "py",
        "rust" | "rs" => "rs",
        "svelte" => "svelte",
        "json" => "json",
        "html" => "html",
        "css" => "css",
        "bash" | "sh" | "shell" => "sh",
        _ => "txt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_single_block() {
        let text = r"
Some text before

```javascript
const x = 1;
```

Some text after
";

        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, Some("javascript".to_string()));
        assert_eq!(blocks[0].content, "const x = 1;\n");
    }

    #[test]
    fn test_extract_multiple_blocks() {
        let text = r"
```python
x = 1
```

```rust
let x = 1;
```
";

        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, Some("python".to_string()));
        assert_eq!(blocks[1].language, Some("rust".to_string()));
    }

    #[test]
    fn test_extract_block_without_language() {
        let text = r"
```
plain text
```
";

        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, None);
        assert_eq!(blocks[0].content, "plain text\n");
    }

    #[test]
    fn test_extract_by_language() -> Result<(), &'static str> {
        let text = r"
```javascript
const x = 1;
```

```python
x = 1
```
";

        let block = extract_code_block(text, Some("python"));
        let block = block.ok_or("expected python block")?;
        assert_eq!(block.language, Some("python".to_string()));

        let block = extract_code_block(text, Some("rust"));
        assert!(block.is_none());
        Ok(())
    }

    #[test]
    fn test_extract_first_block() -> Result<(), &'static str> {
        let text = r"
```javascript
const x = 1;
```

```python
x = 1
```
";

        let block = extract_code_block(text, None);
        let block = block.ok_or("expected first block")?;
        assert_eq!(block.language, Some("javascript".to_string()));
        Ok(())
    }

    #[test]
    fn test_no_blocks() {
        let text = "Just plain text without code blocks";
        let blocks = extract_code_blocks(text);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_multiline_content() {
        let text = r"
```svelte
<script>
    let count = $state(0);
</script>

<button onclick={() => count++}>
    {count}
</button>
```
";

        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].content.contains("$state"));
        assert!(blocks[0].content.contains("<button"));
    }

    #[test]
    fn test_language_to_extension() {
        assert_eq!(language_to_extension("javascript"), "js");
        assert_eq!(language_to_extension("js"), "js");
        assert_eq!(language_to_extension("typescript"), "ts");
        assert_eq!(language_to_extension("python"), "py");
        assert_eq!(language_to_extension("rust"), "rs");
        assert_eq!(language_to_extension("svelte"), "svelte");
        assert_eq!(language_to_extension("unknown"), "txt");
    }
}
