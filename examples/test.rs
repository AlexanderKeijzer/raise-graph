extern crate raise_graph;

use raise_graph::graph;

fn testje(la: i32) {
    let mut a = -(1 + 1) * 2 / 4;
    let b = a * a;
    let c;
    c = 2;
    let f = c*a;
    return;
}

fn main() {
    testje(1);
    let mut a = test{
        weight: 2.,
        bias: 1.
    };
    a.testje(2.);
}

struct test {
    weight: f32,
    bias: f32
}

trait Lay {
    fn testje(&mut self, input: f32) -> f32;

    fn backward(&mut self, input: f32, output_grad: f32) -> f32;
}

impl Lay for test {

    #[graph]
    fn testje(&mut self, input: f32) -> f32 {
        let a = input.exp();
        self.weight*input + self.bias
    }

    fn backward(&mut self, input: f32, output_grad: f32) -> f32 {
        input
    }
}