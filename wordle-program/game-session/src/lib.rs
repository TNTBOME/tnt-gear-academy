#![no_std]

use gstd::{collections::HashMap, exec, msg, vec::Vec, ActorId};
use session_io::*;

// Global static variable to hold the Wordle program's ActorId
static mut WORDLE_ID: Option<ActorId> = None;

// Constants
const MAX_TRIES: u32 = 6;
const MAX_BLOCKS: u32 = 200;
const WORD_LENGTH: usize = 5;

// Initialization function to set up the Wordle program's ActorId
#[gstd::async_init]
async fn init() {
    let wordle_id: ActorId = msg::load().expect("Unable to load wordle id");
    debug!("wordle id: {:?}", wordle_id);
    unsafe {
        WORDLE_ID = Some(wordle_id);
    }
}

// Main entry point for handling messages
#[gstd::async_main]
async fn main() {
    let user: ActorId = msg::source();
    let self_id = exec::program_id();
    let mut user_status_mutex: Arc<Mutex<HashMap<ActorId, UserStatus>>> = unsafe {
        USER_STATUS_MAP_MUTEX.as_ref().expect("User status map is not initialized").clone()
    };
    let user_status_map: &mut HashMap<ActorId, UserStatus> = unsafe {
        USER_STATUS_MAP.as_mut().expect("User status map is not initialized")
    };

    // Handling messages based on the source and type of action
    if user != self_id {
        let action: UserAction = msg::load().expect("Unable to decode user action");

        if action == UserAction::StartGame {
            handle_start(user, user_status_map, &mut user_status_mutex).await;
        } else {
            handle_guess(user, &action, user_status_map, &mut user_status_mutex).await;
        }
    } else {
        let action: CheckGameStatus = msg::load().expect("Unable to decode check game status");
        check_game_status(action.user, user_status_map, &mut user_status_mutex).await;
    }
}

// Handle the start of a new game
async fn handle_start(
    user_id: ActorId,
    user_status_map: &mut HashMap<ActorId, UserStatus>,
    user_status_mutex: &mut Arc<Mutex<HashMap<ActorId, UserStatus>>>,
) {
    let wordle_id: &ActorId = unsafe { WORDLE_ID.as_ref().expect("Wordle id is not initialized") };
    let user_tries_map = unsafe {
        USER_TRIES_MAP
            .as_mut()
            .expect("User tries map is not initialized")
    };
    let user_last_event = unsafe {
        USER_LAST_EVENT
            .as_mut()
            .expect("User last event is not initialized")
    };
    let shared_status = Arc::clone(user_status_mutex);

    let flag: bool;

    // Check if the user already has a game started
    let current_event: UserEvent = match user_status_map.get(&user_id) {
        Some(UserStatus::GameStarted) => {
            flag = false;
            user_last_event
                .get(&user_id)
                .expect("User last event not found")
                .clone()
        }
        _ => {
            // Start a new game by sending a message to the Wordle program
            let future: CodecMessageFuture<Event> = msg::send_for_reply_as(
                *wordle_id,
                Action::StartGame {
                    user: user_id.clone(),
                },
                0,
                0,
            )
            .expect("Unable to send message");
            let event: Event = future.await.expect("Unable to get reply from wordle");

            if let Event::GameStarted { user } = event {
                let mut status_mutex = shared_status.lock().await;
                status_mutex.insert(user.clone(), UserStatus::GameStarted);
                user_status_map.insert(user.clone(), UserStatus::GameStarted);

                user_tries_map.insert(user.clone(), 0);
                let user_event = UserEvent::Result {
                    user_status: UserStatus::GameStarted,
                    correct_position: None,
                    contained_in_word: None,
                    max_tries: MAX_TRIES,
                    tries: None,
                    time_out: None,
                };
                flag = true;
                user_event
            } else {
                panic!("Unexpected event: {:?}", event);
            }
        }
    };

    // Reply to the user with the current game state
    msg::reply(current_event.clone(), 0).expect("Unable to reply");

    user_last_event.insert(user_id, current_event.clone());

    // Schedule a delayed message to check the game status later
    if flag {    
        msg::send_delayed(exec::program_id(), CheckGameStatus { user: user_id }, 0, MAX_BLOCKS).expect("Unable to send delayed message");
    }
}

