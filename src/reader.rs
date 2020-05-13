use syn::fold::Fold;
use syn::*;
use quote::quote;
use std::mem;
use std::default::Default;
use std::fmt::Display;

#[derive(Debug)]
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

#[derive(Debug)]
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
        write!(f, "{}", self.receiver)?;
        write!(f, "{}(", self.method.to_string())?;
        for i in 0..self.args.len() {
            self.args[i].fmt(f)?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

pub struct Reader {
    input_name: String,
    operations: Vec::<String>,
    objects: Vec<String>,
    ops: Vec<Operation>,
    current_arg: Arg
}

impl Reader {
    pub fn new() -> Reader {
        Reader {
            input_name: "".to_string(),
            operations: Vec::new(),
            objects: Vec::new(),
            ops: Vec::new(),
            current_arg: Arg::None
        }
    }
}

impl Fold for Reader {

    fn fold_pat(&mut self, ii: Pat) -> Pat {
        match &ii {
            Pat::Ident(i) => {
                self.objects.push(i.ident.to_string());
            }
            _ => ()
        }
        fold::fold_pat(self, ii)
    }

    
    fn fold_pat_type(&mut self, ii: PatType) -> PatType {
        match ii.pat.as_ref() {
            Pat::Ident(i) => {
                self.input_name = i.ident.to_string();
                ii
            }
            _ => fold::fold_pat_type(self, ii)
        }
        //fold::fold_pat_type(self, ii)
    }
    

    fn fold_local(&mut self, ii: Local) -> Local {
        match &(&ii).pat {
            Pat::Ident(i) => {
                self.operations.push(format!("let {}", i.ident.to_string()));
            }
            _ => panic!()
        }
        if ii.init.is_some() {
            self.operations.push("=".to_string());
        }
        fold::fold_local(self, ii)
    }

    fn fold_stmt(&mut self, mut ii: Stmt) -> Stmt {
        ii = fold::fold_stmt(self, ii);
        if let Arg::Operation(op) = self.current_arg.take() {
            self.ops.push(*op);
        }
        if let Stmt::Expr(i) = &ii {
            let ops = self.operations.join(" ");
            let objs = self.objects.join(", ");
            let inp = self.input_name.to_string();
            let mut s = "".to_string();
            for k in 0..self.ops.len() {
                s += &(format!("{}", self.ops[k]) + "\n");
            }
            let new_code = quote! {
                {
                    println!("{}", #inp);
                    println!("{}", #s);
                    println!("{}", #ops);
                    println!("{}", #objs);
                    #ii
                }
            };
            ii = syn::parse2(new_code)
            .expect("could not generate prints");
        } else {
            self.operations.push("\n".to_string());
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
                        "+"
                    }
                    BinOp::Sub(_) => {
                        "-"
                    }
                    BinOp::Mul(_) => {
                        "*"
                    }
                    BinOp::Div(_) => {
                        "/"
                    }
                    _ => panic!()
                };
                self.current_arg = Arg::Operation(Box::new(Operation::new(left, method.to_string(), vec![right])));
            }
            Expr::Unary(i) => {
                match i.op {
                    UnOp::Neg(_) => {
                        self.operations.push("-".to_string());
                    }
                    _ => ()
                }
                self.fold_expr(*i.expr);
            }
            Expr::Paren(i) => {
                self.fold_expr(*i.expr);
            }
            Expr::Assign(i) => {
                self.operations.push("ass".to_string());
                self.fold_expr(*i.left);
                self.operations.push("=".to_string());
                self.fold_expr(*i.right);
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
                self.current_arg = Arg::Item(path.clone());
            }
            Expr::Return(i) => {
                let ops = self.operations.join(" ");
                let objs = self.objects.join(", ");
                let inp = self.input_name.to_string();
                let o = format!("{:?}", self.ops);
                let new_code = quote! {
                    {
                        println!("{}", #inp);
                        println!("{}", #o);
                        println!("{}", #ops);
                        println!("{}", #objs);
                        #ii
                    }
                };

                ii = syn::parse2(new_code)
                .expect("could not generate prints");
            }
            Expr::Field(i) => {
                 let mut s;
                match *i.base {
                    Expr::Path(j) => {
                        s = j.path.get_ident().unwrap().to_string();
                    }
                    _ => panic!()
                }
                match i.member {
                    Member::Named(j) => {

                        s += &(".".to_string() + &j.to_string());
                    }
                    _ => panic!()
                }
                self.current_arg = Arg::Item(s.clone());
                self.operations.push(s);
            }
            Expr::MethodCall(i) => {
                self.fold_expr(*i.receiver);
                let receiver = self.current_arg.take();
                let mut  args = Vec::new();
                for k in 0..i.args.len() {
                    self.fold_expr(i.args[k].clone());
                    args.push(self.current_arg.take());
                }
                self.current_arg = Arg::Operation(Box::new(Operation::new(receiver, ".".to_string() + &i.method.to_string(), args)));
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


            _ => panic!("Unsupported expression.")
        }
        ii//fold::fold_expr(self, ii)
    }

}