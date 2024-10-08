#[cfg(test)]
mod tests {
    use gstd::{prelude::*, ActorId};
    use gtest::{Program, System};
    use session_io::*;

    const WORDLE_ID: u64 = 1;
    const GAME_SESSION_ID: u64 = 2;
    const USER1: u64 = 10;

    fn setup() -> System {
        let sys = System::new();
        sys.init_logger();

        let wordle = Program::from_file(&sys, "../target/wasm32-unknown-unknown/debug/wordle.opt.wasm");
        let game_session = Program::from_file(
            &sys,
            "../target/wasm32-unknown-unknown/debug/game_session.opt.wasm",
        );

        let user_id: ActorId = USER1.into();
        let wordle_id: ActorId = WORDLE_ID.into();

        // Initialize the Wordle program
        assert!(!wordle.send(user_id, wordle_id).main_failed());
        // Initialize the Game Session program
        assert!(!game_session.send(user_id, wordle_id).main_failed());

        sys
    }

    #[test]
    fn test_start_game() {
        let sys = setup();
        let game_session = sys.get_program(GAME_SESSION_ID).unwrap();

        // Simulate user sending StartGame request
        assert!(!game_session.send(USER1, GameSessionAction::StartGame).main_failed());

        // Check if the game session has been created
        let state: GameSessionState = game_session.read_state(()).unwrap();
        assert!(state.game_sessions.iter().any(|(user, _)| *user == USER1.into()));
        println!("State after StartGame: {:?}", state);

        // Check session status
        let session_info = &state
            .game_sessions
            .iter()
            .find(|(user, _)| *user == USER1.into())
            .unwrap()
            .1.clone();
        assert!(matches!(session_info.session_status, SessionStatus::WaitUserInput));
    }

    #[test]
    fn test_win() {
        let sys = setup();
        let game_session = sys.get_program(GAME_SESSION_ID).unwrap();

        // Start the game
        assert!(!game_session.send(USER1, GameSessionAction::StartGame).main_failed());

        // Simulate a correct word check
        assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "hello".to_string() }).main_failed());

        // Validate the session status and tries count
        let state: GameSessionState = game_session.read_state(()).unwrap();
        let session_info = &state
            .game_sessions
            .iter()
            .find(|(user, _)| *user == USER1.into())
            .unwrap()
            .1.clone();
        println!("State after CheckWord: {:?}", state);

        assert_eq!(session_info.tries, 0);  // Adjust based on expected behavior
        assert!(matches!(session_info.session_status, SessionStatus::ReplyReceived(_)));
    }

    #[test]
    fn test_game_over() {
        let sys = setup();
        let game_session = sys.get_program(GAME_SESSION_ID).unwrap();

        // Start the game
        assert!(!game_session.send(USER1, GameSessionAction::StartGame).main_failed());

        // Simulate several incorrect word checks until the game is over
        for _ in 0..6 {
            assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "wrong".to_string() }).main_failed());
        }
      assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "wrong".to_string() }).main_failed());
        assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "wrong".to_string() }).main_failed());
        assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "wrong".to_string() }).main_failed());
        assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "wrong".to_string() }).main_failed());
        assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "wrong".to_string() }).main_failed());
        assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "wrong".to_string() }).main_failed());

        // Check if the session status is GameOver
        let state: GameSessionState = game_session.read_state(()).unwrap();
        let session_info = &state
            .game_sessions
            .iter()
            .find(|(user, _)| *user == USER1.into())
            .unwrap()
            .1;
        assert!(matches!(session_info.session_status, SessionStatus::GameOver(_)));
    }

#[test]
fn test_timeout_with_user_action() {
    let sys = setup();
    let game_session = sys.get_program(GAME_SESSION_ID).unwrap();

    // Start the game
    assert!(!game_session.send(USER1, GameSessionAction::StartGame).main_failed());

    // Simulate a delay but before timeout, user checks the word
    sys.spend_blocks(15);
    assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "house".to_string() }).main_failed());

    // Now simulate a timeout
    sys.spend_blocks(10);
    assert!(!game_session.send(USER1, GameSessionAction::CheckWord { word: "house".to_string() }).main_failed());

    // Validate if the session status is handled correctly after the timeout
    let state: GameSessionState = game_session.read_state(()).unwrap();
    let session_info = &state
        .game_sessions
        .iter()
        .find(|(user, _)| *user == USER1.into())
        .unwrap()
        .1;

    assert!(matches!(session_info.session_status, SessionStatus::GameOver(_)));
}
}
