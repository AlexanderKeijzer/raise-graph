
use raise::tensor::Tensor;
use std::ops::Add;

//fn a_diff(function_ident: &str)

fn add_diff(function_ident: &str, diff: fn(Tensor, Tensor) -> Tensor) {
}

fn start() {
    add_diff("add", |_, to| {
        to
    });
    add_diff("sub", |_, to| {
        -to
    });
    add_diff("mul", |with, to| {
        &with.transpose()*&to
    });
    add_diff("div", |with, to| {
        with/to //Transpose...
    });
    add_diff("neg", |_, to| {
        -to
    });
    /*
    add_diff("clamp", |_, to| {
        //to.is_between_()
    });
    add_diff("clamp_min", |_, to| {
        //to.is_bigger_()
    });
    add_diff("clamp_max", |_, to| {
        //to.is_smaller_()
    });
    */
}

//fn t() -> dyn Fn(f32) -> Tensor {
//    Tensor::add
//}