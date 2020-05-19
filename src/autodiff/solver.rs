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
            autodiff: AutoDiff::new(),
            curr_var: 1,
        }
    }

    pub fn solve(&mut self, arg_graph: Arg, grad: TokenStream, solve_for: Vec<String>) -> TokenStream {

        let mut solution_map: HashMap<String, Vec<TokenStream>> = HashMap::new();
        for needed_grad in solve_for {
            solution_map.insert(needed_grad, Vec::new());
        }
        let result = self.solve_operation(arg_graph, grad, &mut solution_map);

        let mut outputs = TokenStream::new();

        for (variable, solution) in solution_map {
            if variable != "input".to_string() {
                let ident: TokenStream = variable.parse().unwrap();
                outputs = quote! {
                    #ident.gradient = Some(Box::new(#(#solution)+*));
                    #outputs
                }
            } else {
                outputs = quote! {
                    #outputs
                    #(#solution)+*
                }
            }
        }
        quote! {
            #result
            #outputs
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
            Arg::Operation(op) => self.diff(*op, grad, solution_map)
        }
    }

    
    fn diff(&mut self, operation: Operation, grad: TokenStream, solution_map: &mut HashMap<String, Vec<TokenStream>>) -> TokenStream {

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
                #expressions
            }
            #(
                #next_level_solved
            )*
        }
    }

    fn get_needed_expressions(operation: &Operation, solution_map: &mut HashMap<String, Vec<TokenStream>>) -> Vec<u8> {
        let mut calc_expression: Vec<u8> = Vec::new();

        let input_n = operation.receiver.to_tokenstream();
        for to_grad_element in solution_map.keys()  {
            if input_n.to_string().contains(to_grad_element) {
                calc_expression.push(0);
            }
        }

        for i in 0..operation.args.len() {
            let input_n = operation.args[i].to_tokenstream();
            for to_grad_element in solution_map.keys()  {
                if input_n.to_string().contains(to_grad_element) {
                    calc_expression.push((i+1) as u8);
                }
            }
        }
        calc_expression
    }

    fn define_inputs(&self, operation: &Operation, grad: &TokenStream, needed_exprs: &Vec<u8>) -> TokenStream {
        let mut inputs: Vec<TokenStream> = Vec::new();
        let mut input_names = Vec::new();

        let exprs = self.autodiff.get_expressions(&operation.method);

        // We should save and use the forward pass if needed
        let input_n = operation.receiver.to_tokenstream();
        let mut calc = false;
        for needed_exp in needed_exprs {
            let (_, needed_args) = &exprs[*needed_exp as usize];
            if needed_args.contains(&0) {
                calc = true;
                break
            }
        }
        if calc {
            inputs.push(input_n);
            input_names.push(format_ident!("{}", OUTPUT_NAMES[0]));
        }

        for i in 0..operation.args.len() {
            let input_n = operation.args[i].to_tokenstream();
            let mut calc = false;
            for needed_exp in needed_exprs {
                let (_, needed_args) = &exprs[*needed_exp as usize];
                if needed_args.contains(&((i+1) as u8)) {
                    calc = true;
                    break
                }
            }
            if calc {
                inputs.push(input_n);
                input_names.push(format_ident!("{}", OUTPUT_NAMES[i+1]));
            }
        }

        //Since we get input as reference in the forwards pass and as owned value in the backwards pass
        //we should replace to avoid issues
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

    fn define_expressions(&mut self, mut operation: Operation, needed_exprs: Vec<u8>) -> (TokenStream, Vec<(Arg, TokenStream)>, Vec<Ident>) {

        let mut output = TokenStream::new();

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

            output = quote! {
                #output
                #ident = #expr;
            };
        }
        (output, next_level, idents)
    }

    /*

    fn reverse_graph(operation: &Operation, param_list: &Vec<String>) {

        let mut op_dep_map = HashMap::new();
        let mut var_dep_map = HashMap::new();

        op_dep_map.insert(1, (0, operation));

        let curr_var = 1;
        Solver::fill_map(&mut op_dep_map, &mut var_dep_map, param_list, curr_var, operation);
/*
        let mut needed_ops = Vec::new();
        for var_deps in var_dep_map.values() {
            needed_ops.append(&mut var_deps.clone());
            let mut next_ops = Vec::new();
            for needed in var_deps {
                if needed > &0 {
                    let (next, _) = op_dep_map.get(needed).unwrap();
                    next_ops.push(*next);
                }
            }
            while !next_ops.is_empty() {
                let needed = next_ops.pop().unwrap();
                if needed > 0 {
                    let (next, _) = op_dep_map.get(&needed).unwrap();
                    next_ops.push(*next);
                }
            }
        }
*/
        println!("{:?}", op_dep_map);
        //println!("{:?}", var_dep_map);


    }

    fn fill_map<'a, 'b: 'a>(op_dep_map: &'a mut HashMap<i32, (i32, &'b Operation)>, var_dep_map: &mut HashMap<String, Vec<i32>>, param_list: &Vec<String>, mut curr_var: i32, operation: &'b Operation) {

        let mut ops = Vec::new();
        let dep = curr_var;

        if let Arg::Operation(inner_op) = &operation.receiver {
            ops.push(inner_op);
            curr_var += 1;
            op_dep_map.insert(curr_var, (dep, inner_op));
        } else if let Arg::Item(param) = &operation.receiver {
            if param_list.contains(param) {
                let res = var_dep_map.remove(param);
                if let Some(mut curr) = res {
                    curr.push(curr_var);
                    var_dep_map.insert(param.to_owned(), curr);
                } else {
                    var_dep_map.insert(param.to_owned(), vec![curr_var]);
                }
            }
        }
        for arg in &operation.args {
            if let Arg::Operation(inner_op) = arg {
                ops.push(inner_op);
                curr_var += 1;
                op_dep_map.insert(curr_var, (dep, inner_op));
            } else if let Arg::Item(param) = &operation.receiver {
                if param_list.contains(param) {
                    let res = var_dep_map.remove(param);
                    if let Some(mut curr) = res {
                        curr.push(curr_var);
                        var_dep_map.insert(param.to_owned(), curr);
                    } else {
                        var_dep_map.insert(param.to_owned(), vec![curr_var]);
                    }
                }
            }
        }
        let mut c_v = dep;
        for op in ops {
            c_v += 1;
            Solver::fill_map(op_dep_map, var_dep_map, param_list, c_v, op);
        }
    }

    */
}