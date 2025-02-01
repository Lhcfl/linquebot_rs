/// 设置头衔
use log::warn;
use teloxide_core::prelude::*;
use teloxide_core::types::*;
use teloxide_core::ApiError;
use teloxide_core::RequestError;

use crate::linquebot::*;
use crate::utils::telegram::prelude::WarnOnError;
use crate::Consumption;
use msg_context::Context;

async fn reply_info(bot: &Bot, message: Message, info: &str) {
    if let Err(err) = bot
        .send_message(message.chat.id, info)
        .reply_parameters(ReplyParameters::new(message.id))
        .send()
        .await
    {
        warn!("Failed to send reply: {}", err.to_string());
    }
}

async fn handle_err(err: RequestError, bot: &Bot, message: Message) {
    warn!("{err:?}");
    match err {
        RequestError::Api(ApiError::CantDemoteChatCreator) => {
            reply_info(bot, message, "不能给群主设置头衔哦").await
        }
        RequestError::Api(ApiError::Unknown(str)) => {
            if str.contains("can't remove chat owner") {
                reply_info(bot, message, "不能给群主设置头衔哦").await
            } else if str.contains("not enough rights") {
                reply_info(bot, message, "琳酱还没有这个权限哦").await
            } else {
                reply_info(bot, message, "因为未知错误而失败……").await
            }
        }
        _ => reply_info(bot, message, "因为未知错误而失败……").await,
    }
}

async fn clear_title(bot: &Bot, message: Message, user: User) {
    if let Err(err) = bot
        .promote_chat_member(message.chat.id, user.id)
        .send()
        .await
    {
        handle_err(err, bot, message).await;
    } else {
        reply_info(bot, message, "清除头衔成功！").await;
    }
}

async fn set_title(bot: &Bot, message: Message, user: User, title: String) {
    if title.chars().count() >= 16 {
        reply_info(bot, message, "你想要的头衔太长了哦").await;
        return;
    }

    if let Err(err) = bot
        .promote_chat_member(message.chat.id, user.id)
        .can_pin_messages(true)
        .send()
        .await
    {
        handle_err(err, bot, message).await;
        return;
    }

    if let Err(err) = bot
        .set_chat_administrator_custom_title(message.chat.id, user.id, &title)
        .send()
        .await
    {
        handle_err(err, bot, message).await;
        return;
    }

    reply_info(bot, message, &format!("设置成功，现在你是 {title} 了")).await;
}

pub fn on_message(ctx: &mut Context, message: &Message) -> Consumption {
    let title = ctx.cmd?.content;
    let message = message.clone();
    let user = message.from.as_ref()?.clone();

    if !message.chat.is_group() && !message.chat.is_supergroup() {
        Consumption::StopWith(Box::pin(reply_info(
            &ctx.app.bot,
            message,
            "需要在群里才能设置头衔哦",
        )))
    } else if title.is_empty() {
        Consumption::StopWith(Box::pin(clear_title(&ctx.app.bot, message, user)))
    } else {
        Consumption::StopWith(Box::pin(set_title(
            &ctx.app.bot,
            message,
            user,
            title.to_string(),
        )))
    }
}

pub fn on_my_chat_member(app: &'static App, chat_member: &ChatMemberUpdated) -> Consumption {
    if !chat_member.old_chat_member.is_privileged()
        && chat_member.new_chat_member.can_promote_members()
    {
        tokio::spawn(
            app.bot
                .send_message(
                    chat_member.chat.id,
                    format!(
                        "感谢 {}，琳酱现在是管理员了，可以使用 /t 功能 ^_^",
                        chat_member.from.full_name()
                    ),
                )
                .send()
                .warn_on_error("set-title"),
        );
    } else if chat_member.old_chat_member.can_promote_members()
        && !chat_member.new_chat_member.can_promote_members()
    {
        tokio::spawn(
            app.bot
                .send_message(
                    chat_member.chat.id,
                    "琳酱被取消了管理权限，不再能使用 /t 功能 >_<",
                )
                .send()
                .warn_on_error("set-title"),
        );
    }
    Consumption::Next
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "t",
        description: "设置头衔",
        description_detailed: Some(concat!(
            "不加参数的 /t 会清除头衔。\n",
            "加参数的 /t xxx 设置头衔为 xxx。\n",
            "琳酱必须具有设置管理员的权限，琳酱没办法对非琳酱设置的管理员设置头衔"
        )),
    }),
    task: on_message,
};

pub static ADMIN_CALLBACK: MicroTask = MicroTask::OnMyChatMember(on_my_chat_member);
