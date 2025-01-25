use crate::{linquebot::Module, MicroTask};

pub mod answer_book;
pub mod bot_on_off;
pub mod dice;
pub mod help;
pub mod hitokoto;
pub mod rand;
pub mod rong;
pub mod set_title;
pub mod todo;

/// Module Handles 的顺序很重要
/// 请确保这些函数是拓扑排序的
pub static MODULES: &[&'static Module] = &[
    &help::MODULE,
    &todo::MODULE,
    &hitokoto::MODULE,
    &answer_book::MODULE,
    &rand::MODULE,
    &dice::MODULE,
    &set_title::MODULE,
    &rong::MODULE,
];

pub static MICRO_TASKS: &[&'static MicroTask] = &[&help::HELP_CALLBACK];
