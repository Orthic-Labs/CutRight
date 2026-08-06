use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let values: Vec<String> = serde_json::from_str(&input).unwrap();
    let polished: Vec<String> = values
        .iter()
        .map(|value| heardright_core::text_pipeline::deterministic_polish(value))
        .collect();
    println!("{}", serde_json::to_string(&polished).unwrap());
}
