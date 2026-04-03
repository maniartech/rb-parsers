use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_ejs_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: true,
        continue_on_error: true,
        error_tolerance_limit: 10,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // HTML content fallback (adding this first so it has lowest priority)
    tokenizer.add_closure_scanner(Box::new(|input: &str| -> Result<Option<rb_tokenizer::tokens::Token>, rb_tokenizer::tokens::TokenizationError> {
        // If the input starts with any part of an EJS tag, let the block scanners handle it
        if input.starts_with("<%") || input.starts_with("<%=") || input.starts_with("<%#") ||
           input.starts_with("<%-") || input.starts_with("%>") {
            return Ok(None);
        }

        // Find the next potential EJS tag start or end
        let next_tag_start = input.find("<%");
        let next_tag_end = input.find("%>");

        // Determine how much HTML content to capture
        let html_content = match (next_tag_start, next_tag_end) {
            // If we find a tag end before a tag start, this is invalid
            (None, Some(_)) => return Ok(None),
            // If we find a tag start, capture up to it
            (Some(pos), _) if pos > 0 => &input[..pos],
            // If we find a tag with no content before it, skip it
            (Some(_), _) => "",
            // If we find no tags, take all remaining content
            (None, None) => input,
        };

        if html_content.is_empty() {
            Ok(None)
        } else {
            Ok(Some(rb_tokenizer::tokens::Token {
                token_type: "HTML",
                token_sub_type: None,
                value: html_content.to_string(),
                line: 0,
                column: 0,
            }))
        }
    }));

    // EJS tags - order matters, more specific patterns first
    // Comments: <%# comment %>
    tokenizer.add_block_scanner("<%#", "%>", "EJS", Some("Comment"), false, true, true);

    // Output expression: <%= expression %>
    tokenizer.add_block_scanner("<%=", "%>", "EJS", Some("Output"), false, false, true);

    // Unescaped output: <%- unescaped %>
    tokenizer.add_block_scanner("<%-", "%>", "EJS", Some("Unescaped"), false, false, true);

    // Code blocks: <% code %> (most general pattern last)
    tokenizer.add_block_scanner("<%", "%>", "EJS", Some("Code"), false, false, true);

    tokenizer
}

#[cfg(test)]
mod ejs_tests {
    use super::*;

    #[test]
    fn test_basic_ejs_template() {
        let tokenizer = get_ejs_tokenizer();
        let template = "<html><body>\n<h1><%= title %></h1>\n<% if (showList) { %>\n<ul>\n<% items.forEach(function(item) { %>\n<li><%= item %></li>\n<% }); %>\n</ul>\n<% } %>\n</body></html>";

        let result = tokenizer.tokenize(template).expect("Tokenization failed");

        // Print tokens for debugging
        for (i, token) in result.iter().enumerate() {
            println!("{}: {:?} - '{}'", i, token.token_type, token.value);
        }

        // Verify we have alternating HTML and EJS sections
        assert!(result.len() > 5, "Should have multiple tokens");

        // Check that the first token is HTML
        assert_eq!(result[0].token_type, "HTML");
        assert_eq!(result[0].value, "<html><body>\n<h1>");

        // Check that the second token is EJS Output
        assert_eq!(result[1].token_type, "EJS");
        assert_eq!(result[1].token_sub_type.as_deref().unwrap_or(""), "Output");
        assert!(result[1].value.contains("title"));
    }

    #[test]
    fn test_ejs_comment() {
        let tokenizer = get_ejs_tokenizer();
        let template = "Hello <%# This is a comment %> World";

        let result = tokenizer.tokenize(template).expect("Tokenization failed");

        assert_eq!(result.len(), 3, "Should have HTML, Comment, HTML tokens");

        assert_eq!(result[0].token_type, "HTML");
        assert_eq!(result[0].value, "Hello ");

        assert_eq!(result[1].token_type, "EJS");
        assert_eq!(result[1].token_sub_type.as_deref().unwrap_or(""), "Comment");
        assert_eq!(result[1].value, "<%# This is a comment %>");

        assert_eq!(result[2].token_type, "HTML");
        assert_eq!(result[2].value, " World");
    }

    #[test]
    fn test_ejs_unescaped_output() {
        let tokenizer = get_ejs_tokenizer();
        let template = "<p><%- rawHtml %></p>";

        let result = tokenizer.tokenize(template).expect("Tokenization failed");

        assert_eq!(result.len(), 3, "Should have opening HTML, EJS, closing HTML tokens");

        assert_eq!(result[1].token_type, "EJS");
        assert_eq!(result[1].token_sub_type.as_deref().unwrap_or(""), "Unescaped");
        assert_eq!(result[1].value, "<%- rawHtml %>");
    }

