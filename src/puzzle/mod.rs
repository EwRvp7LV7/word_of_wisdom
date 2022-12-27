use crate::proto::{Puzzle, PuzzleSolution, PUZZLE_SIZE};
use rand::Rng;
use sha2::{Digest, Sha256};

pub const DEFAULT_COMPLEXITY: u8 = 4;

impl Puzzle {
    pub fn new(complexity: u8) -> Self {
        let value = rand::thread_rng().gen::<[u8; PUZZLE_SIZE]>();
        Puzzle { complexity, value }
    }
}

impl Default for Puzzle {
    fn default() -> Self {
        Self::new(DEFAULT_COMPLEXITY)
    }
}

/// A separate solver structure is needed in order not to blow the protocol structures.
pub struct PuzzleSolver<'a> {
    puzzle: &'a Puzzle,
    precomputed_hash: Sha256,
}

pub struct SolvingResult {
    pub solution: PuzzleSolution,
    pub hashes_tried: u128,
}

impl<'a> PuzzleSolver<'a> {
    pub fn new(puzzle: &'a Puzzle) -> Self {
        let mut precomputed_hash = Sha256::new();
        precomputed_hash.update(puzzle.value);
        Self {
            puzzle,
            precomputed_hash,
        }
    }

    pub fn solve(&self) -> SolvingResult {
        let mut rng = rand::thread_rng();
        let mut hashes_tried: u128 = 0;
        loop {
            let solution = rng.gen::<PuzzleSolution>();
            hashes_tried += 1;
            if self.is_valid_solution(&solution) {
                // let s = format!("{:?}, {:?}", &self.puzzle.value, &solution);
                // log::info!("Puzzle received {}", s);
                // let xxx = &solution.to_vec();
                // let s = String::from_utf8_lossy(xxx);
                // println!("solution {}", s);
                //
                // let xxx1 = &self.puzzle.value.to_vec();
                // let s1 = String::from_utf8_lossy(xxx1);
                // println!("puzzle {}", s1);

                return SolvingResult {
                    solution,
                    hashes_tried,
                };
            }
        }
    }

    pub fn is_valid_solution(&self, solution: &PuzzleSolution) -> bool {
        let mut hasher = self.precomputed_hash.clone();
        hasher.update(solution); //

        let hash = hasher.finalize();
        let mut leading_zeros = 0;

        for c in hash.iter().take(self.puzzle.complexity as usize / 2 + 1) {
            if c >> 4 == 0 {
                leading_zeros += 1;
            } else {
                break;
            }
            if c & 0xF == 0 {
                leading_zeros += 1;
            } else {
                break;
            }
        }

        log::debug!("Hash: {:x}", hash);
        leading_zeros >= self.puzzle.complexity
    }
}
