#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use std::net::{Shutdown, TcpStream};
    use std::thread;
    use std::time::Duration;
    use thread::sleep;
    use bincode::{deserialize, serialize};
    use mockstream::SharedMockStream;
    use sha2::{Digest, Sha256};
    use tempfile::NamedTempFile;
    use std::io::Write;

    use crate::proto::{Puzzle, PUZZLE_SIZE, SOLUTION_SIZE, SOLUTION_STATE_SIZE, SolutionState};
    use crate::puzzle::{DEFAULT_COMPLEXITY, PuzzleSolver};
    use crate::transport::client::Client;
    use crate::transport::server::Server;
    use crate::transport::Transport;

    #[test]
    fn test_puzzle_new() {
        let p = Puzzle::new(5);
        assert_eq!(p.complexity, 5);
        assert_ne!(p.value, [0u8; PUZZLE_SIZE]);
    }

    #[test]
    fn test_puzzle_default() {
        let p = Puzzle::default();
        assert_eq!(p.complexity, DEFAULT_COMPLEXITY);
        assert_ne!(p.value, [0u8; PUZZLE_SIZE]);
    }

    #[test]
    fn test_is_not_valid_solution() {
        let puzzle = Puzzle::new(30);
        let solver = PuzzleSolver::new(&puzzle);
        assert!(!solver.is_valid_solution(&[0u8; SOLUTION_SIZE]));
    }

    #[test]
    fn test_puzzle_solve() {
        //calculates result
        let puzzle = Puzzle::new(3);
        let solver = PuzzleSolver::new(&puzzle);
        let result = solver.solve();
        assert!(result.hashes_tried > 0);
        assert!(solver.is_valid_solution(&result.solution));
        //checks result complexity
        let mut hasher = Sha256::default();
        hasher.update(puzzle.value);
        hasher.update(result.solution);
        let hash_hex = format!("{:x}", hasher.finalize());
        assert!(hash_hex.starts_with("000"));
    }

    #[test]
    fn test_transport_send() {
        let mut mock_stream = SharedMockStream::new();
        let mut transport = Transport::<SharedMockStream>::new(mock_stream.clone());

        transport.send(&SolutionState::Accepted).unwrap();
        let received = mock_stream.pop_bytes_written();
        assert_eq!(received.len(), SOLUTION_STATE_SIZE);
        assert_eq!(received, serialize(&SolutionState::Accepted).unwrap());

        let puzzle = Puzzle::default();
        transport.send(&puzzle).unwrap();
        let received = mock_stream.pop_bytes_written();
        assert_eq!(received.len(), size_of::<Puzzle>());
        assert_eq!(received, serialize(&puzzle).unwrap());
    }

    #[test]
    fn test_transport_send_with_varsize() {
        let mut mock_stream = SharedMockStream::new();
        let mut transport = Transport::<SharedMockStream>::new(mock_stream.clone());
        let sent_message = String::from("hello, world");

        transport.send_with_varsize(&sent_message).unwrap();
        let received_data = mock_stream.pop_bytes_written();
        let size: usize = deserialize(&received_data[..size_of::<usize>()]).unwrap();
        assert_eq!(size, serialize(&sent_message).unwrap().len());

        let received_message: String = deserialize(&received_data[size_of::<usize>()..]).unwrap();
        assert_eq!(sent_message, received_message);
    }

    #[test]
    fn test_transport_receive() {
        let mut mock_stream = SharedMockStream::new();
        let mut transport = Transport::<SharedMockStream>::new(mock_stream.clone());

        let sent_puzzle = Puzzle::default();
        let bin_data = serialize(&sent_puzzle).unwrap();
        mock_stream.push_bytes_to_read(&bin_data);

        let received_puzzle = transport.receive::<Puzzle>(size_of::<Puzzle>()).unwrap();
        assert_eq!(sent_puzzle, received_puzzle);
    }

    #[test]
    fn test_transport_receive_varsize() {
        let mut mock_stream = SharedMockStream::new();
        let mut transport = Transport::<SharedMockStream>::new(mock_stream.clone());
        let sent_msg = String::from("hello, world");
        let bin_data = serialize(&sent_msg).unwrap();
        let msg_size = bin_data.len();
        mock_stream.push_bytes_to_read(&serialize(&msg_size).unwrap());
        mock_stream.push_bytes_to_read(&bin_data);
        let received_msg: String = transport.receive_varsize().unwrap();
        assert_eq!(sent_msg, received_msg);
    }

    #[test]
    fn test_server_new_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "response 1 \n\n").unwrap();
        writeln!(file, " \rresponse 2 \n").unwrap();
        writeln!(file, "\nresponse 3").unwrap();
        file.flush().unwrap();

        let server = Server::new_from_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(
            server.responses,
            vec![
                String::from("response 1"),
                String::from("response 2"),
                String::from("response 3"),
            ]
        );
    }

    fn wait_server_ready(addr: &str) {
        loop {
            if let Ok(c) = TcpStream::connect(addr) {
                let _ = c.shutdown(Shutdown::Both);
                break;
            }
            sleep(Duration::from_millis(100));
        }
    }

    #[test]
    fn test_client_and_server() {
        let addr = "127.0.0.1:4000";
        let mut server =
            Server::new(vec![String::from("response 1"), String::from("response 2")]).unwrap();
        server.set_puzzle_complexity(3);
        thread::spawn(move || {
            server.run(addr).unwrap();
        });
        wait_server_ready(addr);
        let client = Client::new(addr);
        let response = client.get_response().unwrap();
        assert!(&response == "response 1" || &response == "response 2")
    }

    #[test]
    fn test_server_invalid_solution() {
        let addr = "127.0.0.1:4001";
        let mut server = Server::new(vec![String::from("response 1")]).unwrap();
        server.set_puzzle_complexity(30);
        thread::spawn(move || {
            server.run(addr).unwrap();
        });
        wait_server_ready(addr);
        let mut transport = Transport::new(TcpStream::connect(addr).unwrap());
        transport.receive::<Puzzle>(size_of::<Puzzle>()).unwrap();
        transport.send(&[0u8; SOLUTION_SIZE]).unwrap();
        let result: SolutionState = transport.receive(SOLUTION_STATE_SIZE).unwrap();
        assert_eq!(result, SolutionState::Rejected);
    }
}
