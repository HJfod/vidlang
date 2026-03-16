pub mod config;

use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::{
    ast::expr::{Ast, ParseArgs},
    codebase::config::VidToml,
    pools::{exprs::Exprs, items::{ItemId, Items}, messages::Messages, modules::{AddModuleError, ModId, Modules, Span, SrcIterator}, names::Names},
    tokens::{token::Token, tokenstream::Tokens}
};

/// Packages are the root unit of codebases. Each project and library is 
/// one package with a root module, which may contain any number of submodules
pub struct Package {
    pub path: PathBuf,
    pub config: VidToml,
    pub root_id: ModId,
}

pub struct Codebase {
    pub packages: HashMap<String, Package>,
    pub modules: Modules,
    pub names: Names,
    pub messages: Messages,
    pub exprs: Exprs,
    pub items: Items,
    pub parsed_asts: HashMap<ModId, Ast>,
    pub root_items: HashMap<ModId, ItemId>,
}

#[derive(Debug)]
pub enum AddPackageError {
    NoVidToml,
    CantReadVidToml(std::io::Error),
    BadVidToml(toml::de::Error),
    DuplicateName(String),
    ModuleError(AddModuleError),
}

impl Codebase {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            modules: Modules::new(),
            names: Names::new(),
            messages: Messages::new(),
            exprs: Exprs::new(),
            items: Items::new(),
            parsed_asts: HashMap::new(),
            root_items: HashMap::new(),
        }
    }
    #[cfg(test)]
    pub fn new_with_test_package(name: &str, data: &str) -> (Self, ModId) {
        let mut ret = Self::new();
        ret.add_package(&std::env::current_dir().unwrap().join("std")).unwrap();
        let id = ret.modules.add_test_module(name.to_string(), data);
        ret.packages.insert(name.to_string(), Package {
            path: PathBuf::from(name),
            config: VidToml::new_test(name),
            root_id: id,
        });
        (ret, id)
    }

    pub fn add_package(&mut self, pkg_dir: &Path) -> Result<String, AddPackageError> {
        // Load up vid.toml from the package root directory
        let config_path = pkg_dir.join("vid.toml");
        if !config_path.exists() {
            return Err(AddPackageError::NoVidToml);
        }
        let config_data = std::fs::read_to_string(config_path)
            .map_err(AddPackageError::CantReadVidToml)?;
        let config = toml::from_str::<VidToml>(&config_data)
            .map_err(AddPackageError::BadVidToml)?;

        let package_name = config.name().to_string();

        // Check if a package with this name has already been added
        if self.packages.contains_key(&package_name) {
            return Err(AddPackageError::DuplicateName(package_name));
        }

        // Find all source files for this package
        let root_id = self.modules.add_dir_recursive(package_name.clone(), pkg_dir)
            .map_err(AddPackageError::ModuleError)?;
        
        // Add to list of packages
        self.packages.insert(package_name.clone(), Package {
            path: pkg_dir.to_path_buf(),
            config,
            root_id,
        });
        Ok(package_name)
    }

    pub fn tokenize_mod(&mut self, mod_id: ModId) -> Option<Tokens> {
        // Can't use `get()` here because overlapping borrows
        let mut iter = SrcIterator::new(mod_id, self.modules.get(mod_id).data.as_ref()?.chars());
        let mut tokens = Vec::new();
        while let Some(tk) = Token::parse(&mut iter, &mut self.names, &mut self.messages) {
            tokens.push(tk);
        }
        Some(Tokens::new(tokens, "eof", Span::zero(mod_id)))
    }
    pub fn parse_one(&mut self, id: ModId, args: ParseArgs) -> Option<&Ast> {
        // Having to re-get this is silly but I ran into borrow checker issues
        if self.parsed_asts.contains_key(&id) {
            return self.parsed_asts.get(&id);
        }
        let mut tokens = self.tokenize_mod(id)?;
        let parsed = Ast::parse(&mut tokens, self, args);
        self.parsed_asts.insert(id, parsed);
        self.parsed_asts.get(&id)
    }
    pub fn parse_all(&mut self, args: ParseArgs) {
        for id in self.modules.all_ids() {
            self.parse_one(id, args);
        }
    }
}

#[test]
fn create_codebase() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();

    // Directory must have a vid.toml file
    std::fs::write(dir.path().join("vid.toml"), r#"
        [project]
        name = "create_codebase_test"
    "#).unwrap();
    
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
    codebase.add_package(dir.path()).unwrap();

    let pkgs = &codebase.packages;
    assert_eq!(pkgs.len(), 1);
    assert!(pkgs.contains_key("create_codebase_test"));

    let pkg_id = pkgs.get("create_codebase_test").unwrap().root_id;
    let subs = codebase.modules.get_submodules_for(pkg_id).collect::<Vec<_>>();
    assert_eq!(subs.len(), 3);

    assert!(subs.iter().find(|s| s.0 == "main").is_some());
    assert!(subs.iter().find(|s| s.0 == "empty").is_some());
    assert!(subs.iter().find(|s| s.0 == "shadow").is_some());
    assert!(subs.iter().find(|s| s.0 == "not-a-vid-file").is_none());

    let shadow_id = subs.iter().find(|s| s.0 == "shadow").unwrap().1;

    assert_eq!(codebase.modules.get(shadow_id).data, Some("shadow data".into()));

    let shadow_subs = codebase.modules.get_submodules_for(shadow_id).collect::<Vec<_>>();
    assert_eq!(shadow_subs.len(), 1);
    assert_eq!(shadow_subs[0].0, "another");

    assert_eq!(codebase.modules.get_full_mod_name(pkg_id), "create_codebase_test");
    assert_eq!(codebase.modules.get_full_mod_name(shadow_id), "create_codebase_test::shadow");
    assert_eq!(codebase.modules.get_full_mod_name(shadow_subs[0].1), "create_codebase_test::shadow::another");
}
