#![feature(proc_macro_diagnostic)]
mod autodiff;
mod reader;

extern crate raise;
extern crate proc_macro;

use reader::Reader;
use proc_macro::TokenStream;
use quote::quote;
use syn::*;
use syn::fold::Fold;

#[proc_macro_attribute]
pub fn graph(attr: TokenStream, item: TokenStream) -> TokenStream {

    let attribute_args = parse_macro_input!(attr as AttributeArgs);
    let mut item_fn = parse::<ItemFn>(item).unwrap();

    let mut reader = Reader::new();
    item_fn = reader.fold_item_fn(item_fn);


    let expanded = quote! {
        #item_fn
    };

    TokenStream::from(expanded)
}