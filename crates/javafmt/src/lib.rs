mod compat;
mod format;
mod syntax;

pub mod bench_support;
mod comments;
mod lexer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatResult {
    pub output: String,
    pub changed: bool,
}

pub fn format_str(input: &str) -> FormatResult {
    let normalized = compat::normalize_input(input);
    let parsed = syntax::parse(&normalized.source);
    let emitted = format::format(&parsed);
    let output = compat::finalize_output(emitted, normalized.line_ending);
    let changed = output != input;
    FormatResult { output, changed }
}

#[cfg(test)]
mod tests {
    use super::{compat, format, format_str, syntax};
    use crate::lexer::{TokenKind, lex};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct SignificantToken {
        kind: TokenKind,
        text: String,
    }

    #[test]
    fn keeps_formatted_text_unchanged() {
        let input = "class A {}\n";
        let result = format_str(input);
        assert_eq!(result.output, input);
        assert!(!result.changed);
    }

    #[test]
    fn preserves_crlf_if_input_uses_crlf() {
        let input = "class A {}\r\n";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\r\n");
        assert!(!result.changed);
    }

    #[test]
    fn mixed_newlines_fall_back_to_lf_output() {
        let input = "class A {\r\n}\n";
        let result = format_str(input);
        assert!(!result.output.contains('\r'));
        assert!(result.output.ends_with('\n'));
        assert!(result.changed);
    }

    #[test]
    fn keeps_text_block_intact() {
        let input = "class A{String f(){return \"\"\"\nline1\nline2\n\"\"\";}}\n";
        let result = format_str(input);
        assert!(result.output.contains("\"\"\"\nline1\nline2\n\"\"\""));
    }

    #[test]
    fn appends_trailing_newline() {
        let input = "class A {}";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\n");
        assert!(result.changed);
    }

    #[test]
    fn trims_trailing_whitespace() {
        let input = "class A {}   \n";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\n");
        assert!(result.changed);
    }

    #[test]
    fn sorts_static_imports_before_normal_imports() {
        let input = "package p;\nimport java.util.List;\nimport static java.util.Collections.emptyList;\nclass A{List<String> xs=emptyList();}\n";
        let result = format_str(input);
        assert!(
            result.output.contains(
                "import static java.util.Collections.emptyList;\n\nimport java.util.List;"
            )
        );
    }

    #[test]
    fn sorts_imports_lexicographically_within_groups() {
        let input = "package p;\nimport java.util.List;\nimport java.util.Date;\nimport static java.util.Collections.singletonList;\nimport static java.util.Collections.emptyList;\nclass A{List<Date> a=emptyList();List<Date> b=singletonList(new Date());}\n";
        let result = format_str(input);
        assert!(result.output.contains("import static java.util.Collections.emptyList;\nimport static java.util.Collections.singletonList;"));
        assert!(
            result
                .output
                .contains("import java.util.Date;\nimport java.util.List;")
        );
    }

    #[test]
    fn keeps_import_order_when_comments_are_in_import_block() {
        let input = "package p;\nimport java.util.List;\n// c\nimport static java.util.Collections.emptyList;\nclass A{List<String> xs=emptyList();}\n";
        let result = format_str(input);
        assert!(
            result
                .output
                .contains("import java.util.List;\n// c\nimport static")
        );
    }

    #[test]
    fn import_reorder_skips_non_package_prefix_content() {
        let input = "class A {}\nimport java.util.List;\n";
        assert_eq!(compat::reorder_top_level_imports(input.to_owned()), input);
    }

    #[test]
    fn import_reorder_preserves_comment_adjacent_suffix() {
        let input = "package p;\nimport java.util.List;\n\n// keep with the type\nclass A {}\n";
        assert_eq!(compat::reorder_top_level_imports(input.to_owned()), input);
    }

    #[test]
    fn import_reorder_rebuilds_prefix_groups_and_suffix() {
        let input = "package p;\n\nimport java.util.List;\nimport static java.util.Collections.emptyList;\n\n\nclass A {}\n";
        let expected = "package p;\n\nimport static java.util.Collections.emptyList;\n\nimport java.util.List;\n\nclass A {}\n";
        assert_eq!(
            compat::reorder_top_level_imports(input.to_owned()),
            expected
        );
    }

