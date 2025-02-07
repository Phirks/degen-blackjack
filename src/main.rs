use std::{io, ops::Index, sync::mpsc, thread, time::Duration, vec};
use rand::{distr::{Distribution, StandardUniform}, seq, Rng};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    prelude::{Buffer, Rect},
    style::Stylize,
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget},
    DefaultTerminal, Frame,
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    // Create the channel via which the events will be sent to the main app.
    let (event_tx, event_rx) = mpsc::channel::<Event>();

    // Thread to listen for input events.
    let tx_to_input_events = event_tx.clone();
    thread::spawn(move || {
        handle_input_events(tx_to_input_events);
    });

    // Thread that does a computational heavy task.
    // If you would like to communicate to the task, i.e. start/stop/pause the process,
    // a second channel is required.
    let tx_to_background_progress_events = event_tx.clone();
    thread::spawn(move || {
        run_background_thread(tx_to_background_progress_events);
    });

    let mut app = App {
        exit: false,
        players: vec!(),
        dealer_hand: Hand::new(),
        active_hand_index: (0,0),
    };

    // App runs on the main thread.
    let app_result = app.run(&mut terminal, event_rx);

    // Note: If your threads need clean-up (i.e. the computation thread),
    // you should communicatie to them that the app wants to shut down.
    // This is not required here, as our threads don't use resources.
    ratatui::restore();
    app_result
}

// Events that can be sent to the main thread.
enum Event {
    Input(crossterm::event::KeyEvent), // crossterm key input event
    //Progress(f64),                     // progress update from the computation thread
}

#[derive(PartialEq)]
enum Outcome {
    NotFinished,
    Stand,
    DealerWins(u8,u8),
    DealerBusts(u8,u8),
    PlayerWins(u8,u8),
    PlayerBusts(u8,u8),
    DealerBlackjack(u8),
    PlayerBlackjack(u8),
    Push(u8)
}

impl Outcome {
    fn display_string(&self) -> String {
        match self {
            Outcome::DealerBusts(d,p) => format!("Dealer busts with {}, you win with {}!",d.to_string(),p.to_string()),
            Outcome::DealerWins(d,p) => format!("Dealer wins with {} vs your {}",d.to_string(),p.to_string()),
            Outcome::PlayerBusts(d,p) => format!("You busted with {}, dealer wins with {}",p.to_string(),d.to_string()),
            Outcome::PlayerWins(d,p) => format!("You win with {} vs the dealer's {}!",p.to_string(),d.to_string()),
            Outcome::DealerBlackjack(p) => format!("Dealer got a blackjack vs your {}, dealer wins",p.to_string()),
            Outcome::PlayerBlackjack(d) => format!("You got a blackjack vs dealer's {}, you win!",d.to_string()),
            Outcome::Push(t) => format!("Push, both you and the dealer have {}",t.to_string()),
            Outcome::NotFinished=> "".to_string(),
            Outcome::Stand => "".to_string(),
        }
    }
}

pub struct App {
    exit: bool,
    players: Vec<Player>,
    dealer_hand: Hand,
    active_hand_index: (usize,usize)
}

#[derive(PartialEq)]
pub struct Player {
    hands: Vec<Hand>,
    name: String,
    bank: f64,
}

impl Player {
    fn new() -> Player {
        Player {
            hands: vec!(),
            name: "".to_string(),
            bank: 0.0,
        }
    }
}

/// Block, waiting for input events from the user.
fn handle_input_events(tx: mpsc::Sender<Event>) {
    loop {
        match crossterm::event::read().unwrap() {
            crossterm::event::Event::Key(key_event) => tx.send(Event::Input(key_event)).unwrap(),
            _ => {}
        }
    }
}

/// Simulate a computational heavy task.
fn run_background_thread(tx: mpsc::Sender<Event>) {
    let mut progress = 0_f64;
    let increment = 0.01_f64;
    loop {
        thread::sleep(Duration::from_millis(100));
        progress += increment;
        progress = progress.min(1_f64);
        //tx.send(Event::Progress(progress)).unwrap();
    }
}

