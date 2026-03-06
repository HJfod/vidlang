use std::{collections::HashMap, ffi::OsStr, fs::read_to_string, io::Error, path::{Path, PathBuf}, range::Range, str::Chars};

use slotmap::{SlotMap, new_key_type};

use crate::utils::lookahead_iter::Looakhead;

new_key_type! { pub struct ModId; }

pub enum SrcModuleParent {
    Module(ModId),
    Package(String),
}

pub struct SrcModule {
    pub path: PathBuf,
    pub data: Option<String>,
    pub parent: SrcModuleParent,
    pub submodules: HashMap<String, ModId>,
}

pub struct Modules {
    pool: SlotMap<ModId, SrcModule>,
}

#[derive(Debug)]
pub enum AddModuleError {
    UnableToReadDir(PathBuf, std::io::Error),
    UnableToReadFile(PathBuf, std::io::Error),
}

impl Modules {
    pub fn new() -> Self {
        Self { pool: Default::default() }
    }

    /// Adds a directory to the module pool, with all of its source files and 
    /// subdirectories marked as children of that directory
    pub fn add_dir_recursive(&mut self, pkg_name: String, dir_path: &Path) -> Result<ModId, AddModuleError> {
        let root_id = self.pool.insert(SrcModule {
            path: dir_path.to_path_buf(),
            data: None,
            parent: SrcModuleParent::Package(pkg_name),
            submodules: HashMap::new(),
        });
        self.add_files_in_dir(root_id, dir_path)?;
        Ok(root_id)
    }

    fn add_files_in_dir(&mut self, parent_id: ModId, dir_path: &Path) -> Result<(), AddModuleError> {
        let map_dir_err = |e: Error| AddModuleError::UnableToReadDir(dir_path.to_path_buf(), e);

        for entry_res in dir_path.read_dir().map_err(map_dir_err)? {
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
            let sub_mod_id = match self.get_submodule(parent_id, &sub_mod_name) {
                Some(existing_id) => {
                    // Prefer file modules as the paths; if the existing one isn't 
                    // a file path, update it
                    let existing = self.pool.get_mut(existing_id).unwrap();
                    if existing.path.extension() != Some(OsStr::new("vid")) {
                        existing.path = path.clone();
                    }
                    existing_id
                }
                // Otherwise add this to the map
                None => {
                    let new_id = self.pool.insert(SrcModule {
                        path: path.clone(),
                        data: None,
                        parent: SrcModuleParent::Module(parent_id),
                        submodules: HashMap::new(),
                    });
                    self.pool.get_mut(parent_id).unwrap().submodules.insert(sub_mod_name.clone(), new_id);
                    new_id
                }
            };

            // At this point we have made sure the module is in the pool (and 
            // in the map of submodules); now we can actually read the file or 
            // directory and get the data out of it
            if path.is_dir() {
                self.add_files_in_dir(sub_mod_id, &path)?;
            }
            else {
                let data = read_to_string(&path).map_err(|e| AddModuleError::UnableToReadFile(path, e))?;
                if self.get(sub_mod_id).data.is_some() {
                    panic!(
                        "Something is definitely wrong - module {} already had data",
                        sub_mod_name
                    );
                }
                self.pool.get_mut(sub_mod_id).unwrap().data = Some(data);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn add_test_module(&mut self, pkg_name: String, data: &str) -> ModId {
        self.pool.insert(SrcModule {
            path: PathBuf::from(&pkg_name),
            data: Some(data.to_string()),
            parent: SrcModuleParent::Package(pkg_name),
            submodules: HashMap::new(),
        })
    }

    pub fn get(&self, id: ModId) -> &SrcModule {
        self.pool.get(id).expect("Modules has handed out an invalid ModId")
    }

    pub fn all_ids(&self) -> Vec<ModId> {
        self.pool.keys().collect()
    }

    pub fn get_submodule(&self, id: ModId, sub_name: &str) -> Option<ModId> {
        self.pool.get(id)?.submodules.get(sub_name).copied()
    }
    pub fn get_submodules_for(&self, id: ModId) -> impl Iterator<Item = (&str, ModId)> {
        self.get(id).submodules.iter().map(|p| (p.0.as_str(), *p.1))
    }
    pub fn get_full_mod_name(&self, id: ModId) -> String {
        match &self.get(id).parent {
            SrcModuleParent::Module(pm) => {
                format!(
                    "{}::{}",
                    self.get_full_mod_name(*pm),
                    self.get(*pm).submodules.iter().find(|s| *s.1 == id).unwrap().0
                )
            }
            SrcModuleParent::Package(pkg) => {
                pkg.clone()
            }
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
    pub fn new(id: ModId, chars: Chars<'s>) -> Self {
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
    use crate::codebase::Codebase;
    let (codebase, id) = Codebase::new_with_test_package("test_src_iter", "abcdefg");
    let mut iter = SrcIterator::new(id, codebase.modules.get(id).data.as_ref().unwrap().chars());
    for ch in 'a'..='g' {
        assert_eq!(iter.peek(), Some(ch));
        assert_eq!(iter.next(), Some(ch));
    }
}
