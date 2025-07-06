use std::collections::HashMap;

use crate::Environment;
use crate::parse_and_eval;
use rustyline::{
    Cmd, ConditionalEventHandler, Event, EventContext, EventHandler, Movement, RepeatCount,
};

// ---- LumeKeyHandler
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
        // if ctx.line().is_empty() {
        //     None
        // } else {
        parse_and_eval(&self.command.replace("$CMD_CURRENT", ctx.line()), &mut env);
        Some(Cmd::AcceptLine)
        // }
    }
}

// ---- LumeAbbrHandler
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
pub struct LumeMoveHandler {
    mode: u8,
}
impl LumeMoveHandler {
    pub fn new(mode: u8) -> Self {
        Self { mode }
    }
}
impl From<LumeMoveHandler> for EventHandler {
    fn from(c: LumeMoveHandler) -> Self {
        EventHandler::Conditional(Box::new(c))
    }
}
impl ConditionalEventHandler for LumeMoveHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: RepeatCount,
        _positive: bool,
        ctx: &EventContext,
    ) -> Option<Cmd> {
        if ctx.has_hint() {
            let hint = ctx.hint_text().unwrap();
            let pos = match self.mode {
                1 => {
                    let pos = hint.find(&['<', '[']);
                    pos.unwrap_or(hint.len())
                }
                _ => match hint.find('/') {
                    None => match hint.starts_with(" ") {
                        false => hint.find(" ").unwrap_or(hint.len()),
                        true => match hint.trim_start().find(" ") {
                            Some(x) => x + 1,
                            _ => hint.len(),
                        },
                    },
                    Some(x) => (x + 1).max(hint.len()),
                },
            };

            let hintword = hint[..pos].to_string();
            // dbg!(&hint, &pos, &hintword);
            Some(Cmd::Insert(1, hintword))
        } else {
            return None;
        }
    }
}
