use win32thread::ProcessThread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = crossbeam_channel::bounded(10);

    let mut sensor = ProcessThread::new(Some(tx));
    std::thread::spawn(move || {
        sensor.run().expect("unable to run sensor");
    });

    loop {
        if let Ok(thread) = rx.try_recv() {
            println!("{thread:#?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