// Handle user's guess during the game
async fn handle_guess(
    user_id: ActorId,
    action: &UserAction,
    user_status_map: &mut HashMap<ActorId, UserStatus>,
    user_status_mutex: &mut Arc<Mutex<HashMap<ActorId, UserStatus>>>,
) {
    let wordle_id: &ActorId = unsafe { WORDLE_ID.as_ref().expect("Wordle id is not initialized") };
   
    let user_status: Option<UserStatus> = match user_status_map.get(&user_id) {
        Some(UserStatus::GameStarted) => {
            let guess_word: String = match action {
                UserAction::GuessWord { word } => word.to_string(),
                _ => panic!("Unexpected action: {:?}", action),
            };
           
            check(&guess_word);
            
            // Send the guess to the Wordle program and get a response
            let future: CodecMessageFuture<Event> = msg::send_for_reply_as(
                *wordle_id,
                Action::CheckWord {
                    user: user_id.clone(),
                    word: guess_word,
                },
                0,
                0,
            )
            .expect("Unable to send message");
            let event: Event = future.await.expect("Unable to get reply from wordle");
        
            let option: Option<UserStatus> = update_position_and_contained_map(&event);
            option
        }
        Some(UserStatus::GameOver(GameOver::Win)) => {
            Some(UserStatus::GameOver(GameOver::Win))
        }
        Some(UserStatus::GameOver(GameOver::Lose)) => {
            Some(UserStatus::GameOver(GameOver::Lose))
        }
        _ => {
            debug!("User {:?} has not started the game", user_id);
            None
        }
    };
    update_user_status_and_reply(&user_id, &user_status, user_status_mutex).await;
}

// Check the game status after a delay and update if necessary
#[allow(unused)]
async fn check_game_status(
    user: ActorId, 
    user_status_map: &mut HashMap<ActorId, UserStatus>,
    user_status_mutex: &mut Arc<Mutex<HashMap<ActorId, UserStatus>>>,
) {
    let share_status = Arc::clone(&user_status_mutex);
    let mut status_mutex = share_status.lock().await;

    let user_last_event = unsafe {
        USER_LAST_EVENT
            .as_mut()
            .expect("User last event is not initialized")
    };

    // If the user hasn't completed the game, mark it as a loss due to timeout
    match status_mutex.get(&user) {
        Some(UserStatus::GameStarted) => {
            status_mutex.insert(user.clone(), UserStatus::GameOver(GameOver::Lose));
            user_status_map.insert(user.clone(), UserStatus::GameOver(GameOver::Lose));
   
            let mut current_event = user_last_event.get(&user).expect("User last event not found").clone();
        
            match &mut current_event {
                UserEvent::Result { user_status, correct_position, contained_in_word, max_tries, tries, time_out } => {
                    *user_status = UserStatus::GameOver(GameOver::Lose);
                    *time_out = Some(true);
                },
                _ => (), 
            };
            user_last_event.insert(user, current_event);

        }
        _ => (),
    }

}

// Check the validity of the guessed word
fn check(guess_word: &str) {
    if guess_word.len() != WORD_LENGTH {
        panic!("The length of the word must be {}", WORD_LENGTH);
    }
    if guess_word.chars().any(|c| !c.is_ascii_lowercase()) {
        panic!("The word must be all lowercase");
    }
}

// Update the user's position and containment map after a guess
fn update_position_and_contained_map(event: &Event) -> Option<UserStatus> {
    if let Event::WordChecked {
        user,
        correct_position,
        contained_in_word,
    } = event
    {
        let user_correct_position_map = unsafe {
            USER_CORRECT_POSITION_MAP
                .as_mut()
                .expect("User correct position map is not initialized")
        };
        let user_contained_in_word_map = unsafe {
            USER_CONTAINED_IN_WORD_MAP
                .as_mut()
                .expect("User contained in word map is not initialized")
        };
        user_correct_position_map.insert(user.clone(), correct_position.clone());
        user_contained_in_word_map.insert(user.clone(), contained_in_word.clone());

        let user_tries_map = unsafe {
            USER_TRIES_MAP
                .as_mut()
                .expect("User tries map is not initialized")
        };
        let tries: &mut u32 = user_tries_map.get_mut(user).expect("User tries not found");
        *tries += 1;

        Some(UserStatus::GameStarted)
    } else {
        panic!("Unexpected event: {:?}", event);
    }
}

