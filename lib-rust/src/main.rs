use hypetrigger::pipeline_simple::Hypetrigger;

fn main() {
    println!("Hello world!");
    Hypetrigger::new().set_input("test.mp4".to_string()).run();
}
