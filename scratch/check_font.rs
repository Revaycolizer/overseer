use std::path::Path;

fn main() {
    let p = Path::new("assets/fonts/Roboto-Regular.ttf");
    println!("Exists: {}", p.exists());
    if let Some(parent) = p.parent() {
        println!("Parent: {:?}", parent);
    }
    if let Some(stem) = p.file_stem() {
        println!("Stem: {:?}", stem);
    }
}
