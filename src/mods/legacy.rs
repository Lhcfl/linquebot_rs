use crate::linquebot::*;

pub static MESSAGE_HANDLER: Module = Module {
    kind: ModuleKind::General(Some(ModuleDescription {
        name: "一些 Backport",
        description: "？？？",
        description_detailed: None,
    })),
    task: on_message,
};
