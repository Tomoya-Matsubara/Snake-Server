pub mod game;
pub mod snake;
pub mod connection;

use game::*;
use snake::*;
use connection::*;

use std::net::{TcpListener, TcpStream};
use std::io::{Write, BufReader, BufWriter};
use std::thread;
use serde::{Serialize};
use std::sync::mpsc::{Sender, Receiver, channel, TryRecvError};
use std::time::Duration;
use chrono::{Utc, Timelike};
use std::fs::{File, OpenOptions};

// Log file
const LOG_FILE: &'static str = "log";
// Max number of clients in a game
const MAX_CLIENTS: usize = 4;

/// Channels
struct Channels {
    size: usize,
    senders: Vec<Sender<ClientEventMessage>>,
    receivers: Vec<Receiver<ClientMessage>>,
}
/// Game configuration
#[derive(Serialize, Clone)]
pub struct GameConfig {
    width: usize,
    height: usize,
    snakes: Vec<Vec<Point>>,
    food: Point,
}
impl GameConfig {
    pub fn new(game: &Game) -> Self {
        let config = GameConfig {
            width: game.width,
            height: game.height,
            snakes: game.snakes_to_vec(),
            food: game.food.clone(),
        };
        return config;
    }
}

/// Client Events sent from Game thread to client threads
#[derive(Clone)]
enum ClientEvent {
    ExitLobby,
    SendConfig(GameConfig),
    SendNewTurn,
    WaitDirection,
    SendTurnResult(TurnData),
    SendClientGameState(StateData),
}
/// Client events messages sent from Game thread to client threads
struct ClientEventMessage {
    id: usize,
    event: ClientEvent,
}
/// Client messages sent from client threads to Game thread
enum ClientMessage {
    Direction(snake::Direction),
    StartGame,
}

/// Log function
fn log(s: &str) {
    if let Ok(mut file) = OpenOptions::new().append(true).open(LOG_FILE) {
        let now = Utc::now();
        let line = format!("[{}:{}:{}] {}\n", now.hour(), now.minute(), now.second(), s);
        file.write(line.as_bytes()).unwrap();
    }
}

/// Remove players from the game knowing their id
/// Delete their sender, receiver and snake
fn remove_players(mut ids: Vec<usize>, channels: &mut Channels, game: &mut Game) {
    for i in 0..ids.len() {
        let id = ids[i];
        // Remove corresponding channels entries
        channels.senders.remove(id);
        channels.receivers.remove(id);
        channels.size -= 1;
        // Remove states and snakes for this player
        game.states.remove(id);
        game.snakes.remove(id);
        // Update other id
        // (if they are > id, they need -1 since entries have been deleted)
        for j in 0..ids.len() {
            if (i != j) && (ids[j] > ids[i]) {
                ids[j] -= 1;
            }
        }
    }
}

/// Send event to all client threads
fn send_all(event: ClientEvent, channels: &mut Channels, game: &mut Game) {
    let mut ids: Vec<usize> = vec![];
    let mut id = 0;
    for sender in channels.senders.iter() {
        match sender.send(ClientEventMessage { event: event.clone(), id }) {
            Ok(()) => (),
            Err(_) => {
                log(&format!("Client {} closed connection, it will be removed from the pool", id));
                ids.push(id);
            },
        }
        id += 1;
    }
    remove_players(ids, channels, game);
}

/// Receive message from all client threads
fn receive_all(channels: &mut Channels, game: &mut Game) -> Vec<snake::Direction> {
    let mut messages: Vec<Direction> = vec![];
    let mut ids: Vec<usize> = vec![];
    let mut id = 0;
    for receiver in channels.receivers.iter() {
        match receiver.recv() {
            Ok(message) => {
                match message {
                    ClientMessage::Direction(direction) => messages.push(direction),
                    _ => panic!("Wrong ClientMessage type received"),
                }
            },
            Err(_) => {
                log(&format!("Client {} closed connection, it will be removed from the pool", id));
                ids.push(id);
            }
        }
        id += 1;
    }
    remove_players(ids, channels, game);
    return messages;
}

