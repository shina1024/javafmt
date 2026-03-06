use super::doc::{Doc, LineMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutMode {
    Flat,
    Break,
}

#[derive(Clone, Copy)]
struct Command<'a> {
    indent: usize,
    mode: LayoutMode,
    doc: &'a Doc,
}

pub(crate) fn render(doc: &Doc, line_width: usize) -> String {
    let mut out = String::new();
    let mut column = 0usize;
    let mut stack = vec![Command {
        indent: 0,
        mode: LayoutMode::Break,
        doc,
    }];

    while let Some(Command { indent, mode, doc }) = stack.pop() {
        match doc {
            Doc::Nil => {}
            Doc::Text(text) => {
                out.push_str(text);
                column += text.chars().count();
            }
            Doc::Line(LineMode::Hard) => {
                out.push('\n');
                push_indent(&mut out, indent);
                column = indent;
            }
            Doc::Line(LineMode::Soft) => match mode {
                LayoutMode::Flat => {
                    out.push(' ');
                    column += 1;
                }
                LayoutMode::Break => {
                    out.push('\n');
                    push_indent(&mut out, indent);
                    column = indent;
                }
            },
            Doc::Concat(parts) => {
                for part in parts.iter().rev() {
                    stack.push(Command {
                        indent,
                        mode,
                        doc: part,
                    });
                }
            }
            Doc::Indent(extra, doc) => {
                stack.push(Command {
                    indent: indent + extra,
                    mode,
                    doc,
                });
            }
            Doc::Group(doc) => {
                let next_mode = if fits(
                    line_width.saturating_sub(column),
                    Command {
                        indent,
                        mode: LayoutMode::Flat,
                        doc,
                    },
                    &stack,
                ) {
                    LayoutMode::Flat
                } else {
                    LayoutMode::Break
                };
                stack.push(Command {
                    indent,
                    mode: next_mode,
                    doc,
                });
            }
        }
    }

    out
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

fn fits(mut remaining: usize, first: Command<'_>, rest: &[Command<'_>]) -> bool {
    let mut stack = Vec::with_capacity(rest.len() + 1);
    stack.extend(rest.iter().copied());
    stack.push(first);

    while let Some(Command { mode, doc, .. }) = stack.pop() {
        match doc {
            Doc::Nil => {}
            Doc::Text(text) => {
                let width = text.chars().count();
                if width > remaining {
                    return false;
                }
                remaining -= width;
            }
            Doc::Line(LineMode::Hard) => return true,
            Doc::Line(LineMode::Soft) => match mode {
                LayoutMode::Flat => {
                    if remaining == 0 {
                        return false;
                    }
                    remaining -= 1;
                }
                LayoutMode::Break => return true,
            },
            Doc::Concat(parts) => {
                for part in parts.iter().rev() {
                    stack.push(Command {
                        indent: 0,
                        mode,
                        doc: part,
                    });
                }
            }
            Doc::Indent(_, doc) => {
                stack.push(Command {
                    indent: 0,
                    mode,
                    doc,
                });
            }
            Doc::Group(doc) => {
                stack.push(Command {
                    indent: 0,
                    mode: LayoutMode::Flat,
                    doc,
                });
            }
        }
    }

    true
}
