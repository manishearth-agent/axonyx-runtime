//! Rewrite only members rooted in known native JSON bindings, never literal text.
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::collections::BTreeSet;

pub(crate) fn is_json_alias(code: &str, bindings: &BTreeSet<String>) -> bool {
    let Ok(tokens) = code.parse::<TokenStream>() else {
        return false;
    };
    let tokens: Vec<_> = tokens.into_iter().collect();
    chain_end(&tokens, 0, bindings).is_some_and(|end| end == tokens.len())
}

fn root(token: &TokenTree, bindings: &BTreeSet<String>) -> bool {
    match token {
        TokenTree::Ident(name) => bindings.contains(&name.to_string()),
        TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
            let tokens: Vec<_> = group.stream().into_iter().collect();
            chain_end(&tokens, 0, bindings).is_some_and(|end| end == tokens.len())
        }
        _ => false,
    }
}

fn dot(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == '.')
}

fn chain_end(tokens: &[TokenTree], start: usize, bindings: &BTreeSet<String>) -> Option<usize> {
    if !tokens.get(start).is_some_and(|token| root(token, bindings)) {
        return None;
    }
    let mut end = start + 1;
    while end < tokens.len() {
        match &tokens[end] {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket => end += 1,
            token if dot(token) && matches!(tokens.get(end + 1), Some(TokenTree::Ident(_))) => {
                if matches!(tokens.get(end + 2), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
                {
                    break;
                }
                end += 2;
            }
            _ => break,
        }
    }
    Some(end)
}

pub(crate) fn rewrite_members(code: &str, bindings: &BTreeSet<String>) -> Result<String, ()> {
    if bindings.is_empty() {
        return Ok(code.into());
    }
    let tokens = code.parse::<TokenStream>().map_err(|_| ())?;
    let mut replacements = Vec::new();
    collect(tokens, bindings, &mut replacements);
    replacements.sort_by_key(|(start, _, _)| *start);
    let mut output = String::new();
    let mut cursor = 0;
    for (start, end, replacement) in replacements {
        if start < cursor
            || end > code.len()
            || !code.is_char_boundary(start)
            || !code.is_char_boundary(end)
        {
            return Err(());
        }
        output.push_str(&code[cursor..start]);
        output.push_str(&replacement);
        cursor = end;
    }
    output.push_str(&code[cursor..]);
    Ok(output)
}

fn collect(
    tokens: TokenStream,
    bindings: &BTreeSet<String>,
    replacements: &mut Vec<(usize, usize, String)>,
) {
    let tokens: Vec<_> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            collect(group.stream(), bindings, replacements);
        }
        // A same-named field/path segment is not a local variable reference.
        if (index > 0 && dot(&tokens[index - 1]))
            || (index > 1
                && matches!(&tokens[index - 1], TokenTree::Punct(punct) if punct.as_char() == ':')
                && matches!(&tokens[index - 2], TokenTree::Punct(punct) if punct.as_char() == ':'))
        {
            continue;
        }
        let Some(end) = chain_end(&tokens, index, bindings) else {
            continue;
        };
        let mut member = index + 1;
        while member < end {
            if dot(&tokens[member]) {
                let TokenTree::Ident(name) = &tokens[member + 1] else {
                    unreachable!()
                };
                replacements.push((
                    tokens[member].span().byte_range().start,
                    name.span().byte_range().end,
                    format!("[{:?}]", name.to_string().trim_start_matches("r#")),
                ));
                member += 2;
            } else {
                member += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_native_chains_without_touching_strings_methods_or_records() {
        let bindings = BTreeSet::from(["prepared".into()]);
        for (source, expected) in [
            ("prepared.columns.body", "prepared[\"columns\"][\"body\"]"),
            ("prepared.items[0].name", "prepared[\"items\"][0][\"name\"]"),
            (
                "(prepared.columns).body",
                "(prepared[\"columns\"])[\"body\"]",
            ),
            (
                "json!({\"prepared.columns\": prepared.columns})",
                "json!({\"prepared.columns\": prepared[\"columns\"]})",
            ),
            ("prepared.columns.clone()", "prepared[\"columns\"].clone()"),
            ("record.prepared.columns", "record.prepared.columns"),
            (
                "\"🙂 prepared.columns\"; prepared.columns",
                "\"🙂 prepared.columns\"; prepared[\"columns\"]",
            ),
        ] {
            assert_eq!(rewrite_members(source, &bindings).unwrap(), expected);
        }
        assert!(is_json_alias("prepared.columns", &bindings));
        assert!(!is_json_alias("prepared.columns.clone()", &bindings));
    }
}
