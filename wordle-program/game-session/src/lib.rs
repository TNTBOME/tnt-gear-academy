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
    // debug!("{:?}",config);
    let game_session = GameSession::from(config);
    unsafe {
        GAME_SESSION_STATE = Some(game_session);
        debug!("Initialized game session{:?},",GAME_SESSION_STATE);
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
            debug!("{:?}",user);
            msg::send(game_session.wordle_program_id, WordleAction::StartGame { user }, 0)
                .expect("Unable to send StartGame action to Wordle program");
            let session_info = SessionInfo {
                session_id: msg::id(),
                original_msg_id: msg::id(),
                send_to_wordle_msg_id: MessageId::default(),
                tries: 0,
                session_status: SessionStatus::WaitUserInput,
            };
            game_session.sessions.insert(user, session_info);

            msg::reply(GameSessionEvent::StartSuccess, 0)
                .expect("Failed to send StartSuccess event");

            msg::send_delayed(
                exec::program_id(),
                GameSessionAction::CheckGameStatus { user, session_id: msg::id() },
                0,
                200,
            )
                .expect("Unable to send CheckGameStatus action");
            exec::wait();
        }

        GameSessionAction::CheckWord { word } => {
            let session = game_session.sessions.get_mut(&user)
                .expect("Session not found");

            if session.tries >= 6 {
                // 处理游戏失败
                session.session_status = SessionStatus::GameOver(GameStatus::Lose);
                msg::reply(GameSessionEvent::GameOver(GameStatus::Lose), 0)
                    .expect("Failed to send GameOver event");
                return;
            }

            let wordle_action = WordleAction::CheckWord { user, word };
            session.session_status = SessionStatus::WaitWordleCheckWordReply;
            session.tries += 1;
            msg::send(game_session.wordle_program_id, wordle_action, 0)
                .expect("Unable to send CheckWord action to Wordle program");
            exec::wait();
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
            }
        }
    }
}

#[no_mangle]
extern "C" fn handle_reply() {
    debug!("asasasasasas");
    let wordle_event: WordleEvent = msg::load().expect("Unable to decode WordleEvent");
    let game_session = get_game_session_mut();
    let user=wordle_event.get_user();
    debug!("ggg{:?}",game_session);
    debug!("g7g{:?}",wordle_event);

    if let Some(session) = game_session.sessions.get_mut(user) {
            session.session_status = SessionStatus::ReplyReceived(wordle_event);
            debug!("ggg{:?}",session);
            exec::wake(session.original_msg_id).expect("Failed to wake");
    }
}

#[no_mangle]
extern "C" fn state() {
    let game_session = get_game_session();
    let state: GameSessionState = game_session.into();
    msg::reply(state, 0).expect("Failed to reply with state");
}
