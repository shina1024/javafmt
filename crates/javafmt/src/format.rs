mod legacy;

use crate::syntax::ParsedFile;

pub(crate) fn format(parsed: &ParsedFile<'_>) -> String {
    legacy::format(parsed)
}
