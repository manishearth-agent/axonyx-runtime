#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxFormatOptions {
    pub indent_width: usize,
}

impl Default for AxFormatOptions {
    fn default() -> Self {
        Self { indent_width: 2 }
    }
}

/// Formats Axonyx source without rewriting expressions, strings, or embedded code.
///
/// V0 deliberately owns whitespace only. Syntax-aware rewrites belong to later
/// formatter versions once every language surface has a lossless syntax tree.
pub fn format_ax_source(source: &str) -> String {
    format_ax_source_with_options(source, AxFormatOptions::default())
}

pub fn format_ax_source_with_options(source: &str, options: AxFormatOptions) -> String {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let indent_unit = " ".repeat(options.indent_width);
    let mut formatted = Vec::new();
    let mut depth = 0usize;
    let mut previous_blank = false;
    let mut scanner = StructuralScanner::default();
    let mut raw_block_depth: Option<usize> = None;

    for raw_line in normalized.lines() {
        if let Some(current_raw_depth) = raw_block_depth {
            let line = raw_line.trim();
            let balance = scanner.scan_line(line);
            let next_raw_depth = current_raw_depth
                .saturating_add(balance.opens)
                .saturating_sub(balance.closes);
            let next_depth = depth
                .saturating_add(balance.opens)
                .saturating_sub(balance.closes);

            if next_raw_depth == 0 {
                let alignment = leading_alignment(line, scanner.pending_tag);
                let line_depth = depth.saturating_sub(alignment);
                formatted.push(format!("{}{}", indent_unit.repeat(line_depth), line));
                previous_blank = false;
                raw_block_depth = None;
            } else {
                formatted.push(raw_line.to_string());
                previous_blank = line.is_empty();
                raw_block_depth = Some(next_raw_depth);
            }
            depth = next_depth;
            continue;
        }

        let line = raw_line.trim();
        if line.is_empty() {
            if !previous_blank && !formatted.is_empty() {
                formatted.push(String::new());
                previous_blank = true;
            }
            continue;
        }

        let alignment = leading_alignment(line, scanner.pending_tag);
        let line_depth = depth.saturating_sub(alignment);
        let expanded = if !scanner.pending_tag && scanner.quote.is_none() && !scanner.block_comment
        {
            expand_compact_markup(line, &indent_unit)
        } else {
            None
        };
        for part in expanded.as_deref().unwrap_or(&[line.to_string()]) {
            formatted.push(format!("{}{}", indent_unit.repeat(line_depth), part));
        }
        previous_blank = false;

        let balance = scanner.scan_line(line);
        if is_raw_block_start(line) && balance.opens > balance.closes {
            raw_block_depth = Some(balance.opens - balance.closes);
        }
        depth = depth
            .saturating_add(balance.opens)
            .saturating_sub(balance.closes);
    }

    while formatted.last().is_some_and(String::is_empty) {
        formatted.pop();
    }

    if formatted.is_empty() {
        String::new()
    } else {
        format!("{}\n", formatted.join("\n"))
    }
}

fn is_raw_block_start(line: &str) -> bool {
    ["client JS", "client WASM", "style"].iter().any(|keyword| {
        line.strip_prefix(keyword)
            .is_some_and(|rest| rest.trim_start().starts_with('{'))
    })
}

struct InlineMarkup<'a> {
    source: &'a str,
    open: &'a str,
    close: &'a str,
    children: Vec<InlineMarkup<'a>>,
    mixed: bool,
}

