use std::collections::HashMap;

use crate::Environment;
use crate::parse_and_eval;
use rustyline::{
    Cmd, ConditionalEventHandler, Event, EventContext, EventHandler, Movement, RepeatCount,
};

pub struct LumeKeyHandler {
    command: String,
}
impl LumeKeyHandler {
    pub fn new(command: String) -> Self {
        Self { command }
    }
}
impl From<LumeKeyHandler> for EventHandler {
    fn from(c: LumeKeyHandler) -> Self {
        EventHandler::Conditional(Box::new(c))
    }
}
impl ConditionalEventHandler for LumeKeyHandler {
    fn handle(&self, _: &Event, _: RepeatCount, _: bool, ctx: &EventContext) -> Option<Cmd> {
        let mut env = Environment::new();
        if ctx.line().is_empty() {
            None
        } else {
            parse_and_eval(&self.command.replace("$CMD_CURRENT", ctx.line()), &mut env);
            Some(Cmd::AcceptLine)
        }
    }
}

// abbr
pub struct LumeAbbrHandler {
    abbrs: HashMap<String, String>,
}
impl LumeAbbrHandler {
    pub fn new(abbrs: HashMap<String, String>) -> Self {
        Self { abbrs }
    }
}
impl From<LumeAbbrHandler> for EventHandler {
    fn from(c: LumeAbbrHandler) -> Self {
        EventHandler::Conditional(Box::new(c))
    }
}
impl ConditionalEventHandler for LumeAbbrHandler {
    fn handle(&self, _: &Event, _: RepeatCount, _: bool, ctx: &EventContext) -> Option<Cmd> {
        if ctx.line().contains(' ') {
            return None;
        }
        self.abbrs.get(ctx.line()).map(|ab| {
            Cmd::Replace(
                // rustyline::Movement::BackwardWord(1, Word::Big),
                Movement::WholeBuffer,
                Some(ab.to_owned() + " "),
            )
        })
    }
}
// move one world
// pub struct LumeMoveHandler {}
// impl LumeMoveHandler {
//     pub fn new() -> Self {
//         Self {}
//     }
// }
// impl From<LumeMoveHandler> for EventHandler {
//     fn from(c: LumeMoveHandler) -> Self {
//         EventHandler::Conditional(Box::new(c))
//     }
// }
// impl ConditionalEventHandler for LumeMoveHandler {
//     fn handle(
//         &self,
//         evt: &Event,
//         n: RepeatCount,
//         positive: bool,
//         ctx: &EventContext,
//     ) -> Option<Cmd> {
//         if ctx.has_hint() {
//             Some(Cmd::Move(Movement::ForwardWord(
//                 1,
//                 At::AfterEnd,
//                 Word::Emacs,
//             )))
//         } else {
//             return None;
//         }
//     }
// }
