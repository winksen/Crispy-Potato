fn main() {
    let name = String::from("world");
    greet(&name); // borrow `name` instead of moving it
    println!("{name} is still usable here");
}

fn greet(name: &str) {
    println!("Hello, {name}!");
}
