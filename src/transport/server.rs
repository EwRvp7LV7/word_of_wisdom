use std::error::Error;
use std::fs;
use std::net::{Shutdown, TcpListener};
use std::sync::Arc;
use std::thread;
use anyhow::Result;
use rand::seq::SliceRandom;
use crate::proto::{Puzzle, PuzzleSolution, SOLUTION_SIZE, SolutionState};
use crate::puzzle::{DEFAULT_COMPLEXITY, PuzzleSolver};
use crate::transport::{ClientState, Connection, Transport};

pub struct Server {
    pub(crate) responses: Vec<String>,
    puzzle_complexity: u8,
}

impl<'a> Server {
    pub fn new(responses: Vec<String>) -> Result<Self, Box<dyn Error>> {
        if responses.is_empty() {
            return Err("responses must not be empty".into());
        }
        Ok(Server {
            responses,
            puzzle_complexity: DEFAULT_COMPLEXITY,
        })
    }

    pub fn new_from_file(filename: &str) -> Result<Self, Box<dyn Error>> {
        let mut responses = Vec::<String>::new();
        log::info!("Loading response phrases from {}", filename);
        for val in fs::read_to_string(filename)?.split("\n\n") {
            responses.push(val.trim_matches(&['\r', '\n', ' '][..]).into());
        }
        log::info!("{} phrases loaded", responses.len());
        Self::new(responses)
    }

    pub fn set_puzzle_complexity(&mut self, complexity: u8) {
        self.puzzle_complexity = complexity;
    }

    fn random_response(&self) -> &String {
        self.responses.choose(&mut rand::thread_rng()).unwrap()
    }

    fn handle_connection(&self, conn: &mut Connection) -> Result<()> {
        let mut client = Transport::new(conn.stream.try_clone()?);
        let puzzle = Puzzle::new(self.puzzle_complexity);
        let solver = PuzzleSolver::new(&puzzle);

        loop {
            match conn.state {
                ClientState::Initial => {
                    client.send(&puzzle)?;
                    log::info!("Puzzle sent");
                    conn.state = ClientState::PuzzleSent;
                }
                ClientState::PuzzleSent => {
                    log::info!("Waiting for solution");
                    let solution: PuzzleSolution = client.receive(SOLUTION_SIZE)?;
                    log::info!("Solution received");

                    if solver.is_valid_solution(&solution) {
                        log::info!("Solution accepted");
                        client.send(&SolutionState::Accepted)?;
                        client.send_with_varsize(self.random_response())?;
                    } else {
                        client.send(&SolutionState::Rejected)?;
                        log::error!("Solution rejected");
                    }

                    conn.stream.shutdown(Shutdown::Both)?;
                    log::info!("Connection closed");
                    break;
                }
            }
        }

        Ok(())
    }

    fn run_listener(self: Arc<Self>, addr: &'a str) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(addr)?;
        log::info!("Listening on {}", addr);

        for stream in listener.incoming() {
            let server_clone = self.clone();
            match stream {
                Ok(stream) => {
                    log::info!("New TCP connection: {}", stream.peer_addr()?);
                    thread::spawn(move || {
                        let mut conn = Connection::new(stream);
                        if let Err(e) = server_clone.handle_connection(&mut conn) {
                            eprintln!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Error establishing TCP connection: {}", e);
                }
            }
        }

        Ok(())
    }

    pub fn run(self, addr: &'a str) -> Result<(), Box<dyn Error>> {
        Arc::new(self).run_listener(addr)?;
        Ok(())
    }
}

