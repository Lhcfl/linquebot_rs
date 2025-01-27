use crate::{linquebot::Module, MicroTask};

pub mod answer_book;
pub mod bot_on_off;
pub mod dice;
pub mod explain;
pub mod help;
pub mod hitokoto;
pub mod jielong;
pub mod rand;
pub mod rong;
pub mod set_title;
pub mod todo;

/// Module Handles 的顺序很重要
/// 请确保这些函数是拓扑排序的
pub static MODULES: &[&'static Module] = &[
    &help::MODULE,
    &bot_on_off::BOT_ON_MODULE,
    &bot_on_off::BOT_OFF_MODULE,
    &bot_on_off::STOP_WHEN_BOT_OFF,
    &todo::MODULE,
    &hitokoto::MODULE,
    &answer_book::MODULE,
    &rand::MODULE,
    &dice::MODULE,
    &explain::MODULE,
    &set_title::MODULE,
    &jielong::COMMAND,
    &jielong::ON_IDIOM,
    &rong::MODULE,
];

pub static MICRO_TASKS: &[&'static MicroTask] = &[&help::HELP_CALLBACK];
