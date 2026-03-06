mod doc;
mod file;
mod layout;
mod legacy;

use crate::syntax::ParsedFile;

pub(crate) fn format(parsed: &ParsedFile<'_>) -> String {
    let doc = file::format_file(parsed);
    layout::render(&doc, 100)
}

#[cfg(test)]
mod tests {
    use super::doc::Doc;
    use super::layout;

    #[test]
    fn keeps_plain_text_doc_unchanged() {
        let doc = Doc::text("class A {}\n");
        assert_eq!(layout::render(&doc, 100), "class A {}\n");
    }

    #[test]
    fn keeps_group_flat_when_it_fits() {
        let doc = Doc::group(Doc::concat([
            Doc::text("foo("),
            Doc::indent(2, Doc::concat([Doc::soft_line(), Doc::text("bar")])),
            Doc::soft_line(),
            Doc::text(")"),
        ]));

        assert_eq!(layout::render(&doc, 32), "foo( bar )");
    }

    #[test]
    fn breaks_group_when_it_does_not_fit() {
        let doc = Doc::group(Doc::concat([
            Doc::text("foo("),
            Doc::indent(
                2,
                Doc::concat([Doc::soft_line(), Doc::text("longArgumentName")]),
            ),
            Doc::soft_line(),
            Doc::text(")"),
        ]));

        assert_eq!(layout::render(&doc, 10), "foo(\n  longArgumentName\n)");
    }

    #[test]
    fn preserves_hard_lines_inside_groups() {
        let doc = Doc::group(Doc::concat([
            Doc::text("a"),
            Doc::indent(
                2,
                Doc::concat([
                    Doc::hard_line(),
                    Doc::text("b"),
                    Doc::hard_line(),
                    Doc::text("c"),
                ]),
            ),
        ]));

        assert_eq!(layout::render(&doc, 100), "a\n  b\n  c");
    }
}
