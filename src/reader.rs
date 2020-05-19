use syn::fold::Fold;
use syn::*;
use quote::quote;
use std::mem;
use std::default::Default;
use std::fmt::Display;
use std::collections::HashMap;
use syn::spanned::Spanned;
use proc_macro2::TokenStream;

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

    pub fn get_output_arg(self) -> Arg {
        self.output
    }

    fn compile_output(&mut self, original: &Expr) -> Expr {
        let arg = self.current_arg.take();
        if Arg::None == arg {
            original.span().unwrap().error("Output cannot be none.").emit(); panic!("Output cannot be none.")
        }
        self.output = arg;
        /*

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
        */
        original.clone()
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
            _ => {ii.span().unwrap().error("Unsupported local variable creation.").emit(); panic!("Unsupported local variable creation.")}
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
                    _ => {i.op.span().unwrap().error("Unsupported binary expression.").emit(); panic!("Unsupported binary expression.")}
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
                    _ => {i.op.span().unwrap().error("Unsupported unary expression.").emit(); panic!("Unsupported unary expression.")}
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
                    panic!("Assigning to expression is not supported.");
                }
            }
            Expr::Lit(i) => {
                let lit = match i.lit {
                    Lit::Int(li) => li.to_string(),
                    Lit::Float(li) => li.to_string(),
                    _ => {i.lit.span().unwrap().error("Unsupported literal.").emit(); panic!("Unsupported literal.")}
                };
                self.current_arg = Arg::Item(lit.clone());
            }
            Expr::Path(i) => {
                let mut path = i.path.segments.last().unwrap().ident.to_string();
                if let Some(arg) = self.objects.get(&path) {
                    self.current_arg = arg.clone();
                } else {
                    if path == self.input_name {
                        path = "input".to_string();
                    }
                    self.current_arg = Arg::Item(path);
                }
            }
            //This has slightly diffent copies Path and Field, can we merge this?
            Expr::Reference(i) => {
                match *i.expr {
                    Expr::Path(j) => {
                        let mut path = j.path.segments.last().unwrap().ident.to_string();
                        if let Some(arg) = self.objects.get(&path) {
                            self.current_arg = arg.clone();
                        } else {
                            if path == self.input_name {
                                path = "input".to_string();
                            }
                            self.current_arg = Arg::Item("&".to_string() + &path);
                        }
                    }
                    Expr::Field(j) => {
                        let mut s = "&".to_string();
                        match *j.base {
                            Expr::Path(k) => {
                                s += &k.path.get_ident().unwrap().to_string();
                            }
                            _ => {j.member.span().unwrap().error("Unsupported field indexing.").emit(); panic!("Unsupported field indexing.")}
                        }
                        match j.member {
                            Member::Named(k) => {
                                s += &(".".to_string() + &k.to_string());
                            }
                            _ => {j.member.span().unwrap().error("Only name fields can be accessed.").emit(); panic!("Only name fields can be accessed.")}
                        }
                        self.current_arg = Arg::Item(s);
                    }
                    _ => {i.expr.span().unwrap().error("Unsupported reference.").emit(); panic!("Unsupported reference.")}
                }
            }
            Expr::Return(mut i) => {
                self.fold_expr(*i.clone().expr.unwrap());
                i.expr = Some(Box::new(self.compile_output(&*i.expr.unwrap())));
                ii = Expr::Return(i);
            }
            Expr::Field(i) => {
                let mut s = String::new();
                match *i.base {
                    Expr::Path(j) => {
                        s += &j.path.get_ident().unwrap().to_string();
                    }
                    _ => {i.member.span().unwrap().error("Unsupported field indexing.").emit(); panic!("Unsupported field indexing.")}
                }
                match i.member {
                    Member::Named(j) => {

                        s += &(".".to_string() + &j.to_string());
                    }
                    _ => {i.member.span().unwrap().error("Only name fields can be accessed.").emit(); panic!("Only name fields can be accessed.")}
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


            _ => {ii.span().unwrap().error("Unsupported expression.").emit(); panic!("Unsupported expression.")}
        }
        ii//fold::fold_expr(self, ii)
    }

}

#[derive(Debug, PartialEq, Clone)]
pub enum Arg {
    None,
    Operation(Box<Operation>),
    Item(String)
}

impl Arg {
    fn take(&mut self) -> Arg {
        mem::take(self)
    }

    pub fn to_tokenstream(&self) -> TokenStream {
        match self {
            Arg::None => panic!(),
            Arg::Operation(op) => op.to_tokenstream(),
            Arg::Item(i) => i.parse().unwrap()
        }
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

#[derive(Debug, PartialEq, Clone)]
pub struct Operation {
    pub receiver: Arg,
    pub method: String,
    pub args: Vec<Arg>
}

impl Operation {
    fn new(receiver: Arg, method: String, args: Vec<Arg>) -> Operation {
        Operation {
            receiver: receiver,
            method: method,
            args: args
        }
    }

    pub fn to_tokenstream(&self) -> TokenStream {
        if self.method == "add" {
            let rec = self.receiver.to_tokenstream();
            let arg = self.args[0].to_tokenstream();
            quote! {#rec+#arg}
        } else if self.method == "sub" {
            let rec = self.receiver.to_tokenstream();
            let arg = self.args[0].to_tokenstream();
            quote! {#rec-#arg}
        } else if self.method == "mul" {
            let rec = self.receiver.to_tokenstream();
            let arg = self.args[0].to_tokenstream();
            quote! {#rec*#arg}
        } else if self.method == "div" {
            let rec = self.receiver.to_tokenstream();
            let arg = self.args[0].to_tokenstream();
            quote! {#rec/#arg}
        } else if self.method == "neg" {
            let rec = self.receiver.to_tokenstream();
            quote! {-#rec}
        } else {
            let rec = self.receiver.to_tokenstream();
            let met: TokenStream = self.method.parse().unwrap();
            let args: Vec<TokenStream> = self.args.iter().map(|arg| arg.to_tokenstream()).collect();
            quote! {#rec.#met(#(#args),*)}
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