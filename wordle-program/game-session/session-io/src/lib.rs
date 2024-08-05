#![no_std]

use gmeta::{In, InOut, Metadata, Out};
use gstd::{string::String, vec::Vec, ActorId, Decode, Encode, TypeInfo};

pub struct SessionMetadata;

impl Metadata for SessionMetadata {
    type Init = In<ActorId>;
    type Handle = InOut<UserAction, UserEvent>;
    type Others = ();
    type Reply = ();
    type Signal = ();
    type State = Out<ProgramStatus>;
}

/// 用户发来的 Action 请求
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub enum UserAction {
    StartGame,
    GuessWord { word: String },
}

/// 回复用户的 Event
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum UserEvent {
    Result {
        user_status: UserStatus,
        correct_position: Option<String>,
        contained_in_word: Option<String>,
        max_tries: u32,
        tries: Option<u32>,
        time_out: Option<bool>,
    },
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct CheckGameStatus {
    pub user: ActorId,
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum Action {
    StartGame { user: ActorId },
    CheckWord { user: ActorId, word: String },
}


#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum Event {
    GameStarted {
        user: ActorId,
    },
    WordChecked {
        user: ActorId,
        correct_position: Vec<u8>,
        contained_in_word: Vec<u8>,
    },
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub enum UserStatus {
    GameNotStarted,
    GameStarted,
    GameOver(GameOver),
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub enum GameOver {
    Win,
    Lose,
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct ProgramStatus {
    pub user_status_list: Option<Vec<(ActorId, UserStatus)>>,
    pub word_length: Option<u32>,
    pub max_tries: Option<u32>,
    pub max_blocks: Option<u32>,
}