    #[test]
    fn test_complex_ejs_template() {
        let tokenizer = get_ejs_tokenizer();
        let template = r#"<!DOCTYPE html>
<html>
<head>
    <title><%= pageTitle %></title>
    <style>
        body { font-family: Arial, sans-serif; }
    </style>
    <%# This is a comment that will not be rendered %>
</head>
<body>
    <header>
        <h1><%= headerText %></h1>
    </header>
    <nav>
        <% if (showNavigation) { %>
            <ul>
                <% navigation.forEach(function(item) { %>
                    <li><a href="<%- item.url %>"><%= item.text %></a></li>
                <% }); %>
            </ul>
        <% } else { %>
            <p>Navigation is hidden</p>
        <% } %>
    </nav>
    <main><%- content %></main>
</body>
</html>"#;

        let result = tokenizer.tokenize(template).expect("Tokenization failed");

        // Just check that we have a reasonable number of tokens
        assert!(result.len() > 10, "Complex template should generate multiple tokens");

        // Count the number of different token types
        let ejs_code_count = result.iter()
            .filter(|t| t.token_type == "EJS" && t.token_sub_type.as_deref() == Some("Code"))
            .count();
        let ejs_output_count = result.iter()
            .filter(|t| t.token_type == "EJS" && t.token_sub_type.as_deref() == Some("Output"))
            .count();
        let ejs_unescaped_count = result.iter()
            .filter(|t| t.token_type == "EJS" && t.token_sub_type.as_deref() == Some("Unescaped"))
            .count();
        let ejs_comment_count = result.iter()
            .filter(|t| t.token_type == "EJS" && t.token_sub_type.as_deref() == Some("Comment"))
            .count();

        assert!(ejs_code_count >= 4, "Should have at least 4 code blocks");
        assert!(ejs_output_count >= 3, "Should have at least 3 output expressions");
        assert!(ejs_unescaped_count >= 2, "Should have at least 2 unescaped outputs");
        assert!(ejs_comment_count >= 1, "Should have at least 1 comment");
    }

    #[test]
    fn test_error_handling() {
        // Test strict mode first
        let mut strict_tokenizer = get_ejs_tokenizer();

        // Configure tokenizer using the fluent API
        strict_tokenizer
            .set_continue_on_error(false)
            .set_error_tolerance_limit(1);

        println!("Strict mode config: {:?}", strict_tokenizer.config());

        // Template with unclosed tag
        let invalid_template = "<p><%= unclosedTag </p>";

        println!("\nTesting strict mode with template: {}", invalid_template);
        let result = strict_tokenizer.tokenize(invalid_template);
        println!("Strict mode result: {:?}", result);
        assert!(result.is_err(), "Should return an error for unclosed EJS tag in strict mode");
        if let Err(err) = result {
            println!("Expected error in strict mode: {:?}", err);
        }

        // Now test tolerant mode with a more forgiving configuration
        let mut tolerant_tokenizer = get_ejs_tokenizer();

        // Configure using the more concise with_options method
        tolerant_tokenizer.with_options(
            Some(true),              // continue_on_error
            Some(true),              // tokenize_whitespace
            Some(100),               // error_tolerance_limit
            Some(true)               // track_token_positions
        );

        println!("\nTolerant mode config: {:?}", tolerant_tokenizer.config());

        // Test with a simpler template first
        let simple_template = "<div><%# unclosed";
        println!("\nTesting tolerant mode with simple template: {}", simple_template);
        let simple_result = tolerant_tokenizer.tokenize(simple_template);
        println!("Simple template result: {:?}", simple_result);

        // Verify that errors were stored in last_errors
        let simple_errors = tolerant_tokenizer.last_errors();
        assert!(simple_errors.is_some(), "Should have stored errors in tolerant mode");
        println!("Errors from simple template: {:?}", simple_errors);

        // Test with the full template that has multiple issues
        let template_with_errors = r#"<div>
        <h1><%= title</h1>  <!-- unclosed output tag -->
        <%# comment         <!-- unclosed comment -->
        <%- partial         <!-- unclosed unescaped -->
        <% if (x) {         <!-- unclosed code block -->
        <p>Some text</p>
    </div>"#;
        println!("\nTesting tolerant mode with complex template: {}", template_with_errors);

        let result = tolerant_tokenizer.tokenize(template_with_errors);
        println!("Tolerant mode result: {:?}", result);

        assert!(result.is_ok(), "Should return Ok with tokens in tolerant mode");

        // Verify that errors were stored for the complex template
        let complex_errors = tolerant_tokenizer.last_errors();
        assert!(complex_errors.is_some(), "Should have stored errors for complex template");
        if let Some(errors) = &complex_errors {
            println!("\nFound {} errors in tolerant mode:", errors.len());
            assert!(!errors.is_empty(), "Should have stored at least one error");
            for (i, err) in errors.iter().enumerate() {
                println!("Error {}: {:?}", i, err);
            }
        }

        if let Ok(tokens) = result {
            println!("\nReceived {} tokens in tolerant mode:", tokens.len());
            for (i, token) in tokens.iter().enumerate() {
                println!("Token {}: type={}, subtype={:?}, value='{}'",
                    i,
                    token.token_type,
                    token.token_sub_type,
                    token.value
                );
            }

            assert!(!tokens.is_empty(), "Should return some tokens even with errors");

            // Verify we got the initial HTML
            assert_eq!(tokens[0].token_type, "HTML");
            assert!(tokens[0].value.contains("<div>"),
                "First token should contain opening div. Got: '{}'", tokens[0].value);

            // Verify we got some of the text content
            let html_tokens: Vec<_> = tokens.iter()
                .filter(|t| t.token_type == "HTML")
                .collect();
            println!("\nFound {} HTML tokens:", html_tokens.len());
            for (i, token) in html_tokens.iter().enumerate() {
                println!("HTML token {}: '{}'", i, token.value);
            }
            assert!(html_tokens.len() >= 2, "Should have captured multiple HTML sections");

            // Check if we got the final HTML token
            if let Some(last_html) = html_tokens.last() {
                assert!(last_html.value.contains("</div>"),
                    "Last HTML token should contain closing div. Got: '{}'", last_html.value);
            }
        }
    }
}