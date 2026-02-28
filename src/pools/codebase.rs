use std::{ffi::OsStr, fs::{self, read_to_string}, path::{Path, PathBuf}, range::Range, str::Chars};

use crate::{
    ast::expr::{Ast, ParseArgs},
    utils::lookahead_iter::Looakhead,
    pools::{exprs::Exprs, messages::Messages, names::Names},
    tokens::{tokenizer::Tokenizer, tokenstream::Tokens}
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ModId(usize);

enum ModuleSrc {
    File(PathBuf, String),
    Dir(PathBuf),
    Memory {
        name: String,
        data: String,
    }
}

pub struct Module {
    id: ModId,
    src: ModuleSrc,
    submodules: Vec<ModId>,
    ast: Option<Ast>,
}

impl Module {
    fn new(id: ModId, submodules: Vec<ModId>, src: ModuleSrc) -> Self {
        Self { id, submodules, src, ast: None }
    }
    pub fn name(&self) -> String {
        match &self.src {
            ModuleSrc::File(p, _) => p.file_stem().unwrap_or(p.as_os_str()).display().to_string(),
            ModuleSrc::Dir(p) => p.file_name().unwrap_or(p.as_os_str()).display().to_string(),
            ModuleSrc::Memory { name, data: _ } => name.clone(),
        }
    }
    pub fn data(&self) -> Option<&str> {
        match &self.src {
            ModuleSrc::File(_, data) => Some(data),
            ModuleSrc::Dir(_) => None,
            ModuleSrc::Memory { name: _, data } => Some(data),
        }
    }
    pub fn path(&self) -> Option<&Path> {
        match &self.src {
            ModuleSrc::File(p, _) => Some(p),
            ModuleSrc::Dir(p) => Some(p),
            ModuleSrc::Memory { name: _, data: _ } => None,
        }
    }
    pub fn ast(&self) -> Option<&Ast> {
        self.ast.as_ref()
    }
    pub fn create_iter(&self) -> Option<SrcIterator<'_>> {
        Some(SrcIterator::new(self.id, self.data()?.chars()))
    }
    pub fn tokenize(&self, names: Names, messages: Messages) -> Option<Tokens> {
        Some(Tokens::new(
            Tokenizer::new(&mut self.create_iter()?, names.clone(), messages.clone()).collect(),
            "eof",
            Span(self.id, (0..1).into()),
            names,
            messages
        ))
    }
    pub fn parse(&mut self, names: Names, messages: Messages, exprs: Exprs, args: ParseArgs) -> Option<&Ast> {
        if self.ast.is_some() {
            return self.ast.as_ref();
        }
        let mut tokens = self.tokenize(names, messages.clone())?;
        self.ast = Some(Ast::parse(&mut tokens, exprs.clone(), args));
        self.ast.as_ref()
    }
}

pub struct SrcIterator<'s> {
    id: ModId,
    // We need three characters of lookahead for distinguishing doc comments 
    // '///' from normal comments '//'
    iter: Looakhead<Chars<'s>, 3>,
    index: usize,
    // This is for better errors
    last_nonspace_index: usize,
}