    #[test]
    fn formats_package_import_only_file_through_structured_path() {
        let input = "package p ;\nimport b.B ;\nimport a.A ;\n";
        let expected = "package p;\n\nimport a.A;\nimport b.B;\n";
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_optional_chain_wrapping_case() {
        let input = include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B124394008.input");
        let expected =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B124394008.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_then_return_array_wrapping_case() {
        let input = include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B20531711.input");
        let expected =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B20531711.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn breaks_long_field_chain_assignment_like_gjf() {
        let input = "class A{void f(){this.overflowContactCompositeSupportedRenderers=this.getSharePanelResponse.contents.unifiedSharePanelRenderer.contents[0].connectionSection.connectionsOverflowMenu.connectionsOverflowMenuRenderer.contents[0].overflowConnectionSectionRenderer.contacts[0];}}";
        let expected = "class A {\n  void f() {\n    this.overflowContactCompositeSupportedRenderers =\n        this.getSharePanelResponse\n            .contents\n            .unifiedSharePanelRenderer\n            .contents[0]\n            .connectionSection\n            .connectionsOverflowMenu\n            .connectionsOverflowMenuRenderer\n            .contents[0]\n            .overflowConnectionSectionRenderer\n            .contacts[0];\n  }\n}\n";
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_complex_ternary_wrapping_case() {
        let input = include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B24202287.input");
        let expected =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B24202287.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_statement_chain_wrapping_case() {
        let input = include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B126411718.input");
        let expected =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B126411718.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_single_nested_call_argument_case() {
        let input = include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B154342628.input");
        let expected =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/B154342628.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_javadoc_paragraph_reflow_case() {
        let input =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testjavadoc/B28750242.input");
        let expected =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testjavadoc/B28750242.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_javadoc_list_spacing_case() {
        let input =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testjavadoc/B31404367.input");
        let expected =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testjavadoc/B31404367.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_sealed_modifier_order_case() {
        let input = include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/Sealed.input");
        let expected = include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/Sealed.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_multivariable_declaration_case() {
        let input =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/Multivariables.input");
        let expected =
            include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/Multivariables.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn matches_upstream_field_javadoc_spacing_case() {
        let input = include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/Fields.input");
        let expected = include_str!("../../../fixtures/upstream-gjf/1.34.1/testdata/Fields.output");
        assert_eq!(format_str(input).output, expected);
    }

    #[test]
    fn internal_pipeline_matches_public_api() {
        let input = "class A{int f(){return 1;}}\n";
        let normalized = compat::normalize_input(input);
        let parsed = syntax::parse(&normalized.source);
        let output = compat::finalize_output(format::format(&parsed), normalized.line_ending);
        assert_eq!(output, format_str(input).output);
    }

    #[test]
    fn formatting_preserves_non_whitespace_tokens_for_representative_inputs() {
        let cases = [
            "class A{}",
            "class A{/*keep*/int x=1+2;}",
            "class A{void f(){if(a){b();}else{c();}}}",
            "class A{String s=\"//not-comment\";char c='x';}",
            "class A{String s=\"\"\"\nline1\nline2\n\"\"\";}",
            "@Anno(value={\"a\",\"b\"}) class A{}",
            "class A{void f(){Runnable r=()->{work();};}}",
            "class A{void f(){var x=Foo.<Bar>baz(1,2);}}",
        ];

        for input in cases {
            let formatted = format_str(input).output;
            assert_eq!(
                significant_tokens(input),
                significant_tokens(&formatted),
                "non-whitespace tokens changed for input:\n{input}\nformatted as:\n{formatted}"
            );
        }
    }

    fn significant_tokens(source: &str) -> Vec<SignificantToken> {
        let lexed = lex(source);
        lexed
            .tokens
            .iter()
            .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
            .map(|token| SignificantToken {
                kind: token.kind,
                text: source[token.start..token.end].to_owned(),
            })
            .collect::<Vec<_>>()
    }
}
