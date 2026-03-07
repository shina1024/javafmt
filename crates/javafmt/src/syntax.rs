mod parse;
mod tree;

pub(crate) use tree::{ParsedFile, TopLevelItemKind};

pub(crate) fn parse(source: &str) -> ParsedFile<'_> {
    parse::parse_file(source)
}

#[cfg(test)]
mod tests {
    use super::{TopLevelItemKind, parse};

    #[test]
    fn outlines_package_import_and_type_items() {
        let parsed =
            parse("package p;\nimport java.util.List;\npublic final class A {}\nclass B {}\n");
        let kinds = parsed
            .outline
            .items
            .iter()
            .map(|item| item.kind)
            .collect::<Vec<_>>();

        assert_eq!(
            kinds,
            vec![
                TopLevelItemKind::Package,
                TopLevelItemKind::Import,
                TopLevelItemKind::TypeDeclaration,
                TopLevelItemKind::TypeDeclaration,
            ]
        );
    }

    #[test]
    fn outlines_open_module_as_module_declaration() {
        let parsed = parse("open module example.mod { requires java.base; }\n");
        assert_eq!(parsed.outline.items.len(), 1);
        assert_eq!(
            parsed.outline.items[0].kind,
            TopLevelItemKind::ModuleDeclaration
        );
    }
}