#[derive(PartialEq)]
struct Hand {
    contains: Vec<Card>,
    number_of_aces: u8,
    value: u8,
    bet: f64,
    outcome: Outcome,
}

impl Hand {
    fn new() -> Hand {
        Hand{
            contains: vec!(),
            number_of_aces: 0,
            value: 0,
            bet: 0.0,
            outcome: Outcome::NotFinished,
        }
    }
    fn add_card(&mut self){
        self.contains.push(Card::new());
        self.value=self.get_value();
        self.number_of_aces=self.get_number_of_aces();
    }
    fn add_hidden_card(&mut self){
        self.contains.push(Card::new_hidden());
    }
    fn get_value(&self) -> u8{
        let mut value = 0;
        for card in &self.contains {
            value+=card.numerical_value();

        }
        value
    }
    fn get_real_value(&self) -> (u8,bool) {
        let mut value = self.value;
        let mut aces = self.number_of_aces;
        let mut soft = false;
        while value>21 && aces>0{
            aces-=1;
            value-=10;
        }
        if aces>0{soft=true}
        (value,soft)
    }
    fn get_number_of_aces(&self) -> u8 {
        let mut aces = 0;
            for card in &self.contains {
                if card.value == CardValue::Ace {
                    aces += 1;
                }
            }
        aces
    }
    fn value_string(&self) -> String {
        let (value,soft) = self.get_real_value();
        let mut soft_string = "";
        if soft {soft_string = "a soft "}
        format!("{}{}",soft_string,value.to_string())
    }
}


#[derive(PartialEq)]
struct Card {
    value: CardValue,
    suit: CardSuit,
}

impl Card {
    fn new() -> Card {
        Card {
            value: rand::random(),
            suit: rand::random(),
        }
    }
    fn new_hidden() -> Card {
        Card {
            value: CardValue::Hidden,
            suit: CardSuit::Hidden,
        }
    }
    fn flip_card(&mut self) {
        self.value = rand::random();
        self.suit = rand::random();
    }
    fn to_paragraph(&self) -> Paragraph {
        let value_string = match self.value {
            CardValue::Two => Line::from("  2"),
            CardValue::Three => Line::from("  3"),
            CardValue::Four => Line::from("  4"),
            CardValue::Five => Line::from("  5"),
            CardValue::Six => Line::from("  6"),
            CardValue::Seven => Line::from("  7"),
            CardValue::Eight => Line::from("  8"),
            CardValue::Nine => Line::from("  9"),
            CardValue::Ten => Line::from(" 10"),
            CardValue::Jack => Line::from("  J"),
            CardValue::Queen => Line::from("  Q"),
            CardValue::King => Line::from("  K"),
            CardValue::Ace => Line::from("  A"),
            CardValue::Hidden => Line::from(""),

        };
        let suit_string = match self.suit {
            CardSuit::Heart => Line::from(" ♥ ").red(),
            CardSuit::Spade => Line::from(" ♠ ").black(),
            CardSuit::Club => Line::from(" ♣ ").black(),
            CardSuit::Diamond => Line::from(" ♦ ").red(),
            CardSuit::Hidden => Line::from(""),
        };
        Paragraph::new(vec![suit_string,value_string]).block(Block::bordered().border_type(BorderType::Rounded))
    }
    fn numerical_value(&self) -> u8{
        match self.value {
            CardValue::Two => 2,
            CardValue::Three => 3,
            CardValue::Four => 4,
            CardValue::Five => 5,
            CardValue::Six => 6,
            CardValue::Seven => 7,
            CardValue::Eight => 8,
            CardValue::Nine => 9,
            CardValue::Ten => 10,
            CardValue::Jack => 10,
            CardValue::Queen => 10,
            CardValue::King => 10,
            CardValue::Ace => 11,
            CardValue::Hidden => 0,
        }
    }

    fn render_card(&self ,x: u16, y: u16, buf: &mut Buffer) {
        if self.value==CardValue::Hidden{
            Paragraph::new(vec![" ░▒".into()," ▒░".into()])
                .block(Block::bordered())
                .render(Rect::new(0, 1, 6, 4), buf);

        } else {
            self.to_paragraph().render(Rect::new(x,y, 6, 4), buf);
        }
    }
}

