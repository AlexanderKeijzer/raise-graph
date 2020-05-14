use syn::fold::Fold;
use syn::*;
use quote::quote;
use std::mem;
use std::default::Default;
use std::fmt::Display;
use std::collections::HashMap;
use syn::spanned::Spanned;

#[derive(Debug, PartialEq)]
enum Arg {
    None,
    Operation(Box<Operation>),
    Item(String)
}

impl Arg {
    fn take(&mut self) -> Arg {
        mem::take(self)
    }
}

impl Default for Arg {
    fn default() -> Self {
        Arg::None
    }
}

impl Display for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Arg::Operation(op) => op.fmt(f),
            Arg::Item(item) => write!(f, "{}", item),
            _ => Ok(())
        }
    }
}

#[derive(Debug, PartialEq)]
struct Operation {
    receiver: Arg,
    method: String,
    args: Vec<Arg>
}

impl Operation {
    fn new(receiver: Arg, method: String, args: Vec<Arg>) -> Operation {
        Operation {
            receiver: receiver,
            method: method,
            args: args
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}(", self.method.to_string())?;
        write!(f, "{}", self.receiver)?;
        for i in 0..self.args.len() {
            write!(f, ", ")?;
            self.args[i].fmt(f)?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

pub struct Reader {
    input_name: String,
    objects: HashMap<String, Arg>,
    ops: Vec<Operation>,
    current_arg: Arg,
    output: Arg
}

impl Reader {
    pub fn new() -> Reader {
        Reader {
            input_name: "".to_string(),
            objects: HashMap::new(),
            ops: Vec::new(),
            current_arg: Arg::None,
            output: Arg::None,
        }
    }

    fn compile_output(&mut self, original: &Expr) -> Expr {
        let arg = self.current_arg.take();
        if Arg::None == arg {
            original.span().unwrap().error("Output cannot be none.").emit(); panic!()
        }
        self.output = arg;

        let inp = "Input: ".to_string() + &self.input_name;
        let mut expressions = "".to_string();
        for k in 0..self.ops.len() {
            expressions += &(format!("{}", self.ops[k]) + "\n");
        }
        let out = "Output: ".to_string() + &self.output.to_string();
        let new_code = quote! {
            {
                println!("{}", #inp);
                if !(#expressions).is_empty() {
                    println!("{}", #expressions);
                }
                println!("{}", #out);
                #original
            }
        };
        syn::parse2(new_code)
        .expect("could not generate prints")
    }
}

impl Fold for Reader {

    fn fold_pat_type(&mut self, ii: PatType) -> PatType {
        match ii.pat.as_ref() {
            Pat::Ident(i) => {
                self.input_name = i.ident.to_string();
                ii
            }
            _ => fold::fold_pat_type(self, ii)
        }
    }
    

    fn fold_local(&mut self, ii: Local) -> Local {
        let var_name = match &(&ii).pat {
            Pat::Ident(i) => {
                i.ident.to_string()
            }
            _ => {ii.span().unwrap().error("Unsupported local variable creation.").emit(); panic!()}
        };

        let arg;
        if (&ii).init.is_some() {
            let (_, exp) = ii.clone().init.unwrap();
            self.fold_expr(*exp);
            arg = self.current_arg.take();
        } else {
            arg = Arg::None;
        }
        self.objects.insert(var_name, arg);
        ii
    }

    fn fold_stmt(&mut self, mut ii: Stmt) -> Stmt {
        ii = fold::fold_stmt(self, ii);
        if let Stmt::Expr(i) = &ii {
            ii = Stmt::Expr(self.compile_output(i));
        } else if let Arg::Operation(op) = self.current_arg.take() {
            self.ops.push(*op);
        }
        ii
    }

    fn fold_expr(&mut self, mut ii: Expr) -> Expr {
        match ii.clone() {
            Expr::Binary(i) => {

                self.fold_expr(*i.left);
                let left = self.current_arg.take();

                self.fold_expr(*i.right);
                let right = self.current_arg.take();

                let method = match i.op {
                    BinOp::Add(_) => {
                        "add"
                    }
                    BinOp::Sub(_) => {
                        "sub"
                    }
                    BinOp::Mul(_) => {
                        "mul"
                    }
                    BinOp::Div(_) => {
                        "div"
                    }
                    _ => {i.op.span().unwrap().error("Unsupported bianry expression.").emit(); panic!()}
                };
                self.current_arg = Arg::Operation(Box::new(Operation::new(left, method.to_string(), vec![right])));
            }
            Expr::Unary(i) => {
                self.fold_expr(*i.expr);
                let receiver = self.current_arg.take();

                let op = match i.op {
                    UnOp::Neg(_) => {
                        "neg"
                    }
                    _ => {i.op.span().unwrap().error("Unsupported unary expression.").emit(); panic!()}
                };
                self.current_arg = Arg::Operation(Box::new(Operation::new(receiver, op.to_string(), vec![])));
            }
            Expr::Paren(i) => {
                self.fold_expr(*i.expr);
            }
            Expr::Assign(i) => {
                self.fold_expr(*i.left.clone());
                let arg = self.current_arg.take();
                if let Arg::Item(obj_name) = arg {
                    self.fold_expr(*i.right);
                    let arg = self.current_arg.take();
                    self.objects.insert(obj_name, arg);
                } else {
                    (*i.left).span().unwrap().error("Assigning to expression is not supported.").emit();
                    panic!();
                }
            }
            Expr::Lit(i) => {
                let lit = match i.lit {
                    Lit::Int(li) => li.to_string(),
                    Lit::Float(li) => li.to_string(),
                    _ => panic!()
                };
                self.current_arg = Arg::Item(lit.clone());
            }
            Expr::Path(i) => {
                let path = i.path.segments.last().unwrap().ident.to_string();
                if let Some(arg) = self.objects.remove(&path) {
                    self.current_arg = arg;
                } else {
                    self.current_arg = Arg::Item(path.clone());
                }
            }
            Expr::Return(mut i) => {
                self.fold_expr(*i.clone().expr.unwrap());
                i.expr = Some(Box::new(self.compile_output(&*i.expr.unwrap())));
                ii = Expr::Return(i);
            }
            Expr::Field(i) => {
                 let mut s;
                match *i.base {
                    Expr::Path(j) => {
                        s = j.path.get_ident().unwrap().to_string();
                    }
                    _ => {i.member.span().unwrap().error("Unsupported field indexing.").emit(); panic!()}
                }
                match i.member {
                    Member::Named(j) => {

                        s += &(".".to_string() + &j.to_string());
                    }
                    _ => {i.member.span().unwrap().error("Only name fields can be accessed.").emit(); panic!()} //panic!("Only name fields can be accessed.")
                }
                self.current_arg = Arg::Item(s.clone());
            }
            Expr::MethodCall(i) => {
                self.fold_expr(*i.receiver);
                let receiver = self.current_arg.take();
                let mut  args = Vec::new();
                for k in 0..i.args.len() {
                    self.fold_expr(i.args[k].clone());
                    args.push(self.current_arg.take());
                }
                self.current_arg = Arg::Operation(Box::new(Operation::new(receiver, i.method.to_string(), args)));
            }

            /*
            Expr::Let(i) => {
                self.operations.push("let".to_string());
            }
            Expr::Block(i) => {
                self.operations.push("block".to_string());
            }
            Expr::Verbatim(i) => {
                self.operations.push("verb".to_string());
            }
            Expr::Group(i) => {
                self.operations.push("group".to_string());
            }
            */


            _ => {ii.span().unwrap().error("Unsupported expression.").emit(); panic!()}
        }
        ii//fold::fold_expr(self, ii)
    }

}