// Update the user's status and reply with the game result
async fn update_user_status_and_reply(
    user: &ActorId,
    user_status: &Option<UserStatus>,
    user_status_mutex: &mut Arc<Mutex<HashMap<ActorId, UserStatus>>>,
) {
    let mut flag = false;

    let share_status = Arc::clone(&user_status_mutex);
    let mut status_mutex = share_status.lock().await;

    if let Some(status) = user_status {
        let mut user_event: UserEvent = {
            let user_last_event = unsafe {
                USER_LAST_EVENT
                    .as_mut()
                    .expect("User last event is not initialized")
            };
            user_last_event.get(user).expect("User last event not found").clone()
        };
        if let Some(tries) = {
            let user_tries_map = unsafe {
                USER_TRIES_MAP
                    .as_mut()
                    .expect("User tries map is not initialized")
            };
            user_tries_map.get(user)
        } {
            match user_event {
                UserEvent::Result { ref mut user_status, ref mut correct_position, ref mut contained_in_word, max_tries, ref mut tries, ref mut time_out } => {
                    let mut user_correct_position_map = unsafe {
                        USER_CORRECT_POSITION_MAP
                            .as_mut()
                            .expect("User correct position map is not initialized")
                    };
                    let mut user_contained_in_word_map = unsafe {
                        USER_CONTAINED_IN_WORD_MAP
                            .as_mut()
                            .expect("User contained in word map is not initialized")
                    };
                    *user_status = status.clone();
                    *correct_position = user_correct_position_map.get(user).expect("Correct position not found").clone();
                    *contained_in_word = user_contained_in_word_map.get(user).expect("Contained in word not found").clone();
                    *tries = Some(*tries);

                    if *tries >= max_tries {
                        *user_status = UserStatus::GameOver(GameOver::Lose);
                        flag = true;
                    } else {
                        if correct_position.iter().all(|&x| x) {
                            *user_status = UserStatus::GameOver(GameOver::Win);
                            flag = true;
                        }
                    }
                    let user_last_event = unsafe {
                        USER_LAST_EVENT
                            .as_mut()
                            .expect("User last event is not initialized")
                    };
                    user_last_event.insert(*user, user_event.clone());
                }
            };
        }
        msg::reply(user_event, 0).expect("Unable to reply");
    } else {
        let user_event = UserEvent::Result {
            user_status: UserStatus::GameOver(GameOver::Lose),
            correct_position: None,
            contained_in_word: None,
            max_tries: MAX_TRIES,
            tries: None,
            time_out: None,
        };
        msg::reply(user_event, 0).expect("Unable to reply");
    }

    if flag {
        let mut status_mutex = share_status.lock().await;
        status_mutex.insert(*user, UserStatus::GameOver(GameOver::Lose));
        let user_status_map = unsafe {
            USER_STATUS_MAP
                .as_mut()
                .expect("User status map is not initialized")
        };
        user_status_map.insert(*user, UserStatus::GameOver(GameOver::Lose));
    }
}

static mut USER_TRIES_MAP: Option<HashMap<ActorId, u32>> = None;
static mut USER_CORRECT_POSITION_MAP: Option<HashMap<ActorId, Vec<bool>>> = None;
static mut USER_CONTAINED_IN_WORD_MAP: Option<HashMap<ActorId, Vec<bool>>> = None;
static mut USER_LAST_EVENT: Option<HashMap<ActorId, UserEvent>> = None;
static mut USER_STATUS_MAP_MUTEX: Option<Arc<Mutex<HashMap<ActorId, UserStatus>>>> = None;
static mut USER_STATUS_MAP: Option<HashMap<ActorId, UserStatus>> = None;

#[gstd::async_main]
async fn init() {
    unsafe {
        USER_TRIES_MAP = Some(HashMap::new());
        USER_CORRECT_POSITION_MAP = Some(HashMap::new());
        USER_CONTAINED_IN_WORD_MAP = Some(HashMap::new());
        USER_LAST_EVENT = Some(HashMap::new());
        USER_STATUS_MAP_MUTEX = Some(Arc::new(Mutex::new(HashMap::new())));
        USER_STATUS_MAP = Some(HashMap::new());
    }
}
