mod legacy;
mod tree;

pub(crate) use tree::ParsedFile;

pub(crate) fn parse(source: &str) -> ParsedFile<'_> {
    legacy::parse(source)
}
