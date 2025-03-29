use crate::tokens::Token;
use std::fmt::Write;

/// Pretty prints a list of tokens in a human-readable format
pub fn pretty_print_tokens(tokens: &[Token]) -> String {
    let mut output = String::new();
    writeln!(&mut output, "Tokens:").unwrap();
    for (i, token) in tokens.iter().enumerate() {
        writeln!(
            &mut output,
            "> {:3}: {:15} {:20} '{}' (line {}, col {}) \n\n",
            i,
            token.token_type,
            token.token_sub_type.as_deref().unwrap_or(""),
            token.value.replace('\n', "\\n"),
            token.line,
            token.column
        )
        .unwrap();
    }
    output
}

/// Returns a compact single-line representation of a token for debug output
pub fn token_summary(token: &Token) -> String {
    format!(
        "{}:{} '{}'",
        token.token_type,
        token.token_sub_type.as_deref().unwrap_or(""),
        token.value.replace('\n', "\\n")
    )
}

/// Creates a table-like view of tokens for comparing expected vs actual results
pub fn compare_tokens(expected: &[Token], actual: &[Token]) -> String {
    let mut output = String::new();
    writeln!(&mut output, "Token Comparison:").unwrap();
    writeln!(&mut output, "{:3} {:^40} {:^40}", "", "Expected", "Actual").unwrap();
    writeln!(&mut output, "{:-<85}", "").unwrap();

    let max_len = expected.len().max(actual.len());
    for i in 0..max_len {
        let expected_str = expected.get(i).map_or("".to_string(), token_summary);
        let actual_str = actual.get(i).map_or("".to_string(), token_summary);
        let status = if i < expected.len() && i < actual.len() && expected[i] == actual[i] {
            "✓"
        } else {
            "✗"
        };
        writeln!(
            &mut output,
            "{:3} {:40} {:40}",
            status,
            expected_str,
            actual_str
        )
        .unwrap();
    }
    output
}

/// Generates a visual representation of token positions in the input text
pub fn visualize_token_positions(input: &str, tokens: &[Token]) -> String {
    let mut output = String::new();
    let mut lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
    if !input.ends_with('\n') {
        lines.push(String::new());
    }

    // Create pointer lines for each source line
    let mut pointer_lines = vec![String::new(); lines.len()];

    // Add markers for each token
    for (i, token) in tokens.iter().enumerate() {
        if token.line > 0 && token.line <= lines.len() {
            let line_idx = token.line - 1;
            let col_idx = token.column - 1;

            // Ensure we have enough space in the pointer line
            while pointer_lines[line_idx].len() < col_idx {
                pointer_lines[line_idx].push(' ');
            }

            // Add the token index marker
            pointer_lines[line_idx].push_str(&format!("^{}", i));
        }
    }

    // Combine source and pointer lines
    for (i, (line, pointers)) in lines.iter().zip(pointer_lines.iter()).enumerate() {
        writeln!(&mut output, "{:4} | {}", i + 1, line).unwrap();
        if !pointers.is_empty() {
            writeln!(&mut output, "     | {}", pointers).unwrap();
        }
    }
    output
}

/// Provides detailed token analysis including statistics and potential issues
pub fn analyze_tokens(tokens: &[Token]) -> String {
    let mut output = String::new();
    let mut type_counts = std::collections::HashMap::new();
    let mut subtype_counts = std::collections::HashMap::new();
    let mut max_line = 0;
    let mut tokens_per_line = std::collections::HashMap::new();

    for token in tokens {
        *type_counts.entry(&token.token_type).or_insert(0) += 1;
        if let Some(subtype) = &token.token_sub_type {
            *subtype_counts.entry(subtype.as_str()).or_insert(0) += 1;
        }
        max_line = max_line.max(token.line);
        *tokens_per_line.entry(token.line).or_insert(0) += 1;
    }

    writeln!(&mut output, "Token Analysis:").unwrap();
    writeln!(&mut output, "Total Tokens: {}", tokens.len()).unwrap();
    writeln!(&mut output, "Lines Covered: {}", max_line).unwrap();
    writeln!(&mut output, "\nToken Type Distribution:").unwrap();
    for (typ, count) in type_counts.iter() {
        writeln!(&mut output, "{:15}: {}", typ, count).unwrap();
    }

    if !subtype_counts.is_empty() {
        writeln!(&mut output, "\nToken Subtype Distribution:").unwrap();
        for (subtype, count) in subtype_counts.iter() {
            writeln!(&mut output, "{:15}: {}", subtype, count).unwrap();
        }
    }

    // Check for potential issues
    writeln!(&mut output, "\nPotential Issues:").unwrap();
    let mut issues = Vec::new();

    // Check for lines with unusually high token counts
    let avg_tokens_per_line: f64 = tokens.len() as f64 / max_line as f64;
    for (line, count) in tokens_per_line.iter() {
        if *count as f64 > avg_tokens_per_line * 2.0 {
            issues.push(format!(
                "Line {} has high token count: {} (avg: {:.1})",
                line, count, avg_tokens_per_line
            ));
        }
    }

    // Check for tokens with empty values
    for (i, token) in tokens.iter().enumerate() {
        if token.value.is_empty() {
            issues.push(format!("Token {} has empty value", i));
        }
    }

    if issues.is_empty() {
        writeln!(&mut output, "No issues detected").unwrap();
    } else {
        for issue in issues {
            writeln!(&mut output, "- {}", issue).unwrap();
        }
    }

    output
}