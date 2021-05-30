use crate::snake::*;
use crate::game::*;

use serde::{Serialize, Deserialize};
use std::net::{TcpStream};
use std::io::{Write, BufRead, BufReader, BufWriter};


// Information necessary for server-client connection
pub const SERVER_ADDR: &'static str = "127.0.0.1";
pub const SERVER_PORT: usize = 8080;


/*----------------------------------------------------------------------------------*/
/*  Definition of stream structure and function used for server-client connection   */
/*----------------------------------------------------------------------------------*/                                                                                  

/// Stream object to store our reader and writer object
pub struct Stream<'a> {
    pub reader: BufReader<&'a TcpStream>,
    pub writer: BufWriter<&'a TcpStream>,
}

/// Serialize object and send it as a json to the server
pub fn send<T>(stream: &mut Stream, object: T) where T: Serialize {
    let payload = format!("{}\n", serde_json::to_string(&object).unwrap());
    stream.writer.write(payload.as_bytes()).unwrap();
    stream.writer.flush().unwrap();
}

/// Wait for client message, read it and deserialize it depeding on T
pub fn receive<'a, T>(stream: &mut Stream, response: &'a mut String) -> Result<T, ()> where T: Deserialize<'a> {
    let message = stream.reader.read_line(response);
    let read_num;

    // Error handling
    match message {
        Ok(num) => read_num = num,
        Err(_) => return Err(()),
    }

    // If nothing coundn't be read, it means connection has ended
    if read_num == 0 {
        return Err(());
    }
    
    Ok(serde_json::from_str::<'a, T>(&response[..]).unwrap())
}


/*----------------------------------------------------------------------*/
/*  Definitions of message structures used for server-client connection */
/*----------------------------------------------------------------------*/

/// Direction message
/// Direction: Up, Down, Left, Right
#[derive(Deserialize)]
pub struct DirectionMessage {
    pub direction: Direction,
}

/// Force start message
#[derive(Deserialize)]
pub struct ForceStartMessage {
    pub force_start: bool,
}

/// Turn data
#[derive(Serialize, Clone)]
pub struct TurnData {
    pub snakes: Vec<Vec<Point>>,
    pub food: Point,
}

/// Turn data
#[derive(Serialize, Clone)]
pub struct StateData {
    pub states: Vec<GameState>,
}

/// Game events
/// GameEvent: Start, NewTurn
#[derive(Serialize)]
pub struct EventMessage {
    pub event: GameEvent,
}

/// Game state message
/// GameState: Ready, Playing, Lost
#[derive(Serialize, Clone)]
pub struct StateMessage {
    pub state: GameState,
}

/// Game config message
#[derive(Serialize)]
pub struct GameConfigMessage {
    pub id: usize,
    pub width: usize,
    pub height: usize,
    pub snakes: Vec<Vec<Point>>,
    pub food: Point,
}

/// Turn message
#[derive(Serialize, Clone)]
pub struct TurnMessage {
    pub id: usize,
    pub snakes: Vec<Vec<Point>>,
    pub food: Point,
}