impl<'s> SrcIterator<'s> {
    fn new(id: ModId, chars: Chars<'s>) -> Self {
        Self {
            id,
            iter: Looakhead::new(chars),
            index: 0,
            last_nonspace_index: 0,
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
        self.iter.lookahead(0).copied()
    }
    pub fn peek_n(&self, n: usize) -> Option<char> {
        self.iter.lookahead(n).copied()
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
        let ret = self.iter.next();
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
    // Note: first module in this vec is *always* considered the root module!
    modules: Vec<Module>,
}

pub enum CodebaseCreateError {
    CantFindRoot,
    UnableToReadDir(PathBuf, String),
    UnableToReadFile(PathBuf, String),
}

impl Codebase {
    pub fn from_file(path: &Path) -> Result<Self, CodebaseCreateError> {
        let mut ret = Self { modules: Default::default() };
        ret.add_file(path)?;
        Ok(ret)
    }
    pub fn from_dir(path: &Path) -> Result<Self, CodebaseCreateError> {
        let mut ret = Self { modules: Default::default() };
        let root = path.join("main.vid");
        if !root.exists() {
            return Err(CodebaseCreateError::CantFindRoot)?;
        }
        ret.add_file(&root)?;
        ret.add_dir(path)?;
        Ok(ret)
    }
    pub fn from_memory(name: &str, data: &str) -> Self {
        let mut ret = Self { modules: Default::default() };
        ret.add_memory(name, data);
        ret
    }

    fn add_file(&mut self, path: &Path) -> Result<ModId, CodebaseCreateError> {
        if let Some((id, _)) = self.modules.iter().enumerate().find(|m| m.1.path() == Some(path)) {
            return Ok(ModId(id));
        }
        let id = ModId(self.modules.len());
        self.modules.push(Module::new(
            id,
            Vec::new(),
            ModuleSrc::File(path.to_path_buf(), match read_to_string(path) {
                Ok(d) => d,
                Err(e) => Err(CodebaseCreateError::UnableToReadFile(path.to_path_buf(), e.to_string()))?
            })
        ));
        Ok(id)
    }
    fn add_memory(&mut self, name: &str, data: &str) -> ModId {
        // In-memory modules are always unique
        let id = ModId(self.modules.len());
        self.modules.push(Module::new(
            id,
            Vec::new(),
            ModuleSrc::Memory { name: name.to_string(), data: data.to_string() }
        ));
        id
    }
    fn add_dir(&mut self, dir: &Path) -> Result<ModId, CodebaseCreateError> {
        if let Some((id, _)) = self.modules.iter().enumerate().find(|m| m.1.path() == Some(dir)) {
            return Ok(ModId(id));
        }
        let mut submodules = Vec::new();
        match fs::read_dir(dir) {
            Ok(files) => for file in files {
                match file {
                    Ok(f) => {
                        if f.file_type().is_ok_and(|t| t.is_dir()) {
                            submodules.push(self.add_dir(&f.path())?);
                        }
                        else if f.path().extension() == Some(OsStr::new("vid")) {
                            submodules.push(self.add_file(&f.path())?);
                        }
                    }
                    Err(e) => Err(CodebaseCreateError::UnableToReadFile(dir.to_path_buf(), e.to_string()))?
                }
            }
            Err(e) => Err(CodebaseCreateError::UnableToReadDir(dir.to_path_buf(), e.to_string()))?
        }
        let dir_id = ModId(self.modules.len());
        self.modules.push(Module::new(dir_id, submodules, ModuleSrc::Dir(dir.to_path_buf())));
        Ok(dir_id)
    }

    pub fn all_ids(&self) -> Vec<ModId> {
        self.modules.iter().enumerate().map(|m| ModId(m.0)).collect()
    }
    pub fn root(&self) -> ModId {
        ModId(0)
    }

    pub fn fetch(&self, id: ModId) -> &Module {
        self.modules.get(id.0).expect("Codebase has apparently handed out an invalid ModId")
    }
    pub fn parse_all(&mut self, names: Names, messages: Messages, exprs: Exprs, args: ParseArgs) {
        for module in &mut self.modules {
            module.parse(names.clone(), messages.clone(), exprs.clone(), args);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Span(ModId, Range<usize>);

impl Span {
    #[cfg(test)]
    pub fn zero(id: ModId) -> Self {
        Self(id, (0..0).into())
    }
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

// For tests, Span must be PartialEq (to check that asts match) but it should 
// just always resolve to true since we don't care
#[cfg(test)]
impl PartialEq for Span {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[test]
fn src_iter() {
    let codebase = Codebase::from_memory("test_src_iter", "abcdefg");
    let mut iter = codebase.fetch(codebase.root()).create_iter().unwrap();
    for ch in 'a'..='g' {
        assert_eq!(iter.peek(), Some(ch));
        assert_eq!(iter.next(), Some(ch));
    }
}
