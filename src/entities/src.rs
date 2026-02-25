use std::{ffi::OsStr, fs::{self, read_to_string}, path::{Path, PathBuf}, range::Range, str::Chars};

use crate::{
    entities::{messages::{Message, MessageLevel, Messages}, names::Names}, tokens::{tokenizer::Tokenizer, tokenstream::Tokens}
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ModId(usize);

pub enum Module {
    File(PathBuf, String),
    Dir(PathBuf),
    Memory(String, String),
}

impl Module {
    pub fn name(&self) -> String {
        match self {
            Self::File(p, _) => p.file_stem().unwrap_or(p.as_os_str()).display().to_string(),
            Self::Dir(p) => p.file_name().unwrap_or(p.as_os_str()).display().to_string(),
            Self::Memory(name, _) => name.clone(),
        }
    }
    pub fn data(&self) -> &str {
        match self {
            Self::File(_, d) => d,
            Self::Dir(_) => "",
            Self::Memory(_, d) => d,
        }
    }
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::File(p, _) => Some(p),
            Self::Dir(p) => Some(p),
            Self::Memory(_, _) => None,
        }
    }
}

const SRC_ITERATOR_PEEK_WINDOW: usize = 3;

pub struct SrcIterator<'s> {
    id: ModId,
    inner: Chars<'s>,
    index: usize,
    // We need three characters of lookahead for distinguishing doc comments 
    // '///' from normal comments '//'
    peek: [Option<char>; SRC_ITERATOR_PEEK_WINDOW],
    // This is for better errors
    last_nonspace_index: usize,
}

impl<'s> SrcIterator<'s> {
    fn new(id: ModId, mut chars: Chars<'s>) -> Self {
        Self {
            id,
            peek: std::array::from_fn(|_| chars.next()),
            index: 0,
            last_nonspace_index: 0,
            inner: chars,
        }
    }
    pub fn next_while<F: Fn(char) -> bool>(&mut self, pred: F) -> String {
        let mut result = String::new();
        while self.peek().is_some_and(&pred) {
            result.push(self.next().unwrap());
        }
        result
    }
    pub fn peek(&self) -> Option<char> {
        self.peek[0]
    }
    pub fn peek_n(&self, n: usize) -> Option<char> {
        self.peek[n]
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn head(&self) -> Span {
        Span(self.id, (self.last_nonspace_index..(self.last_nonspace_index + 1)).into())
    }
    pub fn span_from(&self, start: usize) -> Span {
        Span(self.id, (start..self.index).into())
    }
}

impl<'s> Iterator for SrcIterator<'s> {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        self.peek.rotate_left(1);
        let ret = std::mem::replace(&mut self.peek[SRC_ITERATOR_PEEK_WINDOW - 1], self.inner.next());
        if let Some(c) = ret {
            self.index += 1;
            if !c.is_whitespace() {
                self.last_nonspace_index = self.index;
            }
        }
        ret
    }
}

pub struct Codebase {
    modules: Vec<Module>,
}

impl Codebase {
    pub fn new() -> Self {
        Self {
            modules: Default::default(),
        }
    }
    pub fn add_file(&mut self, path: &Path, messages: Messages) -> ModId {
        if let Some((id, _)) = self.modules.iter().enumerate().find(|m| m.1.path() == Some(path)) {
            return ModId(id);
        }
        let id = ModId(self.modules.len());
        self.modules.push(Module::File(path.to_path_buf(), match read_to_string(path) {
            Ok(data) => data,
            Err(e) => {
                messages.add(Message::new_error(
                    format!("unable to read source module {}: {e}", path.display()),
                    Span(id, (0..1).into())
                ));
                String::new()
            }
        }));
        id
    }
    pub fn add_memory(&mut self, name: &str, data: &str) -> ModId {
        // In-memory modules are always unique
        self.modules.push(Module::Memory(name.to_string(), data.to_string()));
        ModId(self.modules.len() - 1)
    }
    pub fn add_dir(&mut self, dir: &Path, messages: Messages) -> ModId {
        self.modules.push(Module::Dir(dir.to_path_buf()));
        let dir_id = ModId(self.modules.len() - 1);
        match fs::read_dir(dir) {
            Ok(files) => for file in files {
                match file {
                    Ok(f) => {
                        if f.file_type().is_ok_and(|t| t.is_dir()) {
                            self.add_dir(&f.path(), messages.clone());
                        }
                        else if f.path().extension() == Some(OsStr::new("vid")) {
                            self.add_file(&f.path(), messages.clone());
                        }
                    }
                    Err(e) => {
                        messages.add(Message::new(
                            MessageLevel::Error,
                            format!("unable to read directory {}: {e}", dir.display()),
                            None,
                        ));
                    }
                }
            }
            Err(e) => {
                messages.add(Message::new(
                    MessageLevel::Error,
                    format!("unable to read directory {}: {e}", dir.display()),
                    None,
                ));
            }
        }
        dir_id
    }

    pub fn all_ids(&self) -> Vec<ModId> {
        self.modules.iter().enumerate().map(|m| ModId(m.0)).collect()
    }

    pub fn fetch(&self, id: ModId) -> &Module {
        self.modules.get(id.0).expect("Codebase has apparently handed out an invalid ModId")
    }
    pub fn iter_mod(&self, id: ModId) -> SrcIterator<'_> {
        SrcIterator::new(id, self.fetch(id).data().chars())
    }
    pub fn tokenize(&self, id: ModId, names: Names, messages: Messages) -> Tokens {
        Tokens::new(
            Tokenizer::new(&mut self.iter_mod(id), names, messages).collect(),
            "eof",
            Span(id, (0..1).into())
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Span(ModId, Range<usize>);

impl Span {
    pub fn next_ch(self) -> Span {
        Span(self.0, (self.1.end..(self.1.end + 1)).into())
    }
    pub fn id(self) -> ModId {
        self.0
    }
    pub fn range(self) -> Range<usize> {
        self.1
    }
    pub fn start(self) -> usize {
        self.1.start
    }
    pub fn end(self) -> usize {
        self.1.end
    }
    pub fn extend_from(self, start: usize) -> Span {
        Span(self.0, (start..self.1.end).into())
    }
}

#[test]
fn test_src_iter() {
    let mut codebase = Codebase::new();
    let id = codebase.add_memory("test_src_iter", "abcdefg");
    let mut iter = codebase.iter_mod(id);
    for ch in 'a'..='g' {
        assert_eq!(iter.peek(), Some(ch));
        assert_eq!(iter.next(), Some(ch));
    }
}
