#![feature(proc_macro_diagnostic)]
mod autodiff;
mod reader;

extern crate raise;
extern crate proc_macro;

use reader::Reader;
use autodiff::solver::Solver;
use proc_macro::TokenStream;
use quote::quote;
use syn::*;
use syn::fold::Fold;

#[proc_macro_attribute]
pub fn into_backward(_: TokenStream, item: TokenStream) -> TokenStream {

    //let attribute_args = parse_macro_input!(attr as AttributeArgs);
    let mut item_fn = parse::<ItemFn>(item).unwrap();

    let mut reader = Reader::new();
    item_fn = reader.fold_item_fn(item_fn);
    let input_name: proc_macro2::TokenStream = reader.get_input_name().parse().unwrap();
    let arg = reader.get_output_arg();

    let mut solver = Solver::new();
    let backwards_block = solver.solve(arg, "output_grad".parse().unwrap(), &input_name);

    let expanded = quote! {
        #item_fn

        fn backward(&mut self, #input_name: Tensor, output_grad: Tensor) -> Tensor {
            #backwards_block
        }
    };

    TokenStream::from(expanded)
}