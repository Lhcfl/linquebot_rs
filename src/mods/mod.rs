use crate::linquebot::Module;

pub mod answer_book;
pub mod bot_on_off;
pub mod help;
pub mod hitokoto;
pub mod rand;
pub mod rong;
pub mod set_title;
pub mod skip_other_bot;
pub mod todo;

pub static MODULES: &[&'static Module] = &[&rong::MODULE, &help::MODULE, &todo::MODULE];
