use crate::snake::*;
use rand::Rng;
use serde::{Serialize, Deserialize};

pub const SPEED: usize = 1000;

const WIDTH: usize = 20;
const HEIGHT: usize = 20;

/// A point
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

/// A Game Event
#[derive(Serialize)]
pub enum GameEvent {
    WaitInLobby,
    Start,
    NewTurn,
}

/// Collision kinds
enum Collision {
    None,
    Food,
    BorderOrSnake,
}

/// Game state
#[derive(Serialize, Clone)]
pub enum GameState {
    Ready,
    Playing,
    Lost,
}

pub struct Game {
    pub snakes: Vec<Snake>,
    pub food: Point,
    pub width: usize,
    pub height: usize,
    pub states: Vec<GameState>,
}
impl Game {
    /// Create new Game
    pub fn new(nb: usize) -> Self {
        let mut snakes: Vec<Snake> = vec![];
        let mut states: Vec<GameState> = vec![];
        for id in 0..nb {
            snakes.push(Snake::init(id, nb, WIDTH, HEIGHT));
            states.push(GameState::Ready);
        }
        let mut game = Game {
            snakes,
            food: Point { x: 0, y: 0 }, // Food is initialized afterwards
            width: WIDTH,
            height: HEIGHT,
            states,
        };
        game.create_food();
        return game;
    }

    /// Check if a point overlaps with snakes
    fn do_overlap(&self, point: Point) -> bool {
        for snake in self.snakes.iter() {
            if snake._do_overlap(point.clone()) {
                return true;
            }
        }
        return false;
    }

    /// Add a food item on the field, don't overlap with snakes
    fn create_food(&mut self) {
        let mut rng = rand::thread_rng();
        let mut point = Point {
            x: rng.gen_range(2..self.width-1) as u16,
            y: rng.gen_range(2..self.height-1) as u16
        };
        while self.do_overlap(point.clone()) {
            point = Point {
                x: rng.gen_range(2..self.width-1) as u16,
                y: rng.gen_range(2..self.height-1) as u16,
            };
        }
        self.food = point;
    }

    /// Check collisions between snakes
    fn check_snake_collisions(&self, snake: &Snake) -> bool {
        let last = snake.body.last().unwrap();
        for s in self.snakes.iter() {
            for p in s.body.iter() {
                if (p as *const _) != (last as *const _) && p.x == last.x && p.y == last.y {
                    return true;
                }
            }
        }
        return false;
    }

    /// Check all kinds of collisions
    fn check_collisions(&self, snake: &Snake) -> Collision {
        if snake._check_border_collisions(self.width, self.height) ||
            self.check_snake_collisions(snake) {
                return Collision::BorderOrSnake;
            }
        if snake._check_food_collision(self.food.clone()) {
            return Collision::Food;
        }
        return Collision::None;
    }

    /// Move all snakes
    fn move_snakes(&mut self) {
        for snake in self.snakes.iter_mut() {
            snake._move();
        }
    }

    /// Play one turn
    pub fn play_turn(&mut self) {
        let old_positions = self.snakes_to_vec();

        self.move_snakes();

        for id in 0..self.snakes.len() {
            match self.check_collisions(&self.snakes[id]) {
                Collision::BorderOrSnake => self.states[id] = GameState::Lost,
                Collision::Food => {
                    self.snakes[id].body = old_positions[id].clone();
                    self.snakes[id]._grow(self.food.clone());
                    self.create_food();
                },
                _ => (),
            }
        }
    }

    /// Set all states to state value
    pub fn set_states(&mut self, state: GameState) {
        for i in 0..self.states.len() {
            self.states[i] = state.clone();
        }
    }

    /// Convert snakes to vectors
    pub fn snakes_to_vec(&self) -> Vec<Vec<Point>> {
        let mut snakes: Vec<Vec<Point>> = vec![];
        for snake in self.snakes.iter() {
            snakes.push(snake.body.clone());
        }
        return snakes;
    }
}
