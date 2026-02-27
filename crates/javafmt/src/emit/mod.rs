use crate::printer::PrintedDoc;

pub fn emit(doc: PrintedDoc<'_>) -> String {
    if doc.text.is_empty() {
        return String::new();
    }

    if doc.text.ends_with('\n') {
        return doc.text.to_owned();
    }

    let mut out = String::with_capacity(doc.text.len() + 1);
    out.push_str(doc.text);
    out.push('\n');
    out
}