// Only expand complete element-only containers. Leaf text and mixed prose keep
// their exact source bytes; the parser equality check is the final safety gate.
fn expand_compact_markup(line: &str, indent: &str) -> Option<Vec<String>> {
    let (node, consumed) = inline_markup(line, 0)?;
    if consumed != line.len() || node.mixed || node.children.is_empty() {
        return None;
    }
    fn emit(node: &InlineMarkup<'_>, prefix: &str, indent: &str, lines: &mut Vec<String>) {
        if node.mixed || node.children.is_empty() {
            lines.push(format!("{prefix}{}", node.source));
        } else {
            lines.push(format!("{prefix}{}", node.open));
            for child in &node.children {
                emit(child, &format!("{prefix}{indent}"), indent, lines);
            }
            lines.push(format!("{prefix}{}", node.close));
        }
    }
    let mut lines = Vec::new();
    emit(&node, "", indent, &mut lines);
    let wrap = |markup: &str| format!("page FormatProbe() {{\nreturn ASX {{\n{markup}\n}}\n}}");
    let before = crate::ax_parser_v2::parse_ax_v2(&wrap(line)).ok()?;
    let after = crate::ax_parser_v2::parse_ax_v2(&wrap(&lines.join("\n"))).ok()?;
    (before == after).then_some(lines)
}

fn inline_markup(source: &str, depth: usize) -> Option<(InlineMarkup<'_>, usize)> {
    if depth > 128 || !source.starts_with('<') || source.starts_with("</") {
        return None;
    }
    let open_end = inline_tag_end(source)?;
    let open = &source[..open_end];
    let name = open[1..].split([' ', '\t', '/', '>']).next()?;
    if !name.is_empty()
        && !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return None;
    }
    if open.ends_with("/>") {
        return Some((
            InlineMarkup {
                source: open,
                open,
                close: "",
                children: Vec::new(),
                mixed: false,
            },
            open_end,
        ));
    }
    let closing = format!("</{name}>");
    let mut cursor = open_end;
    let mut children = Vec::new();
    let mut mixed = matches!(name, "pre" | "code" | "script" | "style" | "textarea");
    loop {
        let remaining = source.get(cursor..)?;
        if remaining.starts_with(&closing) {
            let end = cursor + closing.len();
            return Some((
                InlineMarkup {
                    source: &source[..end],
                    open,
                    close: &source[cursor..end],
                    children,
                    mixed,
                },
                end,
            ));
        }
        if remaining.starts_with('<') {
            let (child, length) = inline_markup(remaining, depth + 1)?;
            children.push(child);
            cursor += length;
        } else {
            // Expressions may contain strings that look like markup.
            let length = if remaining.starts_with('{') {
                inline_expression_end(remaining)?
            } else {
                remaining.chars().next()?.len_utf8()
            };
            mixed = true;
            cursor += length;
        }
    }
}

fn inline_expression_end(source: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in source.char_indices() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
        } else if matches!(ch, '\'' | '"' | '`') {
            quote = Some(ch);
        } else if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index + 1);
            }
        }
    }
    None
}

fn inline_tag_end(source: &str) -> Option<usize> {
    let mut cursor = 1;
    let mut quote = None;
    let mut escaped = false;
    while let Some(ch) = source.get(cursor..)?.chars().next() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch == '{' {
            cursor += inline_expression_end(&source[cursor..])?;
            continue;
        } else if ch == '>' {
            return Some(cursor + 1);
        }
        cursor += ch.len_utf8();
    }
    None
}

fn leading_alignment(line: &str, pending_tag: bool) -> usize {
    let mut rest = line.trim_start();
    let mut closes = 0;

    loop {
        if rest.starts_with("</") {
            closes += 1;
            rest = rest
                .find('>')
                .map(|index| &rest[index + 1..])
                .unwrap_or_default()
                .trim_start();
            continue;
        }
        if let Some(first) = rest
            .chars()
            .next()
            .filter(|ch| matches!(ch, '}' | ']' | ')'))
        {
            closes += 1;
            rest = rest[first.len_utf8()..].trim_start();
            continue;
        }
        break;
    }

    if closes == 0 && pending_tag && (rest.starts_with('>') || rest.starts_with("/>")) {
        1
    } else {
        closes
    }
}

#[derive(Debug, Default)]
struct StructuralScanner {
    pending_tag: bool,
    block_comment: bool,
    quote: Option<char>,
    escaped: bool,
}

