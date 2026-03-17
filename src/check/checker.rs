use std::collections::HashMap;

use crate::{
    check::ty::Item,
    codebase::Codebase,
    pools::{items::{ItemId, Items}, modules::{ModId, Modules}, names::Names}
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NameHint {
    // This is probably the name of a variable
    Variable,
    // This is probably the name of a function
    Function,
    // This is probably the name of a module
    Module,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CheckPhase {
    /// Find item names
    Discovery,
    /// Check types
    Check {
        name_hint: Option<NameHint>,
    },
}

#[derive(Debug)]
pub struct Checker {
    check_phase: CheckPhase,
    current_item: ItemId,
}

impl Checker {
    fn init_modules(items: &mut Items, names: &mut Names, modules: &Modules, name: &str, id: ModId) -> ItemId {
        let mut result = HashMap::new();
        for (name, sub) in modules.get_submodules_for(id) {
            result.insert(
                names.add(&name),
                Self::init_modules(items, names, modules, &name, sub)
            );
        }
        items.add(Item::Module {
            name: names.add(name),
            definition: id,
            items: result,
        })
    }
    pub fn init_items(codebase: &mut Codebase) {
        for (name, pkg) in &codebase.packages {
            // Skip packages that have already been added (in case `init_items` 
            // is called multiple times)
            if codebase.root_items.contains_key(&pkg.root_id) {
                continue;
            }
            let root = Self::init_modules(
                &mut codebase.items,
                &mut codebase.names,
                &codebase.modules,
                name, pkg.root_id
            );
            codebase.root_items.insert(pkg.root_id, root);
        }
    }

    pub fn new(root_item: ItemId) -> Checker {
        Self {
            check_phase: CheckPhase::Discovery,
            current_item: root_item,
        }
    }

    pub fn discovering(&self) -> bool {
        matches!(self.check_phase, CheckPhase::Discovery)
    }
    pub fn phase(&self) -> &CheckPhase {
        &self.check_phase
    }
}

#[test]
fn type_checker() {
    use crate::ast::expr::ParseArgs;

    let (mut codebase, _) = Codebase::new_with_test_package("type_checker", r#"
        const THING = 2;
        2 + 2;
    "#);
    codebase.parse_all(ParseArgs {
        allow_non_definitions_at_root: true,
        ..Default::default()
    });

    assert_eq!(
        codebase.messages.count_total(), 0,
        "messages was not empty after parsing:\n{}", codebase.messages.to_test_string(&codebase)
    );

    Checker::init_items(&mut codebase);
    for pkg_root_id in &codebase.packages.iter().map(|p| p.1.root_id).collect::<Vec<_>>() {
        let root = codebase.root_items.get(pkg_root_id).unwrap();
        let mut checker = Checker::new(*root);
        checker.check(*codebase.parsed_asts.get(pkg_root_id).unwrap(), &mut codebase);
    }

    assert_eq!(
        codebase.messages.count_total(), 0,
        "messages was not empty after checking:\n{}", codebase.messages.to_test_string(&codebase)
    );
}
