use proc_macro2::TokenStream;
use std::collections::HashMap;
use quote::quote;

static ARRGUMENT_NAMES: &[&str] = &["{ a }", "{ b }", "{ c }", "{ d }", "{ e }", "{ f }", "{ g }"];
static OUTPUT_NAMES: &[&str] = &["a", "b", "c", "d", "e", "f", "g"];

pub struct AutoDiff {
    map: HashMap<String, Vec<(TokenStream, Vec<u8>)>>
}

impl AutoDiff {

    pub fn new() -> AutoDiff {
        let mut s = AutoDiff {
            map: HashMap::new()
        };
        s.init();
        s
    }

    pub fn add_diff(&mut self, method: String, expressions: Vec<TokenStream>) {
        let map_entries: Vec<(TokenStream, Vec<u8>)> = expressions.iter().map(|ts| {
            let mut s = ts.to_string();
            let mut contains_var: Vec<u8> = Vec::new();
            for i in 0..ARRGUMENT_NAMES.len() {
                if s.contains(ARRGUMENT_NAMES[i]) {
                    s = s.replace(ARRGUMENT_NAMES[i], OUTPUT_NAMES[i]);
                    contains_var.push(i as u8);
                }
            }
            (s.parse().unwrap(), contains_var)
        }).collect();
            
        self.map.insert(method, map_entries);
    }

    pub fn get_expressions(&self, method: &str) -> &Vec<(TokenStream, Vec<u8>)> {
        self.map.get(method).expect(&format!("No diff found for function {}", method))
    }

    pub fn init(&mut self) {

        macro_rules! add_diff {
            ($owner:path, $func:literal, $($diff:expr),* ) => {
                {
                    let mut expressions: Vec<proc_macro2::TokenStream> = Vec::new();
                    $(
                        expressions.push( quote! {
                            $diff
                        });
                    )*
                    $owner.add_diff($func.to_string(), expressions);
                }
            };
        }

        //We should resolve type and accept function paths instead, but for now this works
        add_diff!(self, "add", grad, grad);
        add_diff!(self, "sub", grad, -grad);
        add_diff!(self, "mul", grad*{b}.transpose(), {a}.transpose()*grad);
        add_diff!(self, "div", grad/{b}, -(grad*{a})/({b}.powi(2)));
        add_diff!(self, "neg", -grad);
        add_diff!(self, "sin", grad*{a}.cos());
        add_diff!(self, "cos", grad*(-{a}.sin()));
        add_diff!(self, "tan", grad/({a}.cos()).powi(2));
        add_diff!(self, "sinh", grad*{a}.cosh());
        add_diff!(self, "cosh", grad*{a}.sinh());
        add_diff!(self, "tanh", grad/({a}.cosh()).powi(2));
        add_diff!(self, "exp", grad*&{a}.exp());
        add_diff!(self, "ln", grad/{a});
        add_diff!(self, "clamp", grad*(&{a}.is_between({b}, {c})));
        add_diff!(self, "clamp_min", grad*&{a}.is_bigger({b}));
        add_diff!(self, "clamp_max", grad*&{a}.is_smaller({b}));
        add_diff!(self, "clone", grad);
    }
}