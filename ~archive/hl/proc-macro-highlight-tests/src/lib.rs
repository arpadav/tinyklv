mod clap;
mod serde;
mod custom;

use proc_macro_highlight::MyProcMacro;
use proc_macro_highlight::MyProcMacroNested;

#[derive(MyProcMacro)]
struct MyStruct3 {
    #[my_attr(other_fn = "some_fn")]
    _x: String,
}

#[derive(MyProcMacro)]
struct MyStruct4 {
    #[my_attr(other_fn = "nested::another_fn")]
    _x: String,
}

fn some_fn(input: u8) -> u8 {
    input
}
fn some2_fn(input: u8) -> u8 {
    input
}

mod nested {
    pub fn another_fn(input: u8) -> u8 {
        input
    }
}

#[derive(MyProcMacroNested)]
struct MyStruct5 {
    #[my_attr(
        other_fn = "some2_fn",
        nested(
            one_fn = some_fn,
            two_fn = nested::another_fn
        ),
        allow_me,
        allow_another,
    )]
    _x: String,
}

trait MyTrait {
    fn other_fn(input: u8) -> u8;
}

trait MyTraitNested {
    fn one_fn(input: u8) -> u8;
    fn two_fn(input: u8) -> u8;
}