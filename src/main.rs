use std::fs::File;
use std::sync::mpsc;
use std::io::BufReader;
use std::time::Duration;
use std::time::Instant;
use iced::{button, executor, futures, Align, Application, Button, Column, Command, Clipboard, Element, Font, HorizontalAlignment, Length, Row, Settings, Subscription, Text};
use rodio::{Decoder, OutputStream, source::Source, Sink};

const FPS: u64 = 30;
const MILLISEC: u64 = 1000;
const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;

const FONT: Font = Font::External{
    name: "PixelMplus12-Regular",
    bytes: include_bytes!("../rsc/PixelMplus12-Regular.ttf"),
};


enum MusicPlayerMessage {
    Quit,
    Initialize(String),
    Play,
    Stop,
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
        let mut loop_flag = true;
        while loop_flag {
            let ret = rx.recv_timeout(Duration::from_millis(10));
            match ret {
                Ok(msg) => {
                    match msg {
                        MusicPlayerMessage::Quit => {
                            loop_flag = false
                        }
                        MusicPlayerMessage::Initialize(path) => {
                            self.initialize(path)
                        }
                        MusicPlayerMessage::Play => {
                            self.play()
                        }
                        MusicPlayerMessage::Stop => {
                            self.stop()
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
    pub fn stop(&mut self) {
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

struct MusicPlayerActor {
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

    pub fn quit(&mut self) {
        self.send_message(MusicPlayerMessage::Quit);
    }
    pub fn initialize(&mut self, path: String) {
        self.send_message(MusicPlayerMessage::Initialize(path));
    }
    pub fn play(&mut self) {
        self.send_message(MusicPlayerMessage::Play);
    }
    pub fn stop(&mut self) {
        self.send_message(MusicPlayerMessage::Stop);
    }
}

pub struct Timer {
    duration: Duration,
}

impl Timer {
    fn new(duration: Duration) -> Timer {
        Timer { duration: duration }
    }
}

impl<H, E> iced_native::subscription::Recipe<H, E> for Timer
where H: std::hash::Hasher {
    type Output = Instant;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;
        std::any::TypeId::of::<Self>().hash(state);
        self.duration.hash(state)
     }
    fn stream(self: std::boxed::Box<Self>, _input: futures::stream::BoxStream<'static, E>) -> futures::stream::BoxStream<'static, Self::Output> {
        use futures::stream::StreamExt;
        async_std::stream::interval(self.duration)
            .map(|_| Instant::now())
            .boxed()
     }
}

#[derive(Debug, Clone)]
pub enum Message {
    Start,
    Stop,
    AlertStop,
    Reset,
    Update,
    OneMinute,
    TenMinute,
}

pub enum TickState {
    Stopped,
    Ticking,
    Alert,
}

struct GUI {
    last_update: Instant,
    total_duration: Duration,
    tick_state: TickState,
    start_stop_button_state: button::State,
    reset_button_state: button::State,
    one_minute_button_state: button::State,
    ten_minute_button_state: button::State,
}

impl Application for GUI {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flaggs: ()) -> (GUI, Command<Self::Message>) {
        (GUI {  
            last_update: Instant::now(),
            total_duration: Duration::default(),
            tick_state: TickState::Stopped,
            start_stop_button_state: button::State::new(),
            reset_button_state: button::State::new(),
            one_minute_button_state: button::State::new(),
            ten_minute_button_state: button::State::new(),
        },
        Command::none())
    }

    fn title(&self) -> String {
        String::from("WorkTimer")
    }

    fn update(&mut self, message: Self::Message, _clipboard: &mut Clipboard) -> Command<Self::Message> {
        match message {
            Message::Start => {
                self.tick_state = TickState::Ticking;
                self.last_update = Instant::now();
            }
            Message::Stop => {
                self.tick_state = TickState::Stopped;

                let now_update = Instant::now();
                let diff_duration = now_update - self.last_update;
                let is_time_out = self.total_duration <= diff_duration;
                self.total_duration = if is_time_out {
                    Duration::default()
                } else {
                    self.total_duration - diff_duration
                };
            }
            Message::AlertStop => {
                self.tick_state = TickState::Stopped;
                self.last_update = Instant::now();
            }
            Message::Reset => match self.tick_state {
                TickState::Stopped => {
                    self.last_update = Instant::now();
                    self.total_duration = Duration::default();
                }
                _ => {}
            }
            Message::Update => match self.tick_state {
                TickState::Ticking => {
                    let now_update = Instant::now();
                    let diff_duration = now_update - self.last_update;
                    let is_time_out = self.total_duration <= diff_duration;
                    self.total_duration = if is_time_out {
                        Duration::default()
                    } else {
                        self.total_duration - diff_duration
                    };
                    self.last_update = now_update;
                    if is_time_out {
                        self.tick_state = TickState::Alert;
                    }

                }
                _ => {}
            }
            Message::OneMinute => {
                self.total_duration += Duration::from_secs(1);
            }
            Message::TenMinute => {
                self.total_duration += Duration::from_secs(10);
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        let seconds = self.total_duration.as_secs();

        let duration_text = format!(
            "{:0>2}:{:0>2}:{:0>2}.{:0>2}",
            seconds / HOUR,
            (seconds % HOUR) / MINUTE,
            seconds % MINUTE,
            self.total_duration.subsec_millis() / 10
        );
        let start_stop_string = match self.tick_state {
            TickState::Stopped => "Start",
            TickState::Ticking => "Stop",
            TickState::Alert => "Alert",
        };
        let start_stop_text = Text::new(start_stop_string)
                .horizontal_alignment(HorizontalAlignment::Center)
                .font(FONT);
        let start_stop_message = match self.tick_state {
            TickState::Stopped => Message::Start,
            TickState::Ticking => Message::Stop,
            TickState::Alert => Message::AlertStop,
        };

        let tick_text = Text::new(duration_text).font(FONT).size(60);

        let start_stop_button = Button::new(&mut self.start_stop_button_state, start_stop_text)
            .min_width(80)
            .on_press(start_stop_message);

            let reset_button = Button::new(
            &mut self.reset_button_state,
            Text::new("Reset")
                .horizontal_alignment(HorizontalAlignment::Center)
                .font(FONT)
        )
            .min_width(80)
            .on_press(Message::Reset);

        let one_minute_button = Button::new(
            &mut self.one_minute_button_state,
             Text::new("1m.")
                .horizontal_alignment(HorizontalAlignment::Center)
                .font(FONT)
        )
            .min_width(40)
            .on_press(Message::OneMinute);

            let ten_minute_button = Button::new(
                &mut self.ten_minute_button_state,
                 Text::new("10m.")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .font(FONT)
            )
            .min_width(40)
            .on_press(Message::TenMinute);

        Column::new()
            .push(tick_text)
            .push(
                Row::new()
                    .push(one_minute_button)
                    .push(ten_minute_button)
                    .push(start_stop_button)
                    .push(reset_button)
                    .spacing(10),
            )
            .spacing(10)
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Align::Center)
            .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        let timer = Timer::new(Duration::from_millis(MILLISEC / FPS));
        iced::Subscription::from_recipe(timer).map(|_| Message::Update)
    }
}


fn main() {
    let mut player = MusicPlayerActor::new();
    player.start();
    player.initialize("alert.mp3".to_string());
    player.play();
    std::thread::sleep(Duration::from_secs(1));
    player.stop();
    std::thread::sleep(Duration::from_secs(1));
    player.play();
    std::thread::sleep(Duration::from_secs(1));
    player.stop();
    std::thread::sleep(Duration::from_secs(1));
    player.play();
    std::thread::sleep(Duration::from_secs(1));
    player.stop();
    std::thread::sleep(Duration::from_secs(1));

    let mut settings = Settings::default();
    settings.window.size = (400u32, 120u32);
    let _result = GUI::run(settings);
}
