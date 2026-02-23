
use std::path::Path;

fn main() {
    let p1 = Path::new("/bin");
    let p2 = Path::new("/bin/");
    let p3 = Path::new("//bin");

    println!("/bin == /bin/ : {}", p1 == p2);
    println!("/bin == //bin : {}", p1 == p3);

    // Components
    println!("/bin components: {:?}", p1.components().collect::<Vec<_>>());
    println!("/bin/ components: {:?}", p2.components().collect::<Vec<_>>());
    println!("//bin components: {:?}", p3.components().collect::<Vec<_>>());
}
