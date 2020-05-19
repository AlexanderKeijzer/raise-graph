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
        //if let Arg::Operation(op) = &arg_graph {
        //    Solver::reverse_graph(&(**op), &vec!["&input".to_string()])
        //}
        let to_grad: Vec<String> = (vec!["input", "self.weight", "self.bias"]).into_iter().map(|s| s.to_string()).collect();
        let result = self.solve_operation(arg_graph, grad, &to_grad, input);
        let s = &self.out_exprs;
        quote! {
            #result
            #(#s)+*
        }
    }

    fn solve_operation(&mut self, arg_graph: Arg, grad: TokenStream, to_grad: &Vec<String>, input: &TokenStream) -> TokenStream {
        match arg_graph {
            Arg::None => panic!("None argument in graph!"),
            Arg::Item(_) => quote! {},//item.parse().unwrap(),
            Arg::Operation(op) => self.diff(*op, grad, to_grad, input)
        }
    }

    
    fn diff(&mut self, operation: Operation, grad: TokenStream, to_grad: &Vec<String>, input: &TokenStream) -> TokenStream {

        // Get expressions needed to solve for input grad and other wanted grads
        let needed_exprs = Solver::get_needed_expressions(&operation, to_grad);

        // Construct expression inputs (grad + a & b & ...)
        let (inputs, save_expr) = self.define_inputs(&operation, &grad, &needed_exprs, &input);

        // Solve every expression at this level collecting the results of the expressions
        let (expressions, next_level, idents) = self.define_expressions(operation, save_expr, needed_exprs);

        // Solve every every expression at sublevel with results of this level
        let next_level_solved: Vec<TokenStream> = next_level.into_iter().map(|(arg, ident)| {
                self.solve_operation(arg, ident, to_grad, input)
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

    fn get_needed_expressions(operation: &Operation, to_grad: &Vec<String>) -> Vec<u8> {
        let mut calc_expression: Vec<u8> = Vec::new();

        let input_n = operation.receiver.to_tokenstream();
        for to_grad_element in to_grad  {
            if input_n.to_string().contains(to_grad_element) {
                calc_expression.push(0);
            }
        }

        for i in 0..operation.args.len() {
            let input_n = operation.args[i].to_tokenstream();
            for to_grad_element in to_grad  {
                if input_n.to_string().contains(to_grad_element) {
                    calc_expression.push((i+1) as u8);
                }
            }
        }
        calc_expression
    }

    fn define_inputs(&self, operation: &Operation, grad: &TokenStream, needed_exprs: &Vec<u8>, input: &TokenStream) -> (TokenStream, usize) {
        let mut inputs: Vec<TokenStream> = Vec::new();
        let mut input_names = Vec::new();
        let mut save_expr: usize = usize::MAX;

        let exprs = self.autodiff.get_expressions(&operation.method);

        // We should save and use the forward pass if needed
        let input_n = operation.receiver.to_tokenstream();
        if input_n.to_string() == input.to_string() {
            save_expr = 0;
        }
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
            input_names.push(format_ident!("{}", ARRGUMENT_NAMES[0]));
        }

        for i in 0..operation.args.len() {
            let input_n = operation.args[i].to_tokenstream();
            if input_n.to_string() == input.to_string() {
                save_expr = i+1;
            }
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
                input_names.push(format_ident!("{}", ARRGUMENT_NAMES[i+1]));
            }
        }

        //Since we get input as reference in the forwards pass and as owned value in the backwards pass
        //we should replace to avoid issues
        for i in 0..inputs.len() {
            let mut expr_str = inputs[i].to_string();
            expr_str = expr_str.replace("input", "(&input)");
            inputs[i] = expr_str.parse().unwrap();
        }

        (quote! {
            let grad = #grad;
            #(let #input_names = #inputs;)*
        }, save_expr)
    }

    fn define_expressions(&mut self, mut operation: Operation, save_expr: usize, calc_expression: Vec<u8>) -> (TokenStream, Vec<(Arg, TokenStream)>, Vec<Ident>) {

        let mut output = TokenStream::new();

        let mut next_level: Vec<(Arg, TokenStream)> = Vec::new();
        let mut idents = Vec::new();
        let exprs = self.autodiff.get_expressions(&operation.method).clone();
        for i in 0..exprs.len() {
            if !calc_expression.contains(&(i as u8)) {
                continue;
            }
            let (expr, _) = &exprs[i];

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