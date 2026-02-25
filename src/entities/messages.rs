use std::{fmt::Display, sync::{Arc, Mutex}};

use crate::entities::src::{Codebase, Span};

#[derive(Debug)]
pub enum NoteLevel {
    Note,
    Hint,
}

#[derive(Debug)]
pub struct Note {
    level: NoteLevel,
    text: String,
    span: Option<Span>,
}

#[derive(Debug)]
pub enum MessageLevel {
    Warning,
    Error,
}

#[derive(Debug)]
pub struct Message {
    level: MessageLevel,
    text: String,
    span: Option<Span>,
    notes: Vec<Note>,
}

impl Message {
    pub fn expected<What: Display, Got: Display>(what: What, got: Got, span: Span) -> Self {
        Self::new_error(format!("expected {what}, got {got}"), span)
    }
    pub fn expected_what<What: Display>(what: What, span: Span) -> Self {
        Self::new_error(format!("expected {what}"), span)
    }

    pub fn new<S: Display>(level: MessageLevel, msg: S, span: Option<Span>) -> Self {
        Self {
            level,
            text: msg.to_string(),
            span,
            notes: Vec::new()
        }
    }
    pub fn new_error<S: Display>(msg: S, span: Span) -> Self {
        Self {
            level: MessageLevel::Error,
            text: msg.to_string(),
            span: span.into(),
            notes: Vec::new()
        }
    }
    pub fn new_warning<S: Display>(msg: S, span: Span) -> Self {
        Self {
            level: MessageLevel::Warning,
            text: msg.to_string(),
            span: span.into(),
            notes: Vec::new()
        }
    }
    pub fn with_note<S: Display>(mut self, msg: S, span: Option<Span>) -> Self {
        self.notes.push(Note { level: NoteLevel::Note, text: msg.to_string(), span });
        self
    }
    pub fn with_hint<S: Display>(mut self, msg: S, span: Option<Span>) -> Self {
        self.notes.push(Note { level: NoteLevel::Hint, text: msg.to_string(), span });
        self
    }
}

#[derive(Debug, Clone)]
pub struct Messages {
    messages: Arc<Mutex<Vec<Message>>>,
}

impl Messages {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new()))
        }
    }
    pub fn add(&self, msg: Message) {
        let mut m = self.messages.lock().unwrap();
        m.push(msg);
    }
    pub fn counts(&self) -> (usize, usize) {
        let m = self.messages.lock().unwrap();
        let mut errors = 0;
        let mut warnings = 0;
        for msg in m.iter() {
            match msg.level {
                MessageLevel::Error => errors += 1,
                MessageLevel::Warning => warnings += 1,
            }
        }
        (errors, warnings)
    }
    pub fn count_total(&self) -> usize {
        self.messages.lock().unwrap().len()
    }
    pub fn release<F: Fn(&str)>(&self, codebase: &Codebase, releaser: F) {
        let m = self.messages.lock().unwrap();
        for msg in m.iter() {
            let mut formatted = String::new();
            if let Some(span) = msg.span {
                formatted.push_str(&format!(
                    "[{}:{}..{}] ",
                    codebase.fetch(span.id()).name(),
                    span.start(), span.end(),
                ));
            }
            formatted.push_str(&msg.text);
            releaser(&formatted);
        }
    }
}
