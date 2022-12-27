use anyhow::Result;
use std::error::Error;
use std::mem::size_of;
use std::net::{Shutdown, TcpStream};
use crate::puzzle::PuzzleSolver;
use crate::transport::Transport;
use crate::proto::{Puzzle, SOLUTION_STATE_SIZE, SolutionState};

pub struct Client<'a> {
    addr: &'a str,
}

impl<'a> Client<'a> {
    pub fn new(addr: &'a str) -> Self {
        Self { addr }
    }

    pub fn get_response(&self) -> Result<String, Box<dyn Error>> {
        let stream = TcpStream::connect(self.addr)?;
        let mut server = Transport::new(stream.try_clone()?);
        let puzzle: Puzzle = server.receive(size_of::<Puzzle>())?;
        log::info!("Puzzle received (complexity: {})", puzzle.complexity);

        log::info!("Solving...");
        let solver = PuzzleSolver::new(&puzzle); // precomputes a hash to increase the performance
        let result = solver.solve();
        log::info!("Puzzle solved with {} attempts", result.hashes_tried);
        server.send(&result.solution)?;

        let result = match server.receive::<SolutionState>(SOLUTION_STATE_SIZE)? {
            SolutionState::Accepted => {
                log::info!("Solution accepted");
                let server_msg: String = server.receive_varsize()?;
                Ok(server_msg)
            }
            SolutionState::Rejected => Err("Solution rejected".into()),
        };
        let _ = stream.shutdown(Shutdown::Both);
        result
    }
}