#[derive(Debug, Default)]
struct StructuralBalance {
    opens: usize,
    closes: usize,
}

impl StructuralScanner {
    fn scan_line(&mut self, line: &str) -> StructuralBalance {
        let chars = line.char_indices().collect::<Vec<_>>();
        let mut balance = StructuralBalance::default();
        let mut index = 0;

        while index < chars.len() {
            let (byte_index, ch) = chars[index];
            let next = chars.get(index + 1).map(|(_, ch)| *ch);

            if self.block_comment {
                if ch == '*' && next == Some('/') {
                    self.block_comment = false;
                    index += 2;
                } else {
                    index += 1;
                }
                continue;
            }

            if let Some(quote) = self.quote {
                if self.escaped {
                    self.escaped = false;
                } else if ch == '\\' {
                    self.escaped = true;
                } else if ch == quote {
                    self.quote = None;
                }
                index += 1;
                continue;
            }

            if ch == '/' && next == Some('/') {
                break;
            }
            if ch == '/' && next == Some('*') {
                self.block_comment = true;
                index += 2;
                continue;
            }
            if matches!(ch, '"' | '\'' | '`') {
                self.quote = Some(ch);
                index += 1;
                continue;
            }

            if self.pending_tag {
                if ch == '/' && next == Some('>') {
                    balance.closes += 1;
                    self.pending_tag = false;
                    index += 2;
                    continue;
                }
                if ch == '>' {
                    self.pending_tag = false;
                }
                index += 1;
                continue;
            }

            match ch {
                '{' | '[' | '(' => balance.opens += 1,
                '}' | ']' | ')' => balance.closes += 1,
                '<' => {
                    let tail = &line[byte_index..];
                    if tail.starts_with("</") {
                        balance.closes += 1;
                    } else if tail.starts_with("<>") {
                        balance.opens += 1;
                    } else if next.is_some_and(|next| next.is_ascii_alphabetic())
                        && tag_can_start_at(line, byte_index)
                    {
                        match tag_end_kind(tail) {
                            TagEnd::SelfClosing => {}
                            TagEnd::Open => balance.opens += 1,
                            TagEnd::Pending => {
                                balance.opens += 1;
                                self.pending_tag = true;
                            }
                        }
                    }
                }
                _ => {}
            }

            index += 1;
        }

        balance
    }
}

fn tag_can_start_at(line: &str, byte_index: usize) -> bool {
    line[..byte_index]
        .chars()
        .next_back()
        .is_none_or(|previous| {
            previous.is_whitespace() || matches!(previous, '{' | '(' | '[' | '>' | ',' | ':')
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagEnd {
    SelfClosing,
    Open,
    Pending,
}

fn tag_end_kind(source: &str) -> TagEnd {
    let mut quote = None;
    let mut escaped = false;
    let mut previous = None;

    for ch in source.chars() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            previous = Some(ch);
            continue;
        }

        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
        } else if ch == '>' {
            return if previous == Some('/') {
                TagEnd::SelfClosing
            } else {
                TagEnd::Open
            };
        }
        previous = Some(ch);
    }

    TagEnd::Pending
}

pub mod prelude {
    pub use super::{format_ax_source, format_ax_source_with_options, AxFormatOptions};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ax_backend_parser::parse_backend_ax;
    use crate::ax_parser_v2::parse_ax_v2;

    #[test]
    fn formats_function_shaped_asx_and_is_idempotent() {
        let source = "page Home() {\nreturn ASX {\n<Container max=\"xl\">\n<Copy>{title}</Copy>\n</Container>\n}\n}\n";
        let expected = "page Home() {\n  return ASX {\n    <Container max=\"xl\">\n      <Copy>{title}</Copy>\n    </Container>\n  }\n}\n";

        let formatted = format_ax_source(source);
        assert_eq!(formatted, expected);
        assert_eq!(format_ax_source(&formatted), expected);
    }

