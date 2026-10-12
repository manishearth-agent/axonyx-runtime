//! Lower declared helper calls without rewriting literal text or method names.
use crate::ax_backend_lowering::prelude::AxFunctionPlan;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::collections::BTreeSet;

pub(crate) fn rewrite_calls(
    code: &str,
    functions: &[AxFunctionPlan],
    records: &BTreeSet<String>,
) -> Result<String, ()> {
    if functions.is_empty() {
        return Ok(code.to_string());
    }
    let tokens = code.parse::<TokenStream>().map_err(|_| ())?;
    let mut replacements = Vec::new();
    collect(code, tokens, functions, records, &mut replacements)?;
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
    code: &str,
    tokens: TokenStream,
    functions: &[AxFunctionPlan],
    records: &BTreeSet<String>,
    replacements: &mut Vec<(usize, usize, String)>,
) -> Result<(), ()> {
    let tokens: Vec<_> = tokens.into_iter().collect();
    let mut index = 0;
    while index < tokens.len() {
        let function = match &tokens[index] {
            TokenTree::Ident(name)
                if !(index > 0
                    && matches!(&tokens[index - 1], TokenTree::Punct(p) if p.as_char() == '.')
                    || index > 1
                        && matches!(&tokens[index - 1], TokenTree::Punct(p) if p.as_char() == ':')
                        && matches!(&tokens[index - 2], TokenTree::Punct(p) if p.as_char() == ':')) =>
            {
                functions.iter().find(|function| *name == function.name)
            }
            _ => None,
        };
        if let (Some(function), Some(TokenTree::Group(group))) = (function, tokens.get(index + 1)) {
            if group.delimiter() == Delimiter::Parenthesis {
                let start = tokens[index].span().byte_range().start;
                let end = group.span().byte_range().end;
                let mut ranges = Vec::new();
                let mut arg_start = group.span_open().byte_range().end;
                for token in group.stream() {
                    if matches!(&token, TokenTree::Punct(p) if p.as_char() == ',') {
                        ranges.push((arg_start, token.span().byte_range().start));
                        arg_start = token.span().byte_range().end;
                    }
                }
                let arg_end = group.span_close().byte_range().start;
                if !code[arg_start..arg_end].trim().is_empty() {
                    ranges.push((arg_start, arg_end));
                }
                if ranges.len() != function.input.len() {
                    return Err(());
                }
                let args = ranges
                    .iter()
                    .zip(&function.input)
                    .map(|(&(start, end), field)| {
                        let arg = rewrite_calls(code[start..end].trim(), functions, records)?;
                        Ok(if records.contains(&field.rust_ty) {
                            format!("&({arg})")
                        } else if matches!(
                            field.rust_ty.as_str(),
                            "bool" | "i32" | "i64" | "u32" | "u64" | "f32" | "f64"
                        ) {
                            arg
                        } else {
                            format!("({arg}).clone()")
                        })
                    })
                    .collect::<Result<Vec<_>, ()>>()?;
                replacements.push((
                    start,
                    end,
                    format!("{}({})?", function.name, args.join(", ")),
                ));
                index += 2;
                continue;
            }
        }
        if let TokenTree::Group(group) = &tokens[index] {
            collect(code, group.stream(), functions, records, replacements)?;
        }
        index += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ax_backend_lowering::lower_backend_document;
    use crate::ax_backend_parser::parse_backend_ax;

    fn functions() -> Vec<AxFunctionPlan> {
        let source = "export fn echo(text: String) -> String {\n  return text\n}\nexport fn policy(user: User, age: Int) -> Bool {\n  return true\n}";
        lower_backend_document(&parse_backend_ax(source).unwrap())
            .unwrap()
            .functions
    }

    #[test]
    fn rewrites_nested_calls_in_macros_without_touching_literals_or_methods() {
        let functions = functions();
        let records = BTreeSet::from(["User".into()]);
        for (source, expected) in [
            ("echo(input.text)", "echo((input.text).clone())?"),
            (
                "echo(echo(input.text))",
                "echo((echo((input.text).clone())?).clone())?",
            ),
            ("policy(user, 42)", "policy(&(user), 42)?"),
            (
                "json!({\"echo(input.text)\": echo(input.text)})",
                "json!({\"echo(input.text)\": echo((input.text).clone())?})",
            ),
            (
                "\"🙂 echo(input.text)\"; echo(input.text)",
                "\"🙂 echo(input.text)\"; echo((input.text).clone())?",
            ),
            ("record.echo(input.text)", "record.echo(input.text)"),
            ("Other::echo(input.text)", "Other::echo(input.text)"),
        ] {
            assert_eq!(
                rewrite_calls(source, &functions, &records).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn wrong_arity_is_rejected_before_emitting_rust() {
        for source in ["echo()", "echo(input.text, input.other)", "policy(user)"] {
            assert!(rewrite_calls(source, &functions(), &BTreeSet::new()).is_err());
        }
    }
}