#[derive(PartialEq)]
enum CardSuit {
    Heart,
    Spade,
    Club,
    Diamond,
    Hidden,
}

impl Distribution<CardSuit> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CardSuit {
        let index: u8 = rng.random_range(0..=3);
        match index {
            0 => CardSuit::Heart,
            1 => CardSuit::Spade,
            2 => CardSuit::Club,
            3 => CardSuit::Diamond,
            _ => unreachable!(),
        }
    }

}

#[derive(PartialEq)]
enum CardValue {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
    Hidden,
}

impl Distribution<CardValue> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CardValue {
        let index: u8 = rng.random_range(0..=12);
        match index {
            0 => CardValue::Two,
            1 => CardValue::Three,
            2 => CardValue::Four,
            3 => CardValue::Five,
            4 => CardValue::Six,
            5 => CardValue::Seven,
            6 => CardValue::Eight,
            7 => CardValue::Nine,
            8 => CardValue::Ten,
            9 => CardValue::Jack,
            10 => CardValue::Queen,
            11 => CardValue::King,
            12 => CardValue::Ace,
            _ => unreachable!(),
        }
    }

}

impl App {
    /// Main task to be run continuously
    fn run(&mut self, terminal: &mut DefaultTerminal, rx: mpsc::Receiver<Event>) -> io::Result<()> {
        self.initialize();
        self.reset();
        terminal.draw(|frame| self.draw(frame))?;
        while !self.exit {
            match rx.recv().unwrap() {
                Event::Input(key_event) => self.handle_key_event(key_event)?,
                //Event::Progress(progress) => self.background_progress = progress,
            }
            terminal.draw(|frame| self.draw(frame))?;
        }
        Ok(())
    }

    fn initialize(&mut self) {
        self.players.push(Player::new());
        self.players[0].name = "Nick".to_string();
        self.players[0].bank = 100.0;
    }

    fn reset(&mut self){
        self.active_hand_index = (0,0);
        for player in &mut self.players{
            player.hands=vec!();
        }
        for player in &mut self.players{
            player.hands.push(Hand::new());
            player.hands.push(Hand::new());
        }
        for player in &mut self.players{
            for hand in &mut player.hands{
                hand.bet += 5.0;
                player.bank -= 5.0;
            }
        }
        for player in &mut self.players{
            for hand in &mut player.hands{
                hand.add_card();
            }
        }
        self.dealer_hand = Hand::new();
        self.dealer_hand.add_hidden_card();
        for player in &mut self.players{
            for hand in &mut player.hands{
                hand.add_card();
            }
        }
        self.dealer_hand.add_card();
    }
    /// Render `self`, as we implemented the Widget trait for &App
    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn hit(&mut self){
        let (player_index,hand_index) = self.active_hand_index;
        let hand = &mut self.players[player_index].hands[hand_index];
        if hand.outcome == Outcome::NotFinished{
            if hand.get_real_value().0<21{
                hand.add_card();
            }
        }
    }

    fn next_hand(&mut self) {
        let (mut player_index,mut hand_index) = &mut self.active_hand_index;
        if self.players[player_index].hands.len()-1>hand_index{
           hand_index += 1; 
        } else {
            if self.players.len()-1>player_index{
                player_index += 1;
            }
        }
        self.active_hand_index = (player_index,hand_index);
    }

    fn stay(&mut self){
        let (player_index,hand_index) = self.active_hand_index;
        let hand = &mut self.players[player_index].hands[hand_index];
        if hand.outcome==Outcome::NotFinished{
            hand.outcome=Outcome::Stand;
        }
        self.next_hand();
        let mut finished = true;
        for player in &self.players{
            for hand in &player.hands{
                if hand.outcome == Outcome::NotFinished{
                    finished=false;
                }
            }
        }
        if finished {
            self.end();
        }
    }

