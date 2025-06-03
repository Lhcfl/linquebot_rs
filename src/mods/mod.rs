use crate::{linquebot::Module, MicroTask};

pub mod answer_book;
pub mod bestapo;
pub mod bot_on_off;
pub mod debuger;
pub mod dice;
#[cfg(feature = "explain")]
pub mod explain;
pub mod greetings;
pub mod help;
pub mod hitokoto;
#[cfg(feature = "jielong")]
pub mod jielong;
pub mod markov;
pub mod rand;
pub mod repeater;
pub mod rong;
pub mod say;
pub mod search;
pub mod set_title;
#[cfg(feature = "tarot")]
pub mod tarot;
#[cfg(feature = "tarot_ai")]
pub mod tarot_ai;
pub mod todo;
pub mod tools;
pub mod waife;

/// Module Handles 的顺序很重要  
/// 请确保这些函数是拓扑排序的
pub static MODULES: &[&Module] = &[
    // --- super commands ---
    &help::MODULE,
    &help::SAY_HI,
    &bot_on_off::BOT_ON_MODULE,
    &bot_on_off::BOT_OFF_MODULE,
    &bot_on_off::STOP_WHEN_BOT_OFF,
    &waife::ADD_USER,
    // --- normal commands ---
    &debuger::DEBUGGER,
    &todo::MODULE,
    &hitokoto::MODULE,
    &answer_book::MODULE,
    &say::MODULE,
    &repeater::TOGGLE,
    &markov::TOGGLE,
    &bestapo::TOGGLE,
    &search::TOGGLE_SEARCH,
    &search::TOGGLE_SEARCH_RECORDING,
    &rand::MODULE,
    &tools::MODULE,
    #[cfg(feature = "tarot")]
    &tarot::MODULE,
    #[cfg(feature = "tarot_ai")]
    &tarot_ai::MODULE,
    &dice::MODULE,
    #[cfg(feature = "explain")]
    &explain::MODULE,
    &set_title::MODULE,
    #[cfg(feature = "jielong")]
    &jielong::COMMAND,
    &waife::GET_WAIFE,
    &waife::SET_WAIFE_LIMIT,
    &waife::WAIFE_GRAPH,
    &greetings::TOGGLE,
    // --- special command: rongslashbot ---
    &rong::MODULE,
    // --- normal message handles ---
    &markov::GEN_CTNT,
    #[cfg(feature = "jielong")]
    &jielong::ON_IDIOM,
    &markov::TRAIN_MOD,
    &greetings::MODULE,
    &repeater::MODULE,
    &bestapo::MESSAGE_HANDLER,
    &search::MESSAGE_HANDLER,
];

pub static MICRO_TASKS: &[&MicroTask] = &[&help::HELP_CALLBACK, &set_title::ADMIN_CALLBACK];