    #[test]
    fn expands_nested_table_rows_without_changing_the_ast() {
        let source = "page Posts() {\nreturn ASX {\n<TableRow><TableCell><a href=\"/post\">Post</a></TableCell><TableCell>{post.status}</TableCell></TableRow>\n}\n}\n";
        let expected = "page Posts() {\n  return ASX {\n    <TableRow>\n      <TableCell>\n        <a href=\"/post\">Post</a>\n      </TableCell>\n      <TableCell>{post.status}</TableCell>\n    </TableRow>\n  }\n}\n";
        let formatted = format_ax_source(source);
        assert_eq!(formatted, expected);
        assert_eq!(format_ax_source(&formatted), formatted);
        assert_eq!(
            parse_ax_v2(source).unwrap(),
            parse_ax_v2(&formatted).unwrap()
        );
    }

    #[test]
    fn compact_markup_preserves_mixed_text_and_sensitive_content() {
        for markup in [
            "<Copy>Hello <strong>world</strong>!</Copy>",
            "<pre><span> keep  spaces </span></pre>",
            "<div>{\"<Card><Copy>fake</Copy></Card>\"}<Slot /></div>",
            "<div><span>A</span> <span>B</span></div>",
        ] {
            assert!(expand_compact_markup(markup, "  ").is_none(), "{markup}");
        }
    }

    #[test]
    fn expands_fragments_and_ignores_angle_brackets_in_attributes() {
        let markup = "<><Card title=\"a > b\"><Copy>{value}</Copy></Card><Slot /></>";
        let lines = expand_compact_markup(markup, "  ").unwrap();
        assert_eq!(
            lines,
            [
                "<>",
                "  <Card title=\"a > b\">",
                "    <Copy>{value}</Copy>",
                "  </Card>",
                "  <Slot />",
                "</>"
            ]
        );
        let expression = "<Card when={count > 0}><Copy>Ready</Copy></Card>";
        assert!(expand_compact_markup(expression, "  ").is_some());
    }

    #[test]
    fn compact_markup_leaves_partial_or_invalid_lines_unchanged() {
        for markup in [
            "<Card><Copy>Hi</Copy>",
            "<Card><Copy /></Wrong>",
            "const x = \"<Card><Copy /></Card>\"",
            "<Card><Copy /></Card> // comment",
        ] {
            assert!(expand_compact_markup(markup, "  ").is_none());
        }
    }

    #[test]
    fn formats_multiline_elements_and_self_closing_tags() {
        let source = "page Home() {\nreturn ASX {\n<ComponentPage\ntitle=\"Button\"\ndescription=\"A {safe} value\"\n>\n<Meta\nname=\"description\"\ncontent=\"Foundry\"\n/>\n<Copy>Ready</Copy>\n</ComponentPage>\n}\n}\n";
        let expected = "page Home() {\n  return ASX {\n    <ComponentPage\n      title=\"Button\"\n      description=\"A {safe} value\"\n    >\n      <Meta\n        name=\"description\"\n        content=\"Foundry\"\n      />\n      <Copy>Ready</Copy>\n    </ComponentPage>\n  }\n}\n";

        assert_eq!(format_ax_source(source), expected);
    }

    #[test]
    fn formats_multiline_component_signatures_and_remains_parseable() {
        let source = "page Home() {\ncomponent Button(\nlabel: String,\nvariant: \"primary\" | \"ghost\" = \"primary\",\ndisabled: Bool = false,\n) {\nrender ASX { <button>{label}</button> }\n}\nreturn ASX { <Button label=\"Ship\" /> }\n}\n";
        let expected = "page Home() {\n  component Button(\n    label: String,\n    variant: \"primary\" | \"ghost\" = \"primary\",\n    disabled: Bool = false,\n  ) {\n    render ASX { <button>{label}</button> }\n  }\n  return ASX { <Button label=\"Ship\" /> }\n}\n";

        let formatted = format_ax_source(source);
        assert_eq!(formatted, expected);
        assert_eq!(format_ax_source(&formatted), expected);
        parse_ax_v2(&formatted).expect("formatted multiline signature should parse");
    }

