extern crate raise_graph;

use raise_graph::graph;
use raise::tensor::Tensor;
use raise::layers::layer::Layer;

fn main() {
    let a = Test{
        input: None,
        weight: Tensor::rand([10, 10, 1, 1]),
        bias: Tensor::rand([1, 10, 1, 1])
    };
    let b = Test2{
        input: None,
    };
    let aout = a.forward(&Tensor::rand([1, 10, 1, 1]));
    let bout = b.forward(&aout);
}

#[derive(Clone)]
struct Test {
    input: Option<Tensor>,
    weight: Tensor,
    bias: Tensor
}

impl Layer for Test {

    #[graph]
    fn forward(&self, input: &Tensor) -> Tensor {
        //let a = -input.clamp_min(0.);
        &self.weight*input + &self.bias
    }

    fn take_input(&mut self) -> Tensor {
        self.input.take().unwrap()
    }

    fn set_input(&mut self, input: Tensor) {
        self.input = Some(input);
    }
}

#[derive(Clone)]
struct Test2 {
    input: Option<Tensor>,
}

impl Layer for Test2 {

    #[graph]
    fn forward(&self, input: &Tensor) -> Tensor {
         input.clone().clamp_min(0.)
    }

    fn take_input(&mut self) -> Tensor {
        self.input.take().unwrap()
    }

    fn set_input(&mut self, input: Tensor) {
        self.input = Some(input);
    }
}