
use raise::tensor::Tensor;
use std::ops::Add;
use quote::quote;
use proc_macro2::TokenStream;

//fn a_diff(function_ident: &str)

fn add_diff(function_ident: &str, diff_func: fn(receiver: String, arguments: Vec<String>) -> TokenStream) {
}

fn start() {
    add_diff("add", |receiver, arguments| {
        quote! {
            #receiver + #(arguments[0])
        }
    });
    add_diff("sub", |receiver, arguments| {
        quote! {
            #receiver - #(arguments[0])
        }
    });
    add_diff("mul", |receiver, arguments| {
        quote! {
            (#receiver).transpose() - (#(arguments[0]).transpose())
        }
    });
    add_diff("neg", |receiver, arguments| {
        quote! {
            -#receiver;
        }
    });
    add_diff("clamp", |receiver, arguments| {
        quote! {
            receiver.is_between_(#(arguments[0]), #(arguments[1]))
        }
    });
    add_diff("clamp_min", |receiver, arguments| {
        quote! {
            receiver.is_bigger_(#(arguments[0]))
        }
    });
    add_diff("clamp_max", |receiver, arguments| {
        quote! {
            receiver.is_smaller_(#(arguments[0]))
        }
    });
}

//fn t() -> dyn Fn(f32) -> Tensor {
//    Tensor::add
//}