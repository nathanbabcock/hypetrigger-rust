use hypetrigger::pipeline_simple::Hypetrigger;

fn main() {
    println!("Hello world!");
    match Hypetrigger::new().set_input("test.mp4".to_string()).run() {
        Ok(_) => println!("[main] done"),
        Err(e) => eprintln!("[main] error: {:?}", e),
    }
}
