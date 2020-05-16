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
pub fn graph(_: TokenStream, item: TokenStream) -> TokenStream {

    //let attribute_args = parse_macro_input!(attr as AttributeArgs);
    let mut item_fn = parse::<ItemFn>(item).unwrap();

    let mut reader = Reader::new();
    item_fn = reader.fold_item_fn(item_fn);
    let input_name: proc_macro2::TokenStream = reader.get_input_name().parse().unwrap();
    let arg = reader.get_output_arg();

    let mut solver = Solver::new();
    let backwards_block = solver.solve(arg, "&output_grad".parse().unwrap(), &input_name);

    let expanded = quote! {
        #item_fn

        fn backward(&mut self, #input_name: Tensor, output_grad: Tensor) -> Tensor {
            #backwards_block
            //Tensor::zeros([1, 1, 1, 1])
        }
    };

    TokenStream::from(expanded)
}


/*
//We should resolve type and accept function paths instead, but for now this works
add_diff!(INSTANCE, "add", grad, grad);
add_diff!(INSTANCE, "sub", grad, -grad);
add_diff!(INSTANCE, "mul", b.transpose()*grad, grad*a.transpose());
add_diff!(INSTANCE, "div", grad/b, -(grad*a)/(b.powi(2)));
add_diff!(INSTANCE, "neg", -grad);
add_diff!(INSTANCE, "sin", grad*a.cos());
add_diff!(INSTANCE, "cos", grad*(-a.sin()));
add_diff!(INSTANCE, "tan", grad/(a.cos()).powi(2));
add_diff!(INSTANCE, "sinh", grad*a.cosh());
add_diff!(INSTANCE, "cosh", grad*a.sinh());
add_diff!(INSTANCE, "tanh", grad/(a.cosh()).powi(2));
add_diff!(INSTANCE, "exp", grad*a.exp()); //could store result of fw pass
add_diff!(INSTANCE, "ln", grad/a);
*/



/*
struct Helper {
    method: Lit,
    expressions: Vec<Expr>
}

impl Parse for Helper {
    fn parse(input: ParseStream) -> Result<Self> {
        let method = input.parse()?;
        let expressions = Vec::new();
        expressions.push(input.parse()?);
        //while input.peek(Expr::peek_any) {
        //    expressions.push(input.parse()?);
        //}
        Ok(Helper {
            method,
            expressions
        })
    }
}


#[proc_macro]
pub fn add_diff(input: TokenStream) -> TokenStream {
    let Helper { method, expressions} = parse_macro_input!(input as Helper);

}
*/