use std::fs::File;
use std::sync::mpsc;
use std::io::BufReader;
use std::time::Duration;
use rodio::{Decoder, OutputStream, source::Source, Sink};

enum MusicPlayerMessage {
    Initialize(String),
    Play,
    Pause,
}

enum MusicPlayerState {
    UninitializeState,
    StopState,
    PlayState,
}

struct MusicPlayer {
    state: MusicPlayerState,
    stream: Option<rodio::OutputStream>,
    sink: Option<rodio::Sink>,
}

impl MusicPlayer {
    pub fn new() -> MusicPlayer {
        MusicPlayer {
            state: MusicPlayerState::UninitializeState,
            stream: None,
            sink: None,
        }
    }
    pub fn run(&mut self, rx : mpsc::Receiver<MusicPlayerMessage>) {
        loop {
            let ret = rx.recv_timeout(Duration::from_millis(10));
            match ret {
                Ok(msg) => {
                    match msg {
                        MusicPlayerMessage::Initialize(path) => {
                            self.initialize(path)
                        }
                        MusicPlayerMessage::Play => {
                            self.play()
                        }
                        MusicPlayerMessage::Pause => {
                            self.pause()
                        }
                    }
                }
                Err(_) => {
                }
            }
        }
    }
    pub fn initialize(&mut self, path: String) {
        {
            let (stream, stream_handle) = OutputStream::try_default().unwrap();
            self.stream = Option::from(stream);
            let sink = Sink::try_new(&stream_handle).unwrap();
            let file = BufReader::new(File::open(path).unwrap());
            let source = Decoder::new(file).unwrap().repeat_infinite();
            sink.append(source);
            sink.pause();

            self.state = MusicPlayerState::StopState;
            self.sink = Option::from(sink);
        }
    }
    pub fn play(&mut self) {
        match self.state {
            MusicPlayerState::StopState => {
                if let Some(s) = &self.sink {
                    s.play();
                }
                self.state = MusicPlayerState::PlayState
            }
            _ => {}
        }
    }
    pub fn pause(&mut self) {
        match self.state {
            MusicPlayerState::PlayState => {
                if let Some(s) = &self.sink {
                    s.pause();
                }
                self.state = MusicPlayerState::StopState
            }
            _ => {}
        }
    }
}

pub struct MusicPlayerActor {
    thread: Option<std::thread::JoinHandle<()>>,
    tx: Option<mpsc::Sender<MusicPlayerMessage>>,
}

impl MusicPlayerActor {
    pub fn new() -> MusicPlayerActor {
        MusicPlayerActor {
            thread : None,
            tx : None,
        }
    }

    pub fn new_start_initialize(path:String) -> MusicPlayerActor {
        let mut obj = MusicPlayerActor::new();
        obj.start();
        obj.initialize(path);
        obj
    }

    pub fn start(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.tx = Option::from(tx);
        self.thread = Option::from(std::thread::spawn(move || MusicPlayer::new().run(rx)));
    }

    fn send_message(&mut self, msg: MusicPlayerMessage) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(msg);
        }
    }

    pub fn initialize(&mut self, path: String) {
        self.send_message(MusicPlayerMessage::Initialize(path));
    }
    pub fn play(&mut self) {
        self.send_message(MusicPlayerMessage::Play);
    }
    pub fn pause(&mut self) {
        self.send_message(MusicPlayerMessage::Pause);
    }
}