    #[test]
    fn formats_component_return_asx_and_remains_parseable() {
        let source = "page Home() {\ncomponent Greeting(title = \"Hello\") {\nreturn ASX {\n<div>{title}<Slot /></div>\n}\n}\nreturn ASX { <Greeting /> }\n}\n";
        let expected = "page Home() {\n  component Greeting(title = \"Hello\") {\n    return ASX {\n      <div>{title}<Slot /></div>\n    }\n  }\n  return ASX { <Greeting /> }\n}\n";

        let formatted = format_ax_source(source);
        assert_eq!(formatted, expected);
        assert_eq!(format_ax_source(&formatted), expected);
        parse_ax_v2(&formatted).expect("formatted component return should parse");
    }

    #[test]
    fn keeps_else_blocks_at_the_parent_depth() {
        let source =
            "fn label(value: Bool) {\nif value {\nreturn \"on\"\n} else {\nreturn \"off\"\n}\n}\n";
        let expected = "fn label(value: Bool) {\n  if value {\n    return \"on\"\n  } else {\n    return \"off\"\n  }\n}\n";

        assert_eq!(format_ax_source(source), expected);
    }

    #[test]
    fn normalizes_line_endings_and_blank_lines() {
        let source = "page Home() {\r\n\r\n\r\nreturn ASX { <Copy>Ready</Copy> }\r\n}\r\n\r\n";
        let expected = "page Home() {\n\n  return ASX { <Copy>Ready</Copy> }\n}\n";

        assert_eq!(format_ax_source(source), expected);
    }

    #[test]
    fn ignores_structural_characters_inside_strings_and_comments() {
        let source = "page Home() {\nconst sample = \"{ [ ( <Fake>\"\n// } </Fake>\nreturn ASX {\n<Copy>{sample}</Copy>\n}\n}\n";
        let expected = "page Home() {\n  const sample = \"{ [ ( <Fake>\"\n  // } </Fake>\n  return ASX {\n    <Copy>{sample}</Copy>\n  }\n}\n";

        assert_eq!(format_ax_source(source), expected);
    }

    #[test]
    fn formatted_frontend_and_backend_sources_still_parse() {
        let frontend = "page Home() {\nreturn ASX {\n<><Copy>{title}</Copy><Meta name=\"description\" content=\"Ready\" /></>\n}\n}\n";
        let backend = "export query loadPosts() -> Post[] {\ndata posts = db.posts.all().where({status: \"published\"})\nreturn posts\n}\n";

        parse_ax_v2(&format_ax_source(frontend)).expect("formatted ASX should parse");
        parse_backend_ax(&format_ax_source(backend))
            .expect("formatted backend source should parse");
    }

    #[test]
    fn preserves_embedded_client_and_style_block_contents() {
        let source = "component Probe() {\nclient JS {\n    const template = `first\n  second`;  \n    if (ready) {\n      mount();\n    }\n}\nstyle {\n  .probe { white-space: pre; }  \n}\nrender ASX { <div class=\"probe\">Ready</div> }\n}\n";
        let expected = "component Probe() {\n  client JS {\n    const template = `first\n  second`;  \n    if (ready) {\n      mount();\n    }\n  }\n  style {\n  .probe { white-space: pre; }  \n  }\n  render ASX { <div class=\"probe\">Ready</div> }\n}\n";

        assert_eq!(format_ax_source(source), expected);
    }

    #[test]
    fn does_not_treat_generic_types_as_markup_tags() {
        let source = "export query loadPosts(input: Map<String, Post>) -> List<Post> {\ndata posts: List<Post> = db.posts.all()\nreturn posts\n}\n";
        let expected = "export query loadPosts(input: Map<String, Post>) -> List<Post> {\n  data posts: List<Post> = db.posts.all()\n  return posts\n}\n";

        let formatted = format_ax_source(source);
        assert_eq!(formatted, expected);
        parse_backend_ax(&formatted).expect("formatted generic backend source should parse");
    }
}
