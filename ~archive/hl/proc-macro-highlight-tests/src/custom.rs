use proc_macro_highlight::MyProcMacro;

#[derive(MyProcMacro)]
struct MyStruct2 {
    #[my_attr(other_fn = "some_fn")]
    _x: String,
    // #[my_attr(other_fn = "nested::another_fn")]
    _y: String
}

fn some_fn(input: u8) -> u8 {
    input
}

trait MyTrait {
    fn other_fn(input: u8) -> u8;
}

mod nested {
    pub fn another_fn(input: u8) -> u8 {
        input
    }
}