use crate::game::*;
use serde::{Deserialize};

/// Directions
#[derive(Deserialize, Clone, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Snake's structure
pub struct Snake {
    pub body: Vec<Point>, // Vec of points representing the body of the snake
    pub direction: Direction, // Current direction of our snake
}
impl Snake {
    /// Init the snake at the center of the screen, moving in towards the right
    pub fn init(id: usize, nb: usize, width: usize, height: usize) -> Self {
        let mut body = vec![];
        body.push(Point { x: (width as u16) / 2 - 1, y: (height / (2 * nb) * (id + 1)) as u16 });
        body.push(Point { x: (width as u16) / 2, y: (height / (2 * nb) * (id + 1)) as u16 });
        body.push(Point { x: (width as u16) / 2 + 1, y: (height / (2* nb) * (id +1)) as u16 });
        Snake { body, direction: Direction::Right }
    }

    /// Move the snake for one play
    pub fn _move(&mut self) {
        self.body.remove(0); // Safe remove, our snake is always of sz >= 3
        let p = self.body.last().unwrap();
        let point: Point = match self.direction {
            Direction::Up => Point {x: p.x, y: p.y - 1 },
            Direction::Down => Point { x: p.x, y: p.y + 1 },
            Direction::Right => Point { x: p.x + 1, y: p.y },
            Direction::Left => Point { x: p.x - 1, y: p.y },
        };
        self.body.push(point);
    }

    /// Make the snake grow
    pub fn _grow(&mut self, food: Point) {
        self.body.push(food);
    }

    /// Check if snake body overlaps with point
    pub fn _do_overlap(&self, point: Point) -> bool {
        for p in self.body.iter() {
            if (p.x == point.x) && (p.y == point.y) {
                return true;
            }
        }
        return false;
    }

    /// Check collisions with border
    pub fn _check_border_collisions(&self, width: usize, height: usize) -> bool {
        let p = self.body.last().unwrap();
        if (p.x == 1) || (p.x == (width as u16)) {
            return true;
        }
        if (p.y == 1) || (p.y == (height as u16)) {
            return true;
        }
        return false;
    }

    /// Check self collisions
    pub fn _check_self_collision(&self) -> bool {
        let last = self.body.last().unwrap();
        for p in self.body.iter() {
            if (p as *const _) != (last as *const _) && p.x == last.x && p.y == last.y {
                return true;
            }
        }
        return false;
    }

    /// Check collisions with food elements
    pub fn _check_food_collision(&self, point: Point) -> bool {
        let p = self.body.last().unwrap();
        if (p.x == point.x) && (p.y == point.y) {
            return true;
        }
        return false;
    }
}
