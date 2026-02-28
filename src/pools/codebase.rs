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
    FileOrDir(PathBuf, Option<String>),
    Memory {
        name: String,
        data: String,
    }
}

pub struct Module {
    parent: Option<ModId>,
    id: ModId,
    src: ModuleSrc,
    submodules: Vec<ModId>,
    ast: Option<Ast>,
}

fn module_name(path: &Path) -> String {
    path.file_stem().unwrap_or(path.as_os_str()).display().to_string()
}

impl Module {
    fn new(id: ModId, src: ModuleSrc) -> Self {
        Self { id, parent: None, submodules: Vec::new(), src, ast: None }
    }
    pub fn name(&self) -> String {
        match &self.src {
            ModuleSrc::FileOrDir(p, _) => module_name(p),
            ModuleSrc::Memory { name, data: _ } => name.clone(),
        }
    }
    pub fn data(&self) -> Option<&str> {
        match &self.src {
            ModuleSrc::FileOrDir(_, data) => data.as_deref(),
            ModuleSrc::Memory { name: _, data } => Some(data),
        }
    }
    pub fn path(&self) -> Option<&Path> {
        match &self.src {
            ModuleSrc::FileOrDir(p, _) => Some(p),
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
        if let Some(ref ast) = self.ast {
            return Some(ast);
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

#[derive(Debug)]
pub enum CodebaseCreateError {
    CantFindRoot,
    UnableToReadDir(PathBuf, String),
    UnableToReadFile(PathBuf, String),
    DuplicateNamedModule(String),
}

impl Codebase {
    pub fn from_file(path: &Path) -> Result<Self, CodebaseCreateError> {
        let mut ret = Self { modules: Default::default() };
        ret.add_file(None, path)?;
        Ok(ret)
    }
    pub fn from_dir(path: &Path) -> Result<Self, CodebaseCreateError> {
        let mut ret = Self { modules: Default::default() };

        // Start off by finding and adding the root source file
        let root = path.join("main.vid");
        if !root.exists() {
            return Err(CodebaseCreateError::CantFindRoot)?;
        }
        let root_id = ret.add_file(None, &root)?;

        // Find the rest of the source files
        ret.add_dir(Some(root_id), path)?;
        Ok(ret)
    }
    pub fn from_memory(name: &str, data: &str) -> Result<Self, CodebaseCreateError> {
        let mut ret = Self { modules: Default::default() };
        ret.add_memory(None, name, data)?;
        Ok(ret)
    }
    
    fn get_submodule_by_name(&self, parent: ModId, mod_name: &str) -> Option<ModId> {
        for sub in &self.fetch(parent).submodules {
            if self.fetch(*sub).name() == mod_name {
                return Some(*sub);
            }
        }
        None
    }
    fn add_new_mod(
        &mut self,
        parent: Option<ModId>,
        mod_name: String,
        create_mod: impl FnOnce(ModId) -> Module
    ) -> Result<ModId, CodebaseCreateError> {
        // Check that the name isn't going to conflict with existing paths
        if let Some(pid) = parent && let Some(exist) = self.get_submodule_by_name(pid, &mod_name) {
            Err(CodebaseCreateError::DuplicateNamedModule(self.get_full_mod_name(exist)))?;
        }

        let id = ModId(self.modules.len());
        self.modules.push(create_mod(id));
        
        // Add as submodule if this is one
        if let Some(pid) = parent {
            self.fetch_mut(pid).submodules.push(id);
            self.fetch_mut(id).parent = Some(pid);
        }
        Ok(id)
    }
    fn add_file_or_dir(&mut self, parent: Option<ModId>, path: &Path) -> Result<ModId, CodebaseCreateError> {
        let mod_name = module_name(path);

        // If this file submodule exists already, then keep that, otherwise 
        let exist = parent.and_then(|pid| self.get_submodule_by_name(pid, &mod_name));
        if exist.is_none() {
            self.add_new_mod(
                parent, mod_name,
                |id| Module::new(id, ModuleSrc::FileOrDir(path.to_path_buf(), None))
            );
        }

        Ok()
    }
    fn add_file(&mut self, parent: Option<ModId>, path: &Path) -> Result<ModId, CodebaseCreateError> {
        // Add new file module
        let id = self.add_file_mod(parent, path)?;
        // Update data of new file module
        match &mut self.fetch_mut(id).src {
            ModuleSrc::FileOrDir(_, data) => {
                if data.is_none() {
                    *data = read_file_data()?;
                }
            }
            _ => unreachable!("find_file_mod returned a non-file module")
        }
        let read_file_data = || -> Result<Option<String>, CodebaseCreateError> {
            Ok(match read_to_string(path) {
                Ok(d) => Some(d),
                Err(e) => Err(CodebaseCreateError::UnableToReadFile(
                    path.to_path_buf(),
                    e.to_string()
                ))?
            })
        };
        if let Some(id) = self.find_file_mod(path) {
            return Ok(id);
        }
    }
    fn add_memory(&mut self, parent: Option<ModId>, name: &str, data: &str) -> Result<ModId, CodebaseCreateError> {
        // In-memory modules are always unique, so no need to check if the 
        // memory has already been added
        self.add_mod(
            parent,
            || name.to_string(),
            |id| Ok(Module::new(id, ModuleSrc::Memory { name: name.to_string(), data: data.to_string() }))
        )
    }
    fn add_dir(&mut self, parent: Option<ModId>, dir: &Path) -> Result<ModId, CodebaseCreateError> {
        // If this is an existing file mod, use that, otherwise create a new one
        let dir_id = if let Some(exist) = self.find_file_mod(dir) {
            exist
        }
        else {
            self.add_mod(
                parent,
                || module_name(dir),
                |id| Ok(Module::new(id, ModuleSrc::FileOrDir(dir.to_path_buf(), None)))
            )?
        };

        // Find all source files in this directory
        match fs::read_dir(dir) {
            Ok(files) => for file in files {
                match file {
                    Ok(f) => {
                        // Recursively check all subdirectories aswell
                        if f.file_type().is_ok_and(|t| t.is_dir()) {
                            self.add_dir(Some(dir_id), &f.path())?;
                        }
                        else if f.path().extension() == Some(OsStr::new("vid")) {
                            self.add_file(Some(dir_id), &f.path())?;
                        }
                    }
                    Err(e) => Err(CodebaseCreateError::UnableToReadFile(dir.to_path_buf(), e.to_string()))?
                }
            }
            Err(e) => Err(CodebaseCreateError::UnableToReadDir(dir.to_path_buf(), e.to_string()))?
        }

        Ok(dir_id)
    }

    pub fn root(&self) -> ModId {
        ModId(0)
    }
    pub fn submodules(&self, id: ModId) -> Vec<ModId> {
        self.fetch(id).submodules.clone()
    }

    pub fn get_full_mod_name(&self, id: ModId) -> String {
        let m = self.fetch(id);
        let mut res = m.name();
        if let Some(p) = m.parent {
            res = self.get_full_mod_name(p) + "::" + &res;
        }
        res
    }

    fn fetch_mut(&mut self, id: ModId) -> &mut Module {
        self.modules.get_mut(id.0).expect("Codebase has apparently handed out an invalid ModId")
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

#[test]
fn src_iter() {
    let codebase = Codebase::from_memory("test_src_iter", "abcdefg").unwrap();
    let mut iter = codebase.fetch(codebase.root()).create_iter().unwrap();
    for ch in 'a'..='g' {
        assert_eq!(iter.peek(), Some(ch));
        assert_eq!(iter.next(), Some(ch));
    }
}
