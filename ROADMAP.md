# rb-tokenizer Development Roadmap

This document outlines the planned enhancements and features for the rb-tokenizer project.

## Short-term Goals (0-3 months)

### Scanner Enhancements
- [ ] **Block Scanner**: Add support for recognizing and tokenizing block structures (e.g., code blocks with start/end delimiters)
  - Can serve as a foundation for block comments and other block-based structures
  - Handles matching start/end tokens with proper nesting support
  - Provides raw mode option to preserve all content exactly as-is (no escape sequence processing)
- [ ] **Comment Scanner**: Specialized scanner for handling line comments and block comments
  - Could leverage Block Scanner internally for block comments
  - Handles special semantics like comment continuation in multi-line scenarios
- [ ] **Indentation Scanner**: Scanner to handle significant whitespace at the lexical level (for Python, YAML, etc.)
  - Maintains state between scans (indentation stack)
  - Emits synthetic INDENT/DEDENT tokens that the parser can consume
  - Integrated at the tokenizer level to keep the parser context-free
- [ ] **Delimiter-to-Newline Scanner**: Scanner for handling statements that start with a delimiter and end with a newline
  - Useful for directive-style syntax like preprocessor commands or line comments
  - Handles whitespace and other characters within the statement without requiring multiple scanners
- [ ] **Escape Sequence Scanner**: Enhanced handling of escape sequences in strings and other contexts

### Core Improvements
- [ ] **Token Identifier Registry**: Create a registry system for common token pattern identifiers
  - Register predicates (closures/functions) for identifying token types (whitespace, newlines, etc.)
  - Support regex-based and character-based pattern matchers
  - Allow scanners to reference these identifiers for consistent token recognition
  - Provide defaults for common languages but allow customization
- [ ] **Post-Tokenization Callbacks**: Add support for callbacks after token identification
  - Allow for token transformation, filtering, or replacement
  - Enable parallel parsing of tokens as they're identified
  - Support custom validation and enrichment of tokens
  - Provide hooks for syntax highlighting or other real-time processing
- [ ] **Token Metadata**: Add support for token metadata to store additional information for use by parsers
- [ ] **Token Position Enhancement**: Add end position (line/column) to tokens to support better error reporting
- [ ] **Error Recovery**: Improve error recovery mechanisms for more robust tokenization
- [ ] **Enhanced Error Types**: Expand the TokenizationError enum with more specific error types
  - Add specific error variants for common tokenization issues
  - Improve error messages with more context and suggestions
- [ ] **Performance Optimizations**: Profile and optimize the tokenization process for large input files

## Medium-term Goals (3-6 months)

### Advanced Features
- [ ] **Multi-mode Tokenization**: Support for switching scanning modes within a single tokenization pass (e.g., for templating languages)
- [ ] **Token Streaming**: Implement a streaming tokenization API for processing large files efficiently
  - Emit tokens as they are identified via callback mechanism
  - Support early termination of tokenization process
  - Enable processing of very large files with minimal memory footprint
- [ ] **Incremental Tokenization**: Support for tokenizing changes only instead of the entire input
- [ ] **Token Taxonomy**: Define standard token types and categories for common languages
- [ ] **Scanner Libraries**: Predefined scanner collections for common language patterns
  - Standard scanners for programming language constructs (loops, conditionals, etc.)
  - Common pattern libraries that can be composed for specific languages

### Language Support
- [ ] **JSON/YAML**: Built-in support for common data interchange formats
- [ ] **SQL**: Scanner configurations for SQL syntax
- [ ] **JavaScript/TypeScript**: Scanner configurations for JavaScript and TypeScript
- [ ] **Rust**: Support for Rust's syntax features

## Long-term Goals (6+ months)

### Architecture Evolution
- [ ] **Plugin System**: Architecture to allow extension with external scanners
- [ ] **Zero-copy Tokenization**: Optimize to avoid unnecessary string allocations
- [ ] **Language Server Integration**: Support integration with language servers via LSP
- [ ] **Code Completion Support**: Add features to support code completion and syntax highlighting
- [ ] **Versioned Token Format**: Stable serialization format for tokens to support cross-version compatibility
  - Define a versioning scheme for token structures
  - Support forward and backward compatibility

### More Scanner Types
- [ ] **Context-sensitive Scanner**: Scanners that change behavior based on context
- [ ] **Delimiter-Pair Scanner**: Specialized handling for paired delimiters with potential nesting
- [ ] **Unicode Scanner**: Better handling of Unicode tokens, including emoji and non-Latin script identifiers
- [ ] **Interpolation Scanner**: Support for string interpolation across various language syntaxes

## Maintenance and Documentation
- [ ] **API Documentation**: Complete API documentation with examples
- [ ] **Language Examples**: Example configurations for common programming languages
- [ ] **Performance Benchmarks**: Establish benchmarks for tokenization performance
  - Compare with other popular tokenizers
  - Track performance over releases to prevent regressions
