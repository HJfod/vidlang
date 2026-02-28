use std::{collections::HashMap, ffi::OsStr, fs::read_to_string, io::Error, path::{Path, PathBuf}, range::Range, str::Chars};

use crate::{
    ast::expr::{Ast, ParseArgs, Parser}, pools::{exprs::Exprs, messages::Messages, names::Names}, tokens::{tokenizer::Tokenizer, tokenstream::Tokens}, utils::lookahead_iter::Looakhead
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ModId(usize);

struct SrcModule {
    path: PathBuf,
    name: String,
    data: Option<String>,
    parent: Option<ModId>,
    submodules: HashMap<String, ModId>,
    parsed_ast: Option<Ast>,
}

pub struct Codebase {
    pool: Vec<SrcModule>,
    /// Packages are the root unit of codebases. Each program and library is 
    /// one package with a root module, which may contain any number of submodules
    packages: HashMap<String, ModId>,
}

#[derive(Debug)]
pub enum PackageAddError {
    DuplicateNamedPackage(String),
    UnableToReadDir(PathBuf, String),
    UnableToReadFile(PathBuf, String),
}

impl Codebase {
    pub fn new() -> Self {
        Self { pool: Default::default(), packages: Default::default() }
    }
    #[cfg(test)]
    pub fn new_with_test_package(name: &str, data: &str) -> (Self, ModId) {
        let mut ret = Self::new();
        let id = ret.add_module(SrcModule {
            path: PathBuf::from(name),
            name: name.to_string(),
            data: Some(data.to_string()),
            parent: None,
            submodules: HashMap::new(),
            parsed_ast: None
        });
        ret.packages.insert(name.to_string(), id);
        (ret, id)
    }

    pub fn add_package(&mut self, name: String, root_dir: &Path) -> Result<ModId, PackageAddError> {
        // Check if a package with this name already exists
        if self.packages.contains_key(&name) {
            return Err(PackageAddError::DuplicateNamedPackage(name));
        }

        // Otherwise add the root to the list of modules
        let root_id = self.add_module(SrcModule {
            path: root_dir.to_path_buf(),
            name: name.clone(),
            data: None,
            parent: None,
            submodules: HashMap::new(),
            parsed_ast: None,
        });

        // And then find all submodules
        match self.discover_modules_in_dir(root_id, root_dir) {
            Ok(s) => self.get_mut(root_id).submodules = s,
            Err(e) => {
                // Remove everything new added to the pool if we ran into an 
                // error, so we don't leave stuff in there
                self.pool.truncate(root_id.0);
                Err(e)?
            }
        };

        // And then add the root module itself to the list of packages
        self.packages.insert(name, root_id);
        Ok(root_id)
    }

    fn add_module(&mut self, module: SrcModule) -> ModId {
        let id = ModId(self.pool.len());
        self.pool.push(module);
        id
    }

    fn discover_modules_in_dir(&mut self, parent_id: ModId, dir: &Path) -> Result<HashMap<String, ModId>, PackageAddError> {
        let map_dir_err = |e: Error| PackageAddError::UnableToReadDir(dir.to_path_buf(), e.to_string());

        let mut modules = HashMap::<String, ModId>::new();
        for entry_res in dir.read_dir().map_err(map_dir_err)? {
            let entry = entry_res.map_err(map_dir_err)?;
            let path = entry.path();

            // We only care about directories and source files, skip everything else
            if !path.is_dir() && path.extension() != Some(OsStr::new("vid")) {
                continue;
            }

            // Directories use their full name (like `piece.of.blini`), while 
            // files get the extension clipped off (`main.vid` -> `main`)
            let sub_mod_path_stem = if path.is_dir() { path.file_name() } else { path.file_stem() };
            let sub_mod_name = sub_mod_path_stem.unwrap_or(path.as_os_str()).display().to_string();

            // Check if a module with this name is already added (happens if 
            // you have both a directory named `cats` and a file named `cats.vid`
            // for example)
            let sub_mod_id = match modules.get(&sub_mod_name) {
                Some(existing_id) => {
                    // Prefer file modules as the paths; if the existing one isn't 
                    // a file path, update it
                    let existing = self.get_mut(*existing_id);
                    if existing.path.extension() != Some(OsStr::new("vid")) {
                        existing.path = path.clone();
                    }
                    *existing_id
                }
                // Otherwise add this to the map
                None => {
                    let new_id = self.add_module(SrcModule {
                        path: path.clone(),
                        name: sub_mod_name.clone(),
                        data: None,
                        parent: Some(parent_id),
                        submodules: HashMap::new(),
                        parsed_ast: None
                    });
                    modules.insert(sub_mod_name.clone(), new_id);
                    new_id
                }
            };

            // At this point we have made sure the module is in the pool (and 
            // in the map of submodules); now we can actually read the file or 
            // directory and get the data out of it
            if path.is_dir() {
                let submodules = self.discover_modules_in_dir(sub_mod_id, &path)?;
                if !self.get(sub_mod_id).submodules.is_empty() {
                    panic!(
                        "Something is definitely wrong - module {} already had {} submodules",
                        sub_mod_name,
                        self.get(sub_mod_id).submodules.len()
                    );
                }
                self.get_mut(sub_mod_id).submodules = submodules;
            }
            else {
                let data = read_to_string(&path).map_err(
                    |e| PackageAddError::UnableToReadFile(path, e.to_string())
                )?;
                if self.get(sub_mod_id).data.is_some() {
                    panic!(
                        "Something is definitely wrong - module {} already had data",
                        sub_mod_name
                    );
                }
                self.get_mut(sub_mod_id).data = Some(data);
            }
        }
        Ok(modules)
    }

    fn get_mut(&mut self, id: ModId) -> &mut SrcModule {
        self.pool.get_mut(id.0).expect("Codebase has handed out an invalid ModId")
    }
    fn get(&self, id: ModId) -> &SrcModule {
        self.pool.get(id.0).expect("Codebase has handed out an invalid ModId")
    }

    pub fn packages(&self) -> impl Iterator<Item = (&str, ModId)> {
        self.packages.iter().map(|p| (p.0.as_str(), *p.1))
    }

    pub fn get_package_root(&self, name: &str) -> Option<ModId> {
        self.packages.get(name).copied()
    }
    pub fn get_submodules_for(&self, id: ModId) -> impl Iterator<Item = (&str, ModId)> {
        self.get(id).submodules.iter().map(|p| (p.0.as_str(), *p.1))
    }
    pub fn get_full_mod_name(&self, id: ModId) -> String {
        let m = self.get(id);
        let mut res = m.name.clone();
        if let Some(p) = m.parent {
            res = self.get_full_mod_name(p) + "::" + &res;
        }
        res
    }
    pub fn get_ast_for(&self, id: ModId) -> Option<&Ast> {
        self.get(id).parsed_ast.as_ref()
    }

    pub fn create_src_iter(&self, id: ModId) -> Option<SrcIterator<'_>> {
        Some(SrcIterator::new(id, self.get(id).data.as_ref()?.chars()))
    }
    pub fn tokenize(&self, id: ModId, names: Names, messages: Messages) -> Option<Tokens> {
        Some(Tokens::new(
            Tokenizer::new(&mut self.create_src_iter(id)?, names.clone(), messages.clone()).collect(),
            "eof",
            Span(id, (0..0).into()),
            names,
            messages
        ))
    }
    pub fn parse_one(&mut self, id: ModId, names: Names, messages: Messages, exprs: Exprs, args: ParseArgs) -> Option<&Ast> {
        // Having to re-get this is silly but I ran into borrow checker issues
        if self.get(id).parsed_ast.is_some() {
            return self.get(id).parsed_ast.as_ref();
        }
        let mut tokens = self.tokenize(id, names, messages.clone())?;
        self.get_mut(id).parsed_ast = Some(Ast::parse(&mut Parser::new(&mut tokens, exprs, args)));
        self.get(id).parsed_ast.as_ref()
    }
    pub fn parse_all(&mut self, names: Names, messages: Messages, exprs: Exprs, args: ParseArgs) {
        for id in self.pool.iter().enumerate().map(|m| ModId(m.0)).collect::<Vec<_>>() {
            self.parse_one(id, names.clone(), messages.clone(), exprs.clone(), args);
        }
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
    let (codebase, id) = Codebase::new_with_test_package("test_src_iter", "abcdefg");
    let mut iter = codebase.create_src_iter(id).unwrap();
    for ch in 'a'..='g' {
        assert_eq!(iter.peek(), Some(ch));
        assert_eq!(iter.next(), Some(ch));
    }
}

#[test]
fn create_codebase() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.vid"), "").unwrap();

    // This one should be ignored
    std::fs::write(dir.path().join("not-a-vid-file.txt"), "").unwrap();

    // Empty directories should still be included
    std::fs::create_dir(dir.path().join("empty")).unwrap();

    // Create both a shadow.vid file and a shadow directory
    std::fs::write(dir.path().join("shadow.vid"), "shadow data").unwrap();
    std::fs::create_dir(dir.path().join("shadow")).unwrap();

    // Submodule of shadow
    std::fs::write(dir.path().join("shadow").join("another.vid"), "").unwrap();

    let mut codebase = Codebase::new();
    codebase.add_package("test".into(), dir.path()).unwrap();

    let pkgs = codebase.packages().collect::<Vec<_>>();
    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0].0, "test");

    let pkg_id = pkgs[0].1;
    let subs = codebase.get_submodules_for(pkg_id).collect::<Vec<_>>();
    assert_eq!(subs.len(), 3);

    assert!(subs.iter().find(|s| s.0 == "main").is_some());
    assert!(subs.iter().find(|s| s.0 == "empty").is_some());
    assert!(subs.iter().find(|s| s.0 == "shadow").is_some());
    assert!(subs.iter().find(|s| s.0 == "not-a-vid-file").is_none());

    let shadow_id = subs.iter().find(|s| s.0 == "shadow").unwrap().1;

    assert_eq!(codebase.get(shadow_id).data, Some("shadow data".into()));

    let shadow_subs = codebase.get_submodules_for(shadow_id).collect::<Vec<_>>();
    assert_eq!(shadow_subs.len(), 1);
    assert_eq!(shadow_subs[0].0, "another");
}
