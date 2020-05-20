#![feature(proc_macro_diagnostic)]
mod autodiff;
mod reader;

extern crate proc_macro;

use reader::Reader;
use autodiff::solver::Solver;
use proc_macro::TokenStream;
use quote::quote;
use syn::*;
use syn::fold::Fold;

#[proc_macro_attribute]
pub fn into_backward(attr: TokenStream, item: TokenStream) -> TokenStream {

    let mut needs_grad: Vec<String> = vec!["input".to_string()];

    let attribute_args = parse_macro_input!(attr as AttributeArgs);
    for attribute in attribute_args {
        if let NestedMeta::Meta(meta) = attribute {
            if let Meta::Path(path) = meta {
                let ident = path.segments.last().unwrap().ident.to_string();
                needs_grad.push("self . ".to_string() + &ident);
            } else {
                panic!("Unsupported attribute argument, expected field name!")
            }
        } else {
            panic!("Unsupported attribute argument, expected field name!")
        }
    }
    let mut item_fn = parse::<ItemFn>(item).unwrap();

    let mut reader = Reader::new();
    item_fn = reader.fold_item_fn(item_fn);
    let arg = reader.get_output_arg();

    let mut solver = Solver::new();
    let backwards_block = solver.solve(arg, "output_grad".parse().unwrap(), needs_grad);

    let expanded = quote! {
        #item_fn

        fn backward(&mut self, input: Tensor, output_grad: Tensor) -> Tensor {
            #backwards_block
        }
    };

    TokenStream::from(expanded)
}