- [ ] **Compliance Testing**: Develop test suites for standard language compliance
- [ ] **Test Coverage**: Increase test coverage to at least 90%
- [ ] **API Stability Guidelines**: Define which parts of the API are stable vs. experimental
  - Create guidelines for breaking vs. non-breaking changes
  - Establish deprecation policy

## Community Engagement
- [ ] **Contribution Guidelines**: Enhance contribution guidelines and templates
- [ ] **RFC Process**: Establish a process for discussing and accepting feature proposals
- [ ] **Regular Releases**: Establish a regular release schedule
- [ ] **Example Projects**: Develop sample projects showcasing real-world applications
- [ ] **Integration Guides**: Documentation for integrating with popular frameworks and tools

## Notes on Proposed Scanner Types

### Proposed New Scanner Types
1. **Block Scanner**
   - Purpose: Recognize multi-line structured blocks with start/end markers
   - Example: `{...}`, `begin...end`, `if...endif`
   - Can be specialized for different use cases (code blocks, comments, string literals)
   - Supports raw mode where content is preserved exactly as written (no escape processing)
   - Raw mode is essential for heredocs, raw strings, and code blocks in markdown
   
2. **Comment Scanner**
   - Purpose: Specialized handling of comments with awareness of different comment styles
   - Example: `//`, `/* */`, `#`, `--`
   - Could be implemented using Block Scanner for block comments and simpler scanners for line comments
   
3. **Indentation Scanner**
   - Purpose: Handle significant whitespace in languages like Python or YAML
   - Tracks indentation levels and emits indent/dedent tokens
   - Works at the tokenizer level (not parser level) to maintain a cleaner separation of concerns
   - Provides synthetic tokens that make parsing indentation-sensitive languages more straightforward
   
4. **Delimiter-to-Newline Scanner**
   - Purpose: Recognize statements that start with a specific delimiter and continue until a newline
   - Example: `#include <file>`, `//comment text`, `#define MACRO value`, shell commands
   - Captures everything between the delimiter and newline as a single token
   - Useful for preprocessor directives, line comments, and similar line-based constructs

5. **Context-sensitive Scanner**
   - Purpose: Change behavior based on context or state
   - Example: Special handling of tokens inside JSX/HTML tags vs. outside

6. **Interpolation Scanner**
   - Purpose: Handle string interpolation in various language syntaxes
   - Example: `"Hello ${name}"`, `"Hello #{name}"`

7. **Escape Sequence Scanner**
   - Purpose: Specialized handling of escape sequences
   - Example: `\n`, `\t`, `\u1234`, `\x41`

### Architecture Considerations

1. **Scanner State Management**
   - While most scanners are stateless, certain scanners like the Indentation Scanner require state
   - Need to design a state management approach that fits within the existing scanner architecture

2. **Scanner Composition**
   - Block Scanner could be a foundation for other scanners (comments, string literals, etc.)
   - Consider a composition approach where complex scanners can utilize simpler ones

3. **Token Identifier System**
   - Create a flexible token identifier registry to standardize character/pattern recognition
   - Examples: whitespace detection, newline recognition, identifier patterns, escape sequences
   - Benefits:
     - Consistency across scanners
     - Centralized language-specific configurations 
     - Easy customization for different language dialects
   - Usage pattern:
     ```rust
     // Register common token identifiers
     tokenizer.register_identifier("whitespace", Box::new(|c| c.is_whitespace()));
     tokenizer.register_regex_identifier("number", r"^\d+(\.\d+)?([eE][-+]?\d+)?");
     
     // Scanners can use these identifiers
     let whitespace_fn = tokenizer.get_identifier("whitespace");
     if whitespace_fn(next_char) { /* handle whitespace */ }
     ```

4. **Post-Processing and Callback System**
   - Provide hooks at various stages of the tokenization process
   - Allow for token transformation and enrichment
   - Enable custom processing or validation logic
   - Support parallel parsing as tokens are identified
   - Usage pattern:
     ```rust
     // Register a post-tokenization callback
     tokenizer.add_token_callback(|token| {
       // Transform the token or trigger parsing
       if token.token_type == "Identifier" && token.value == "async" {
         // Convert identifier to keyword
         token.token_type = "Keyword";
         token.token_sub_type = Some("AsyncKeyword".to_string());
       }
       // Return the modified token
       token
     });

     // Or for streaming parser integration
     tokenizer.add_token_callback(move |token| {
       parser_channel.send(token.clone());
       token
     });
     ```

### Versioning and Compatibility

1. **Semantic Versioning**
   - Follow semantic versioning principles
   - Major version changes for breaking API changes
   - Minor version changes for backward-compatible features
   - Patch version for bug fixes and non-breaking improvements

2. **Migration Paths**
   - Provide migration guides for major version changes
   - Include deprecation warnings before removing features
   - Maintain compatibility layers where possible

All of these scanner types would integrate into the existing scanner framework and adhere to the `Scanner` trait.