    fn end(&mut self){
        if self.dealer_hand.contains[0].value == CardValue::Hidden{
            self.dealer_hand.contains[0].flip_card();
            self.dealer_hand.value = self.dealer_hand.get_value();
        }
            while self.dealer_hand.get_real_value().0<17{
                self.dealer_hand.add_card();
            }
        for player in &mut self.players{
            for hand in &mut player.hands{
                if hand.get_real_value().0>21 {
                    hand.outcome = Outcome::PlayerBusts(self.dealer_hand.get_real_value().0,hand.get_real_value().0);
                    hand.bet = 0.0;
                } else if self.dealer_hand.get_real_value().0>21 {
                    hand.outcome = Outcome::DealerBusts(self.dealer_hand.get_real_value().0,hand.get_real_value().0);
                    player.bank += hand.bet*2.0;
                    hand.bet = 0.0;
                } else if hand.get_real_value().0 < self.dealer_hand.get_real_value().0{
                    hand.outcome = Outcome::DealerWins(self.dealer_hand.get_real_value().0,hand.get_real_value().0);
                    hand.bet = 0.0;
                } else if hand.get_real_value().0 > self.dealer_hand.get_real_value().0{
                    hand.outcome = Outcome::PlayerWins(self.dealer_hand.get_real_value().0,hand.get_real_value().0);
                    player.bank += hand.bet*2.0;
                    hand.bet=0.0;
                } else if self.dealer_hand.get_real_value().0 == hand.get_real_value().0{
                    hand.outcome = Outcome::Push(self.dealer_hand.get_real_value().0);
                    player.bank += hand.bet;
                    hand.bet=0.0;
                }
            }
        }
    }

    /// Actions that should be taken when a key event comes in.
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        if key_event.kind == KeyEventKind::Press &&  key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('c') {
            self.exit = true;
        } else if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('h') {
            self.hit();
        } else if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('s') {
            self.stay();
        } else if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('r') {
            self.reset();
        }
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Prepare the widgets for the bottom part of the layout.
        // Block to be displayed around the progress bar.

        let mut pos = 0;
        let (player_index,hand_index) = self.active_hand_index;
        let active_player_hand = &self.players[player_index].hands[hand_index];
        for player in &self.players{
            for hand in &player.hands{
                for card in &hand.contains {
                    card.render_card(pos,area.height-6,buf);
                    pos+= 6;
                }
            pos+=2;
            }
        }
        if self.dealer_hand.contains[0].value==CardValue::Hidden{
            pos = 0;
            for card in &self.dealer_hand.contains {
                card.render_card(pos, 1, buf);
                pos+=6
            }
            if self.dealer_hand.contains[0].value == CardValue::Ace{
                Line::from(format!("Dealer shows an Ace")).render(Rect::new(0, 0, area.width, 1), buf);
            } else if self.dealer_hand.contains[0].value == CardValue::King {
                Line::from(format!("Dealer shows a King")).render(Rect::new(0, 0, area.width, 1), buf);
            } else if self.dealer_hand.contains[0].value == CardValue::Queen {
                Line::from(format!("Dealer shows a Queen")).render(Rect::new(0, 0, area.width, 1), buf);
            } else if self.dealer_hand.contains[0].value == CardValue::Jack {
                Line::from(format!("Dealer shows a Jack")).render(Rect::new(0, 0, area.width, 1), buf);
            } else if self.dealer_hand.contains[0].value == CardValue::Eight {
                Line::from(format!("Dealer shows an 8")).render(Rect::new(0, 0, area.width, 1), buf);
            } else {
                Line::from(format!("Dealer shows a {}",self.dealer_hand.value)).render(Rect::new(0, 0, area.width, 1), buf);
            }

        } else {
            Line::from(format!("Dealer has {}",self.dealer_hand.value_string())).render(Rect::new(0, 0, area.width, 1), buf);
            pos = 0;
            for card in &self.dealer_hand.contains {
                card.render_card(pos, 1, buf);
                pos+=6
            }
        }
        Line::from(format!("You have {}",active_player_hand.value_string())).render(Rect::new(0, area.height-2, area.width, 1), buf);
        Line::from(format!("Bank:{}",self.players[0].bank)).render(Rect::new(0, area.height-1, area.width, 1), buf);

        Line::from(active_player_hand.outcome.display_string()).render(Rect::new(0, 6, area.width, 1), buf);

    }
}