/// Game thread function
fn game_(rx: Receiver<TcpStream>) {
    let _rx = &rx;
    loop {
        let mut channels = Channels { senders: vec![], receivers: vec![], size: 0 };

        loop {
            match _rx.try_recv() {
                Ok(s) => {
                    log(&format!("New client! Connection from: {:?}", s.peer_addr().unwrap()));
                    if channels.size < MAX_CLIENTS {
                        let (tx_c1, rx_c1) = channel();
                        let (tx_c2, rx_c2) = channel();
                        thread::spawn(move || { handle_client(s, tx_c2, rx_c1); });
                        channels.senders.push(tx_c1);
                        channels.receivers.push(rx_c2);
                        channels.size += 1;
                        log(&format!("New client added ! {} clients in the game", channels.size));
                    }
                    // Handle MAX_CLIENTS clients maximum at a time, so other clients will have to wait,
                    // their connection will be terminated
                    if channels.size == MAX_CLIENTS {
                        break
                    }
                },
                Err(e) => match e {
                    TryRecvError::Empty => (), // If empty we do nothing
                    TryRecvError::Disconnected => panic!("Channel Game <-> Main thread disconnected"),
                }
            }

            let mut should_break = false;
            for receiver in channels.receivers.iter() {
                match receiver.try_recv() {
                    Ok(message) => match message {
                        ClientMessage::StartGame => {
                            should_break = true;
                            break;
                        },
                        // If message isn't a Start message, make thread panic
                        _ => panic!("Received wrong event"),
                    }
                    Err(e) => match e {
                        TryRecvError::Empty => (), // If empty we wait
                        TryRecvError::Disconnected => panic!("Channel disconnected"),
                    }
                }
            }
            if should_break { break };

            // Wait a bit, not to make some spam checking
            thread::sleep(Duration::from_millis(500));
        }

        log("Creating game");
        let mut game = Game::new(channels.size);
        
        // Make clients exit lobby
        log("Exiting lobby");
        send_all(ClientEvent::ExitLobby, &mut channels, &mut game);

        let config = GameConfig::new(&game);

        log("Sending client config");
        send_all(ClientEvent::SendConfig(config), &mut channels, &mut game);

        // Now we start playing
        log("Start playing game");
        game.set_states(GameState::Playing);

        loop {
            // If no more snakes are here, exit the loop
            if channels.size == 0 {
                break;
            }

            // Send new turn event to sync with client
            log("Starting new turn");
            send_all(ClientEvent::SendNewTurn, &mut channels, &mut game);

            // Wait client directions
            log("Waiting client directions");
            send_all(ClientEvent::WaitDirection, &mut channels, &mut game);

            // Once it's done receive directions in game thread
            let directions = receive_all(&mut channels, &mut game);
            log(&format!("Directions received: {:?}", directions));
            let mut id = 0;
            for snake in game.snakes.iter_mut() {
                snake.direction = directions[id].clone();
                id += 1;
            }

            // Play turn
            log("Playing turn");
            game.play_turn();

            // Send turn data
            let turn_result = TurnData {
                food: game.food.clone(),
                snakes: game.snakes_to_vec(),
            };
            log("Sending turn results");
            send_all(ClientEvent::SendTurnResult(turn_result), &mut channels, &mut game);
            
            // Send GameState at the end of the turn
            let state = StateData { states: game.states.clone() };
            log("Sending current game state");
            send_all(ClientEvent::SendClientGameState(state), &mut channels, &mut game);

            // Wait a bit, depending on game speed
            thread::sleep(Duration::from_millis(SPEED as u64));
        }

        log("Game is over, starting a new one");
    }
}


/// Client thread function
fn handle_client(
    tcp_stream: TcpStream,
    tx: Sender<ClientMessage>,
    rx: Receiver<ClientEventMessage>
) {
    let mut stream = Stream {
        reader: BufReader::new(&tcp_stream),
        writer: BufWriter::new(&tcp_stream),
    };

    // First client is in Lobby
    // It stays here until a ClientEvent::ExitLobby is sent
    loop {
        match rx.try_recv() {
            Ok(event) => match event.event {
                ClientEvent::ExitLobby => {
                    send(&mut stream, EventMessage { event: game::GameEvent::Start });
                    break;
                },
                // If message isn't a Start message, make thread panic
                _ => panic!("Received wrong event"),
            }
            Err(e) => match e {
                // If empty we stay in lobby
                TryRecvError::Empty => {
                    send(&mut stream, EventMessage { event: game::GameEvent::WaitInLobby });
                }
                TryRecvError::Disconnected => panic!("Channel disconnected"),
            }
        }
        // Check if client don't want to force start the game
        let mut response = String::new();
        match receive::<ForceStartMessage>(&mut stream, &mut response) {
            Err(()) => (), // Handle this case more properly, we skip it for now
            Ok(message) => {
                if message.force_start == true {
                    println!("test");
                    tx.send(ClientMessage::StartGame).unwrap();
                }
            },
        }
        // Make thread sleep a bit
        thread::sleep(Duration::from_millis(1000));
    }

    // Wait SendConfig event
    let ev = rx.recv().unwrap();
    match ev.event {
        ClientEvent::SendConfig(config) => {
            let config_message = GameConfigMessage {
                id: ev.id,
                width: config.width,
                height: config.height,
                snakes: config.snakes,
                food: config.food,
            };
            send(&mut stream, config_message);
        },
        _ => panic!("Received wrong event"),
    }

    for event in rx {
        match event.event {
            ClientEvent::SendNewTurn => {
                send(&mut stream, EventMessage { event: game::GameEvent::NewTurn });
            },
            ClientEvent::WaitDirection => {
                let mut message = String::new();
                match receive::<DirectionMessage>(&mut stream, &mut message) {
                    Ok(dm) => {
                        tx.send(ClientMessage::Direction(dm.direction)).unwrap();
                    },
                    Err(()) => {
                        log(&format!("Client closed connection, closing thread now"));
                        break;
                    },
                }
            },
            ClientEvent::SendTurnResult(turn_data) => {
                let turn_message = TurnMessage {
                    id: event.id,
                    food: turn_data.food,
                    snakes: turn_data.snakes,
                };
                send(&mut stream, turn_message);
            },
            ClientEvent::SendClientGameState(state_data) => {
                send(&mut stream, StateMessage { state: state_data.states[event.id].clone() });
            },
            _ => panic!("Received wrong event"),
        }
    }
}

fn main()
{
    // Reset log file
    File::create(LOG_FILE).unwrap();

    // Create the complete address
    let addrs = format!("{}:{}", connection::SERVER_ADDR, connection::SERVER_PORT);
    println!("Starting server: server address = {}", addrs);
    log(&format!("Server address: {}", addrs));

    // Bind the listener to the socket address
    let listener = TcpListener::bind(addrs).unwrap_or_else(|_| panic!("Could not bind the listener"));

    // Game thread
    let (tx, rx) = channel();
    thread::spawn(move|| { game_(rx) });

    // Deal with incoming client connections
    for tcp_stream in listener.incoming() {
        match tcp_stream {
            Ok(tcp_stream) => {
                tx.send(tcp_stream).unwrap();
            }
            Err(_) => {                 
                eprintln!("Connection failed");
            }
        }
    }
}
 