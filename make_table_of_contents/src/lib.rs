use std::{
    collections::HashMap,
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader, ErrorKind},
};

use proc_macro::{Literal, TokenStream, TokenTree};
use quote::quote;
use syn::{LitStr, Token, parse::Parse, parse_macro_input};

#[derive(Debug)]
struct Input {
    source: String,
    target: Option<String>,
}

impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let source_lit = input.parse::<LitStr>()?;
        let Ok(_) = input.parse::<Token![,]>() else {
            return Ok(Input {
                source: source_lit.value(),
                target: None,
            });
        };
        let target_lit = input.parse::<LitStr>()?;
        Ok(Input {
            source: source_lit.value(),
            target: Some(target_lit.value()),
        })
    }
}

#[proc_macro]
pub fn make_table_of_contents(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as Input);

    let file = match File::open(&input.source) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let error = format!("Source file not found {:?}", input.source);
            return quote! {compile_error!(#error)}.into();
        }
        Err(e) => {
            let error = format!("Cannot read source file {:?}: {}", input.source, e);
            return quote! {compile_error!(#error)}.into();
        }
    };

    let mut toc = String::new();
    let reader = BufReader::new(file);
    let mut section_numbers = HashMap::<i32, i32>::new();
    let mut code_block = false;
    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };

        if line.starts_with("```") {
            code_block = !code_block;
            continue;
        }
        if code_block {
            continue;
        }

        let mut chars = line.chars().peekable();
        let mut heading_level = 0;
        while chars.peek().is_some_and(|c| *c == '#') {
            chars.next();
            heading_level += 1;
        }

        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if heading_level > 0 {
            for _ in 0..(heading_level - 1) {
                _ = write!(toc, "&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;");
            }
            *section_numbers.entry(heading_level - 1).or_insert(0) += 1;
            _ = write!(toc, "_**");
            for i in 0..heading_level {
                _ = write!(
                    toc,
                    "{}\u{2024}", // \u2024 is an alternative . character to avoid the automatic list creationg from markdown
                    section_numbers.get(&(i as i32)).unwrap_or(&1)
                );
            }
            _ = write!(toc, "**_");
            let section = chars.collect::<String>();
            _ = write!(
                toc,
                " [{}]({}#{})  \n",
                section,
                input.target.as_ref().map(|s| s.as_str()).unwrap_or(""),
                section
                    .to_lowercase()
                    .replace(" ", "-")
                    .replace(",", "")
                    .replace("\"", "")
                    .replace("`", ""),
            );
            section_numbers.insert(heading_level, 0);
        }
    }

    TokenTree::Literal(Literal::string(&toc)).into()
}
