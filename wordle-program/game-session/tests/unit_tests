use game_session::*;
use gstd::prelude::*;
use gtest::{Log, Program, RunResult, System};
use session_io::*;

static USER_0: u64 = 100;
static USER_1: u64 = 101;
static USER_2: u64 = 102;

static mut USERS: Option<Vec<u64>> = None;

const MAX_TRIES: u32 = 6;
const WORD_LENGTH: usize = 5;

#[test]
fn test_init() {
    let sys = System::new();
    let (session, _, _) = init_program(&sys);

    read_state(&session);
}

#[test]
fn test_start_game() {
    let sys = System::new();
    let (session, _wordle, users) = init_program(&sys);

    start_game(&session, users[0]);    
}

#[test]
fn test_guess_word_win() {
    let sys = System::new();
    let (session, _wordle, users) = init_program(&sys);

    start_game(&session, users[1]);
    start_game(&session, users[1]);

    guess_word(&session, users[1], "hello", UserStatus::GameStarted, 1);
    let log = guess_word(&session, users[1], "horse", UserStatus::GameStarted, 2);
    repeat_start_game_in_guesses(&session, users[1], &log);
    guess_word(&session, users[1], "house", UserStatus::GameOver(GameOver::Win), 3);
    guess_word(&session, users[1], "house", UserStatus::GameOver(GameOver::Win), 3);
}

#[test]
fn test_guess_word_lose() {
    let sys = System::new();
    let (session, _wordle, users) = init_program(&sys);

    start_game(&session, users[1]);
    start_game(&session, users[1]);

    guess_word(&session, users[1], "hello", UserStatus::GameStarted, 1);
    let mut log = guess_word(&session, users[1], "horse", UserStatus::GameStarted, 2);
    repeat_start_game_in_guesses(&session, users[1], &log);
    guess_word(&session, users[1], "hawck", UserStatus::GameStarted, 3);
    log = guess_word(&session, users[1], "happy", UserStatus::GameStarted, 4);
    repeat_start_game_in_guesses(&session, users[1], &log);
    guess_word(&session, users[1], "human", UserStatus::GameStarted, 5);
    guess_word(&session, users[1], "human", UserStatus::GameOver(GameOver::Lose), 6);
    guess_word(&session, users[1], "human", UserStatus::GameOver(GameOver::Lose), 6);
}

#[test]
fn test_guess_word_failure() {
    let sys = System::new();
    let (session, _wordle, users) = init_program(&sys);

    guess_word_without_start(&session, users[1], "qwwwww");
    guess_word_without_start(&session, users[1], "eerwwq");
}


fn init_program(sys: &System) -> (Program, Program, Vec<u64>) {
    sys.init_logger();
    let users = unsafe {
        USERS = Some(Vec::new());
        USERS.as_mut().unwrap()
    };

    users.push(USER_2);
    users.push(USER_1);
    users.push(USER_0);

    let wordle: Program = Program::from_file(
        &sys,
        "/home/lionchen/git/wordle-game-gear-academy/target/wasm32-unknown-unknown/debug/wordle.opt.wasm");
    let result = wordle.send_bytes(users[0], b"");
    assert!(!result.main_failed());
    assert_eq!(wordle.id(), 1.into());

    let session: Program = Program::current(&sys);
    let result = session.send(users[0], wordle.id());
    assert!(!result.main_failed());

    (session, wordle, users.to_vec())
}

fn start_game(session: &Program, user: u64) {
    let result: RunResult = session.send(user, UserAction::StartGame);
    let log = Log::builder()
        .source(session.id())
        .dest(user)
        .payload(UserEvent::Result {
            user_status: session_io::UserStatus::GameStarted,
            correct_position: None,
            contained_in_word: None,
            max_tries: MAX_TRIES,
            tries: None,
            time_out: None,
        });
    assert!(result.contains(&log));
}

fn repeat_start_game_in_guesses(session: &Program, user: u64, log: &Log) {
    let result: RunResult = session.send(user, UserAction::StartGame);
    assert!(result.contains(log));
}

fn guess_word(session: &Program, user: u64, word: &str, user_status: UserStatus, tries: u32) -> Log {
    let result: RunResult = session.send(
        user,
        UserAction::GuessWord {
            word: word.to_string(),
        },
    );
    let answer = "house";

    let mut matched_indices = Vec::with_capacity(WORD_LENGTH);
    let mut key_indices = Vec::with_capacity(WORD_LENGTH);

    for (i, (a, b)) in answer.chars().zip(word.chars()).enumerate() {
        if a == b {
            // If the characters match at the same index, store the index in matched_indices
            matched_indices.push(i as u8);
        } else if answer.contains(b) {
            // If characters do not match, but the word contains the user's input character,
            // store the index in key_indices
            key_indices.push(i as u8);
        }
    }

    let correct_poses = vec_u8_to_comma_separated_string(&matched_indices);
    let contained_poses = vec_u8_to_comma_separated_string(&key_indices);

    let log = Log::builder()
        .source(session.id())
        .dest(user)
        .payload(UserEvent::Result {
            user_status: user_status,
            correct_position: Some(correct_poses),
            contained_in_word: Some(contained_poses),
            max_tries: MAX_TRIES,
            tries: Some(tries),
            time_out: Some(false),
        });
    assert!(result.contains(&log));

    log
}

fn guess_word_without_start(session: &Program, user: u64, word: &str) {
    let result: RunResult = session.send(
        user,
        UserAction::GuessWord {
            word: word.to_string(),
        },
    );
    let log = Log::builder()
        .source(session.id())
        .dest(user)
        .payload(UserEvent::Result {
            user_status: UserStatus::GameNotStarted,
            correct_position: None,
            contained_in_word: None,
            max_tries: MAX_TRIES,
            tries: None,
            time_out: None,
        });

    assert!(result.contains(&log));
}

fn read_state(program: &Program) {
    let program_status: ProgramStatus = program.read_state(b"").unwrap();
    assert!(program_status.user_status_list.is_some());
    assert_eq!(program_status.word_length, Some(5));
    assert_eq!(program_status.max_tries, Some(6));
}
