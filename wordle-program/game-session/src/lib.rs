#![no_std]
use session_io::*;
use gstd::{exec, msg, vec, MessageId, debug};

static mut GAME_SESSION_STATE: Option<GameSession> = None;

fn get_game_session_mut() -> &'static mut GameSession {
    unsafe { GAME_SESSION_STATE.as_mut().expect("Uninitialized state") }
}

fn get_game_session() -> &'static GameSession {
    unsafe { GAME_SESSION_STATE.as_ref().expect("Uninitialized state") }
}

#[no_mangle]
extern "C" fn init() {
    let config: GameSessionInit = msg::load().expect("Unable to decode GameSessionInit");
    config.assert_valid();
    let game_session = GameSession::from(config);
    unsafe {
        GAME_SESSION_STATE = Some(game_session);
        debug!("Initialized game session: {:?}", GAME_SESSION_STATE);
    }
}

#[no_mangle]
extern "C" fn handle() {
    let action: GameSessionAction = msg::load().expect("Unable to decode GameSessionAction");
    let game_session = get_game_session_mut();
    let user = msg::source();
    debug!("Handling action: {:?}", action);
    debug!("Current game session state: {:?}", game_session);

    match action {
        GameSessionAction::StartGame => {
            let session = game_session.sessions.entry(user).or_insert_with(|| {
                SessionInfo {
                    session_id: msg::id(),
                    original_msg_id: msg::id(),
                    send_to_wordle_msg_id: MessageId::default(),
                    tries: 0,
                    session_status: SessionStatus::Init,
                }
            });

            match &session.session_status {
                SessionStatus::Init => {
                    debug!("{:?}", user);
                    msg::send(game_session.wordle_program_id, WordleAction::StartGame { user }, 0)
                        .expect("Unable to send StartGame action to Wordle program");
                    session.session_status = SessionStatus::WaitWordleStartReply;
                    exec::wait();
                }
                SessionStatus::ReplyReceived(_) => {
                    session.session_status = SessionStatus::WaitUserInput;
                    msg::reply(GameSessionEvent::StartSuccess, 0)
                        .expect("Failed to send StartSuccess event");

                    msg::send_delayed(
                        exec::program_id(),
                        GameSessionAction::CheckGameStatus { user, session_id: msg::id() },
                        0,
                        200,
                    )
                        .expect("Unable to send CheckGameStatus action");
                }
                _ => {
                    debug!("Unexpected session status for StartGame: {:?}", session.session_status);
                }
            }
        }

        GameSessionAction::CheckWord { word } => {
            let session = game_session.sessions.get_mut(&user)
                .expect("Session not found");

            match &session.session_status {
                SessionStatus::WaitUserInput => {
                    let wordle_action = WordleAction::CheckWord { user, word };
                    session.session_status = SessionStatus::WaitWordleCheckWordReply;
                    msg::send(game_session.wordle_program_id, wordle_action, 0)
                        .expect("Unable to send CheckWord action to Wordle program");
                    exec::wait();
                }
                SessionStatus::ReplyReceived(WordleEvent::WordChecked { correct_positions, contained_in_word ,user: _}) => {
                    let correct_positions=correct_positions.clone();
                    let contained_in_word=contained_in_word.clone();
                    session.tries += 1;
                    if session.tries >= 6 {
                        session.session_status = SessionStatus::GameOver(GameStatus::Lose);
                        msg::reply(GameSessionEvent::GameOver(GameStatus::Lose), 0)
                            .expect("Failed to send GameOver event");
                    } else if correct_positions.len() == 5 {
                        session.session_status = SessionStatus::GameOver(GameStatus::Win);
                        msg::reply(GameSessionEvent::GameOver(GameStatus::Win), 0)
                            .expect("Failed to send GameOver event");
                    } else {
                        session.session_status = SessionStatus::WaitUserInput;
                        msg::reply(GameSessionEvent::CheckWordResult {
                            correct_positions,
                            contained_in_word,
                        }, 0).expect("Failed to send CheckWordResult event");
                    }
                }
                _ => {
                    debug!("Unexpected session status for CheckWord: {:?}", session.session_status);
                }
            }
        }

        GameSessionAction::CheckGameStatus { user, session_id } => {
            let session = game_session.sessions.get(&user)
                .expect("Session not found");

            if session.session_id == session_id {
                let event = match &session.session_status {
                    SessionStatus::GameOver(status) => GameSessionEvent::GameOver(status.clone()),
                    _ => GameSessionEvent::CheckWordResult {
                        correct_positions: vec![],
                        contained_in_word: vec![],
                    },
                };
                msg::reply(event, 0).expect("Failed to send reply");
            } else {
                debug!("Session ID mismatch in CheckGameStatus");
            }
        }
    }
}

#[no_mangle]
extern "C" fn handle_reply() {
    let wordle_event: WordleEvent = msg::load().expect("Unable to decode WordleEvent");
    let game_session = get_game_session_mut();
    let user = wordle_event.get_user();
    debug!("Handling WordleEvent: {:?}", wordle_event);

    if let Some(session) = game_session.sessions.get_mut(user) {
        session.session_status = SessionStatus::ReplyReceived(wordle_event);
        debug!("Updated session status: {:?}", session);
        exec::wake(session.original_msg_id).expect("Failed to wake");
    } else {
        debug!("Session not found for user: {:?}", user);
    }
}

#[no_mangle]
extern "C" fn state() {
    let game_session = get_game_session();
    let state: GameSessionState = game_session.into();
    msg::reply(state, 0).expect("Failed to reply with state");
}
