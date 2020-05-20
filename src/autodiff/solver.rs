use crate::reader::Arg;
use crate::reader::Operation;
use crate::autodiff::autodiff::{OUTPUT_NAMES, AutoDiff};
use quote::{quote, format_ident};
use proc_macro2::TokenStream;
use syn::Ident;
use std::collections::HashMap;

pub struct Solver {
    autodiff: AutoDiff,
    curr_var: u32,
}

impl Solver {
    pub fn new() -> Solver {
        Solver {
            autodiff: AutoDiff::new(), // We should have a static instance of this
            curr_var: 1,
        }
    }

    pub fn solve(&mut self, arg_graph: Arg, grad: TokenStream, solve_for: Vec<String>) -> TokenStream {

        let mut solution_map: HashMap<String, Vec<TokenStream>> = HashMap::new();
        for needed_grad in solve_for {
            solution_map.insert(needed_grad, Vec::new());
        }
        let calculations = self.solve_operation(arg_graph, grad, &mut solution_map);

        // Create results of the gradient calculation
        let mut results = TokenStream::new();

        for (variable, solution) in solution_map {
            if variable != "input".to_string() {
                let ident: TokenStream = variable.parse().unwrap();
                results = quote! {
                    #ident.gradient = Some(Box::new(#(#solution)+*));
                    #results
                }
            } else {
                results = quote! {
                    #results
                    #(#solution)+*
                }
            }
        }

        quote! {
            #calculations
            #results
        }
    }

    fn solve_operation(&mut self, arg_graph: Arg, grad: TokenStream, solution_map: &mut HashMap<String, Vec<TokenStream>>) -> TokenStream {
        match arg_graph {
            Arg::None => panic!("None argument in graph!"),
            Arg::Item(mut item) => {
                item = item.replace("&", "").replace(".", " . "); // This is not very nice at all...
                if let Some(vec) = solution_map.get_mut(&item) {
                    vec.push(grad);
                }
                TokenStream::new()
            }
            Arg::Operation(op) => self.diff_operation(*op, grad, solution_map)
        }
    }

    
    fn diff_operation(&mut self, operation: Operation, grad: TokenStream, solution_map: &mut HashMap<String, Vec<TokenStream>>) -> TokenStream {

        // Get expressions needed to solve for input grad and other needed grads
        let needed_exprs = Solver::get_needed_expressions(&operation, solution_map);

        // Construct expression inputs (grad + a & b & ...)
        let inputs = self.define_inputs(&operation, &grad, &needed_exprs);

        // Solve every expression at this level collecting the results of the expressions
        let (expressions, next_level, idents) = self.define_expressions(operation, needed_exprs);

        // Solve every every expression at sublevel with results of this level
        let next_level_solved: Vec<TokenStream> = next_level.into_iter().map(|(arg, grad)| {
                self.solve_operation(arg, grad, solution_map)
            }).collect();
    
        // Create output block
        quote! {
            #(let #idents;)*
            {
                #inputs
                #(
                #expressions
                )*
            }
            #(
                #next_level_solved
            )*
        }
    }

    fn get_needed_expressions(operation: &Operation, solution_map: &HashMap<String, Vec<TokenStream>>) -> Vec<u8> {
        let mut calc_expression: Vec<u8> = Vec::new();

        let mut op_args = vec![&operation.receiver];
        op_args.append(&mut operation.args.iter().map(|f| f).collect());

        for i in 0..op_args.len() {
            let input_n = op_args[i].to_tokenstream();
            for to_grad_element in solution_map.keys()  {
                if input_n.to_string().contains(to_grad_element) {
                    calc_expression.push(i as u8);
                }
            }
        }
        calc_expression
    }

    fn define_inputs(&self, operation: &Operation, grad: &TokenStream, needed_exprs: &Vec<u8>) -> TokenStream {
        let mut inputs: Vec<TokenStream> = Vec::new();
        let mut input_names = Vec::new();

        let exprs = self.autodiff.get_expressions(&operation.method);

        let mut op_args = vec![&operation.receiver];
        op_args.append(&mut operation.args.iter().map(|f| f).collect());

        // We should save and use the forward pass if needed
        for i in 0..op_args.len() {
            let input_n = op_args[i].to_tokenstream();
            let mut calc = false;
            for needed_exp in needed_exprs {
                let (_, needed_args) = &exprs[*needed_exp as usize];
                if needed_args.contains(&(i as u8)) {
                    calc = true;
                    break
                }
            }
            if calc {
                inputs.push(input_n);
                input_names.push(format_ident!("{}", OUTPUT_NAMES[i]));
            }
        }

        // Since we get input as reference in the forwards pass and as owned value in the backwards pass
        // we should replace to avoid issues. However, it would be even better to keep track of usage and
        // use an owned value where the operation can be done inplace.
        for i in 0..inputs.len() {
            let mut expr_str = inputs[i].to_string();
            expr_str = expr_str.replace("input", "(&input)");
            inputs[i] = expr_str.parse().unwrap();
        }

        quote! {
            let grad = #grad;
            #(let #input_names = #inputs;)*
        }
    }

    fn define_expressions(&mut self, mut operation: Operation, needed_exprs: Vec<u8>) -> (Vec<TokenStream>, Vec<(Arg, TokenStream)>, Vec<Ident>) {

        let mut output = Vec::new();

        let mut next_level: Vec<(Arg, TokenStream)> = Vec::new();
        let mut idents = Vec::new();
        let exprs = self.autodiff.get_expressions(&operation.method).clone();
        for i in 0..exprs.len() {
            if !needed_exprs.contains(&(i as u8)) {
                continue;
            }
            let (expr, _) = &exprs[i];

            let ident = format_ident!("x{}", self.curr_var);
            self.curr_var += 1;
            idents.push(ident.clone());

            if i == 0 {
                next_level.push((operation.receiver.clone(), ident.to_string().parse().unwrap()));
            } else {
                next_level.push((operation.args.remove(0), ident.to_string().parse().unwrap()));
            }

            output.push(quote! {
                #ident = #expr;
            });
        }
        (output, next_level, idents)
    }
}