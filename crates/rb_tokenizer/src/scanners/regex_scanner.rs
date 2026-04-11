use super::Scanner;
use super::scanner::ScanMatch;
use crate::tokens::{Token, TokenizationError, SourceSpan};
use super::scanner::AcceptStrategy;
use regex::Regex;
use std::borrow::Cow;

pub struct RegexScanner {
    pub pattern: Regex,
    pub token_type: &'static str,
    pub token_sub_type: Option<&'static str>,
    pub accept_strategy: Option<AcceptStrategy>,
}

impl RegexScanner {
    fn normalize_pattern(pattern: &str) -> String {
        if pattern.starts_with('^') {
            pattern.to_string()
        } else {
            eprintln!(
                "[rb_tokenizer] warning: RegexScanner pattern {:?} did not start with '^'; prepending '^' automatically. Add '^' explicitly to make the intent clear.",
                pattern
            );
            format!("^{pattern}")
        }
    }

    pub fn new(pattern: &str, token_type: &'static str, token_sub_type: Option<&'static str>) -> Result<Self, TokenizationError> {
        let normalized_pattern = Self::normalize_pattern(pattern);

        Ok(Self {
            pattern: Regex::new(&normalized_pattern).map_err(|error| TokenizationError::InvalidRegexPattern {
                pattern: normalized_pattern.clone(),
                message: error.to_string(),
            })?,
            token_type,
            token_sub_type,
            accept_strategy: None,
        })
    }

    pub fn with_accept_strategy(pattern: &str, token_type: &'static str, token_sub_type: Option<&'static str>, accept_strategy: AcceptStrategy) -> Result<Self, TokenizationError> {
        let normalized_pattern = Self::normalize_pattern(pattern);

        Ok(Self {
            pattern: Regex::new(&normalized_pattern).map_err(|error| TokenizationError::InvalidRegexPattern {
                pattern: normalized_pattern.clone(),
                message: error.to_string(),
            })?,
            token_type,
            token_sub_type,
            accept_strategy: Some(accept_strategy),
        })
    }
}

impl Scanner for RegexScanner {
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError> {
        if let Some(strategy) = &self.accept_strategy {
            if !strategy.accepts(input) {
                return Ok(None);
            }
        }
        if let Some(mat) = self.pattern.find(input) {
            if mat.start() != 0 {
                return Ok(None);
            }

            return Ok(Some(Token {
                token_type: self.token_type,
                token_sub_type: self.token_sub_type,
                value: Cow::Borrowed(mat.as_str()),
                span: SourceSpan::UNKNOWN,
            }))
        }
        Ok(None)
    }

    fn scan_with_context<'i>(&self, input: &'i str) -> Result<Option<ScanMatch<'i>>, TokenizationError> {
        if let Some(strategy) = &self.accept_strategy {
            if !strategy.accepts(input) {
                return Ok(None);
            }
        }

        if let Some(mat) = self.pattern.find(input) {
            if mat.start() != 0 {
                return Ok(None);
            }

            return Ok(Some(ScanMatch {
                consumed_len: mat.end(),
                token: Token {
                    token_type: self.token_type,
                    token_sub_type: self.token_sub_type,
                    value: Cow::Borrowed(mat.as_str()),
                    span: SourceSpan::UNKNOWN,
                },
            }));
        }

        Ok(None)
    }
}
