use soft65c02_tester::{CliDisplayer, Displayer, OutputToken};

use std::{
    io::{BufRead, ErrorKind, Write},
    sync::{mpsc::channel, Arc, Mutex},
    thread::spawn,
};

type IoResult<T> = std::io::Result<T>;

#[derive(Debug, Default)]
struct Buffer {
    data: Arc<Mutex<Vec<u8>>>,
}

impl Buffer {
    pub fn new(data: Arc<Mutex<Vec<u8>>>) -> Self {
        Self { data }
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let length = buf.len();
        self.data
            .lock()
            .map_err(|_| ErrorKind::Other)?
            .extend_from_slice(buf);

        Ok(length)
    }

    fn flush(&mut self) -> IoResult<()> {
        let mut lock = self.data.lock().map_err(|_| ErrorKind::Other)?;
        *lock = Vec::new();

        Ok(())
    }
}

#[test]
fn test_displayer() {
    let data = Arc::new(Mutex::new(Vec::new()));
    let join = {
        let (sender, receiver) = channel::<OutputToken>();
        let mut displayer = CliDisplayer::new(Buffer::new(data.clone()), false);
        let join = spawn(move || {
            displayer.display(receiver).unwrap();
        });
        sender
            .send(OutputToken::Marker {
                description: "test".to_string(),
            })
            .unwrap();
        sender
            .send(OutputToken::Assertion {
                failure: Some("this is a failure".to_string()),
                description: "assertion".to_string(),
            })
            .unwrap();

        join
    };
    join.join().unwrap();

    for (no, line) in data.lock().unwrap().lines().enumerate() {
        println!("line {no:02} - {}", line.unwrap());
    }
}
