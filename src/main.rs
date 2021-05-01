#![windows_subsystem = "windows"]

use iced::Slider;
use std::time::Duration;
use std::time::Instant;
use iced::{button, executor, futures, slider, Align, Application, Button, Column, Command, Clipboard, Element, Font, HorizontalAlignment, Length, Row, Settings, Subscription, Text};
mod music_player;
use music_player::MusicPlayerActor;

const FPS: u64 = 30;
const MILLISEC: u64 = 1000;
const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;

const FONT: Font = Font::External{
    name: "PixelMplus12-Regular",
    bytes: include_bytes!("../rsc/PixelMplus12-Regular.ttf"),
};

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
    BGMVolumeChange(f32),
}

#[derive(PartialEq, Debug, Clone)]
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
    alert_player: MusicPlayerActor,
    bgm_player: MusicPlayerActor,
    bgm_volume_state: slider::State,
    bgm_volume_value: f32,
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
            alert_player: MusicPlayerActor::new_start_initialize("media/alert.mp3".to_string(), 1.0),
            bgm_volume_value: 100.0,
            bgm_player: MusicPlayerActor::new_start_initialize("media/bgm.mp3".to_string(), 1.0),
            bgm_volume_state: slider::State::new(),
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
                self.bgm_player.play();

                self.last_update = Instant::now();
            }
            Message::Stop => {
                self.tick_state = TickState::Stopped;
                self.bgm_player.pause();
                self.alert_player.pause();

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
                self.bgm_player.pause();
                self.alert_player.pause();

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
                        self.bgm_player.pause();
                        self.alert_player.play();
                    }

                }
                _ => {}
            }
            Message::OneMinute => {
                self.total_duration += Duration::from_secs(MINUTE);
            }
            Message::TenMinute => {
                self.total_duration += Duration::from_secs(10*MINUTE);
            }
            Message::BGMVolumeChange(volume) => {
                self.bgm_volume_value = volume;
                self.bgm_player.change_volume(volume / 100.0);
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

        let bgm_volume_slider = Slider::new(&mut self.bgm_volume_state, 0.0..=100.0, self.bgm_volume_value, Message::BGMVolumeChange);

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
            .push(bgm_volume_slider)
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
    let mut settings = Settings::default();
    settings.window.size = (400u32, 160u32);
    let _result = GUI::run(settings);
}
