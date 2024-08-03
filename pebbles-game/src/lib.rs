#![no_std]

use gstd::{exec, msg, prelude::*};
use pebbles_game_io::*;

static mut GAME_STATE: Option<GameState> = None;

#[no_mangle]
extern fn init() {
    let init: PebblesInit = msg::load().expect("Unable to load init message");

    // Check for valid input data
    if init.pebbles_count <= 0 || init.max_pebbles_per_turn <= 0 || init.max_pebbles_per_turn > init.pebbles_count {
        panic!("Invalid init parameters");
    }

    // Choose the first player
    let first_player = if get_random_u32() % 2 == 0 {
        Player::User
    } else {
        Player::Program
    };

    // Initialize the game state
    let mut state = GameState {
        pebbles_count: init.pebbles_count,
        max_pebbles_per_turn: init.max_pebbles_per_turn,
        pebbles_remaining: init.pebbles_count,
        difficulty: init.difficulty,
        first_player: first_player.clone(),
        winner: None,
    };

    // Process the first turn if the first player is Program
    if first_player == Player::Program {
        let pebbles_to_remove = match state.difficulty {
            DifficultyLevel::Easy => (get_random_u32() % state.max_pebbles_per_turn) + 1,
            DifficultyLevel::Hard => find_best_move(state.max_pebbles_per_turn, state.pebbles_remaining),
        };
        state.pebbles_remaining -= pebbles_to_remove;
        state.winner = check_winner(&state);
    }

    // Save the state
    unsafe { GAME_STATE = Some(state) };
}

#[no_mangle]
extern fn handle() {
    let action: PebblesAction = msg::load().expect("Unable to load action message");

    unsafe {
        let mut state = GAME_STATE.take().expect("GameState isn't initialized");

        // The game is over, no further actions can be processed
        if state.winner.is_some() {
            GAME_STATE = Some(state);
            return;
        }

        match action {
            PebblesAction::Turn(pebbles) => {
                // Check for valid input data
                if pebbles <= 0 || pebbles > state.max_pebbles_per_turn || pebbles > state.pebbles_remaining {
                    msg::reply(PebblesEvent::InvalidMove, 0).expect("Unable to send message");
                    GAME_STATE = Some(state);
                    return;
                }

                // Process the User's turn
                state.pebbles_remaining -= pebbles;
                state.winner = check_winner(&state);

                // If the game is over, notify the winner
                if let Some(ref winner) = state.winner {
                    msg::reply(PebblesEvent::Won(winner.clone()), 0).expect("Unable to send message");
                    GAME_STATE = Some(state);
                    return;
                }

                // Process the Program's turn
                let pebbles_to_remove = match state.difficulty {
                    DifficultyLevel::Easy => (get_random_u32() % state.max_pebbles_per_turn) + 1,
                    DifficultyLevel::Hard => find_best_move(state.max_pebbles_per_turn, state.pebbles_remaining),
                };
                state.pebbles_remaining -= pebbles_to_remove;
                state.winner = check_winner(&state);

                // Notify the user of the Program's turn
                msg::reply(PebblesEvent::CounterTurn(pebbles_to_remove), 0).expect("Unable to send message");
            },
            PebblesAction::GiveUp => {
                // Handle GiveUp action
                state.winner = Some(Player::Program);
                msg::reply(PebblesEvent::Won(Player::Program), 0).expect("Unable to send message");
            },
            PebblesAction::Restart { difficulty, pebbles_count, max_pebbles_per_turn } => {
                // Handle Restart action
                state = GameState {
                    difficulty,
                    pebbles_count,
                    max_pebbles_per_turn,
                    pebbles_remaining: pebbles_count,
                    first_player: if get_random_u32() % 2 == 0 { Player::User } else { Player::Program },
                    winner: None,
                };
            },
        }

        GAME_STATE = Some(state);
    }
}

/// Returns the GameState structure using the msg::reply function
#[no_mangle]
extern fn state() {
    let game_state = unsafe { GAME_STATE.as_ref().expect("Game state not initialized") };
    msg::reply(game_state, 0).expect("Failed to reply with game state");
}

fn get_random_u32() -> u32 {
    let salt = msg::id();
    let (hash, _num) = exec::random(salt.into()).expect("get_random_u32(): random call failed");
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
}

fn check_winner(state: &GameState) -> Option<Player> {
    if state.pebbles_remaining == 0 {
        Some(match state.first_player {
            Player::User => Player::Program,
            Player::Program => Player::User,
        })
    } else {
        None
    }
}

fn find_best_move(max_pebbles_per_turn: u32, pebbles_remaining: u32) -> u32 {
    if pebbles_remaining % (max_pebbles_per_turn + 1) > 0 {
        pebbles_remaining % (max_pebbles_per_turn + 1)
    } else {
        max_pebbles_per_turn
    }
}
