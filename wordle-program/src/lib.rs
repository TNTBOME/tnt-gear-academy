#![no_std]

use gmeta::{InOut, Metadata};
use gstd::{string::String, vec::Vec, ActorId, Decode, Encode, TypeInfo};

pub struct WordleMetadata;

impl Metadata for WordleMetadata {
    type Init = ();
    type Handle = InOut<Action, Event>;
    type Others = ();
    type Reply = ();
    type Signal = ();
    type State = ();
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
static mut WORDLE: Option<Wordle> = None;
const BANK_OR_WORDS: [&str; 1] = ["house"];
const WORD_LENGTH: usize = 5;

#[derive(Default)]
struct Wordle {
    games: HashMap<ActorId, String>,
}

#[no_mangle]
extern "C" fn init() {
    debug!("开始初始化 Wordle 合约");
    unsafe {
        WORDLE = Some(Wordle {
            games: HashMap::new(),
        });
    }
    debug!("初始化 Wordle 合约完成");
}

#[no_mangle]
extern "C" fn handle() {
    let action: Action = msg::load().expect("Unable to decode action");
    let wordle: &mut Wordle =
        unsafe { WORDLE.as_mut().expect("Wordle program is not initialized") };

    let reply = match action {
        Action::StartGame { user } => {
            let random_id = get_random_value(BANK_OR_WORDS.len() as u8);
            let word = BANK_OR_WORDS[random_id as usize]; 
            wordle.games.insert(user, word.to_string()); 
            Event::GameStarted { user }
        }
        Action::CheckWord { user, word } => {
            if word.len() != WORD_LENGTH {
                panic!("The length of the word must be {}", WORD_LENGTH);
            }
            let key_word = wordle
                .games
                .get(&user)
                .expect("There is no game with this user");
            let mut matched_indices = Vec::with_capacity(WORD_LENGTH);
            let mut key_indices = Vec::with_capacity(WORD_LENGTH);

            for (i, (a, b)) in key_word.chars().zip(word.chars()).enumerate() {
                if a == b {
                    matched_indices.push(i as u8);
                } else if key_word.contains(b) {
                    key_indices.push(i as u8);
                }
            }
            Event::WordChecked {
                user,
                correct_position: matched_indices,
                contained_in_word: key_indices,
            }
        },
    };
    msg::reply(reply, 0).expect("Error in sending a reply");
}

static mut SEED: u8 = 0;
pub fn get_random_value(range: u8) -> u8 {
    let seed = unsafe { SEED };
    unsafe {
        SEED = SEED.wrapping_add(1);
    }

    let mut random_input: [u8; 32] = exec::program_id().into();
    random_input[0] = random_input[0].wrapping_add(seed);

    let (random, _) = exec::random(random_input).expect("Error in getting random number");
    random[0] % range
}
