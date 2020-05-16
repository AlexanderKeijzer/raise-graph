use crate::reader::Arg;
use crate::reader::Operation;
use crate::autodiff::autodiff::AutoDiff;
use quote::{quote, format_ident};
use proc_macro2::TokenStream;
use syn::Ident;

static ARRGUMENT_NAMES: &[&str] = &["a", "b", "c", "d", "e"];

pub struct Solver {
    autodiff: AutoDiff,
    curr_var: u32,
    out_exprs: Vec<Ident>
}

impl Solver {
    pub fn new() -> Solver {
        Solver {
            autodiff: AutoDiff::new(),
            curr_var: 1,
            out_exprs: Vec::new()
        }
    }

    pub fn solve(&mut self, arg_graph: Arg, grad: TokenStream, input: &TokenStream) -> TokenStream { 
        let result = self.solve_operation(arg_graph, grad, input);
        let s = &self.out_exprs;
        quote! {
            #result
            #(#s)+*
        }
    }

    fn solve_operation(&mut self, arg_graph: Arg, grad: TokenStream, input: &TokenStream) -> TokenStream {
        match arg_graph {
            Arg::None => panic!(),
            Arg::Item(_) => quote! {},//item.parse().unwrap(),
            Arg::Operation(op) => self.diff(*op, grad, input)
        }
    }

    
    fn diff(&mut self, operation: Operation, grad: TokenStream, input: &TokenStream) -> TokenStream {

        // Construct expression inputs (grad + a & b & ...)
        let (inputs, save_expr) = Solver::define_inputs(&operation, &grad, &input);

        // Solve every expression at this level collecting the results of the expressions
        let (expressions, next_level, idents) = self.define_expressions(operation, save_expr);

        // Solve every every expression at sublevel with results of this level
        let next_level_solved: Vec<TokenStream> = next_level.into_iter().map(|(arg, ident)| {
                self.solve_operation(arg, ident, input)
            }).collect();
        
        // Create output block
        quote! {
            #(let #idents;)*
            {
                #inputs
                #expressions
            }
            #(
                #next_level_solved
            )*
        }
    }

    fn define_inputs(operation: &Operation, grad: &TokenStream, input: &TokenStream) -> (TokenStream, usize) {
        let mut inputs: Vec<TokenStream> = Vec::new();
        let mut input_names = Vec::new();
        let mut save_expr: usize = usize::MAX;

        // We should detect which ones we actually need! In the best case
        // we can save results in the forward pass.
        let input_n = operation.receiver.to_tokenstream();
        if input_n.to_string() == input.to_string() || input_n.to_string() == "& ".to_string() + &input.to_string() {
            save_expr = 0;
        }
        inputs.push(input_n);
        input_names.push(format_ident!("{}", ARRGUMENT_NAMES[0]));

        for i in 0..operation.args.len() {
            let input_n = operation.args[i].to_tokenstream();
            if input_n.to_string() == input.to_string() || input_n.to_string() == "& ".to_string() + &input.to_string() {
                save_expr = i+1;
            }
            inputs.push(input_n);
            input_names.push(format_ident!("{}", ARRGUMENT_NAMES[i+1]));
        }

        (quote! {
            let grad = #grad;
            #(let #input_names = #inputs;)*
        }, save_expr)
    }

    fn define_expressions(&mut self, mut operation: Operation, save_expr: usize) -> (TokenStream, Vec<(Arg, TokenStream)>, Vec<Ident>) {

        let mut output = TokenStream::new();

        let mut next_level: Vec<(Arg, TokenStream)> = Vec::new();
        let mut idents = Vec::new();
        let exprs = self.autodiff.get_expressions(operation.method).clone();
        for i in 0..exprs.len() {
            let expr = &exprs[i];

            let ident = format_ident!("x{}", self.curr_var);
            self.curr_var += 1;
            idents.push(ident.clone());

            if save_expr == i {
                self.out_exprs.push(ident.clone());
            }

            if i == 0 {
                next_level.push((operation.receiver.clone(), ident.to_string().parse().unwrap()));
            } else {
                next_level.push((operation.args.remove(0), ident.to_string().parse().unwrap()));
            }

            output = quote! {
                #output
                #ident = #expr;
            };
        }
        (output, next_level, idents)
    }
}