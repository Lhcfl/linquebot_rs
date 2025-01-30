use crate::{linquebot::Module, MicroTask};

pub mod answer_book;
pub mod bot_on_off;
pub mod dice;
pub mod explain;
pub mod help;
pub mod hitokoto;
pub mod jielong;
pub mod markov;
pub mod rand;
pub mod repeater;
pub mod rong;
pub mod say;
pub mod set_title;
pub mod tarot;
pub mod tarot_ai;
pub mod todo;

/// Module Handles 的顺序很重要
/// 请确保这些函数是拓扑排序的
pub static MODULES: &[&'static Module] = &[
    // --- super commands ---
    &help::MODULE,
    &bot_on_off::BOT_ON_MODULE,
    &bot_on_off::BOT_OFF_MODULE,
    &bot_on_off::STOP_WHEN_BOT_OFF,
    // --- normal commands ---
    &todo::MODULE,
    &hitokoto::MODULE,
    &answer_book::MODULE,
    &say::MODULE,
    &repeater::TOGGLE,
    &rand::MODULE,
    &tarot::MODULE,
    &tarot_ai::MODULE,
    &dice::MODULE,
    &explain::MODULE,
    &set_title::MODULE,
    &jielong::COMMAND,
    // --- special command: rongslashbot ---
    &rong::MODULE,
    // --- normal message handles ---
    &markov::GEN_CTNT,
    &jielong::ON_IDIOM,
    &markov::TRAIN_MOD,
    &repeater::MODULE,
];

pub static MICRO_TASKS: &[&'static MicroTask] = &[&help::HELP_CALLBACK];
