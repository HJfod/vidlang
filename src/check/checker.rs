use std::collections::HashMap;

use crate::{
    ast::expr::Visibility,
    check::ty::{Item, Ty},
    codebase::Codebase,
    pools::{items::{ItemId, Items}, messages::Message, modules::{ModId, Modules}, names::Names}
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
pub struct Checker<'cb> {
    pub(super) codebase: &'cb mut Codebase,
    pub(super) check_phase: CheckPhase,
    pub(super) current_item: Option<ItemId>,
}

impl<'cb> Checker<'cb> {
    fn init_modules(items: &mut Items, names: &mut Names, modules: &Modules, name: &str, id: ModId) -> ItemId {
        let mut result = HashMap::new();
        for (name, sub) in modules.get_submodules_for(id) {
            result.insert(
                names.add(name),
                Self::init_modules(items, names, modules, name, sub)
            );
        }
        items.add(Item::Module {
            visibility: Visibility::Public,
            name: names.add(name),
            definition: id,
            items: result,
            anon_items: Vec::new(),
        })
    }
    fn init_items(codebase: &mut Codebase) {
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

    pub fn new(codebase: &'cb mut Codebase) -> Self {
        Self::init_items(codebase);
        Self {
            codebase,
            check_phase: CheckPhase::Discovery,
            current_item: None,
        }
    }

    pub fn add_item(&mut self, item: Item) -> ItemId {
        let name = item.name();
        let span = item.span(self.codebase);
        let id = self.codebase.items.add(item);
        match self.codebase.items.get_mut(self.current_item.unwrap()) {
            Item::Constant { .. } => unreachable!("attempted to add a sub item \
                to a constant, something is very very very very very wrong"),
            Item::BlockScope { items, anon_items, .. } | 
            Item::Function { items, anon_items, .. } | 
            Item::Module { items, anon_items, .. } => {
                match name {
                    Some(name) => {
                        if let Some(exist) = items.get(&name) {
                            let id = *exist;
                            let existing_span = self.codebase.items.get(id).span(self.codebase);
                            self.codebase.messages.add(Message::new_error(
                                "item with name {} defined multiple times",
                                span
                            ).with_note("first definition here", Some(existing_span)));
                        }
                        else {
                            items.insert(name, id);
                        }
                    }
                    None => anon_items.push(id),
                }
            }
        }
        id
    }

    pub fn check(&mut self, id: ModId) -> Ty {
        self.current_item = Some(*self.codebase.root_items.get(&id).unwrap());
        self.check_expr(*self.codebase.parsed_asts.get(&id).unwrap())
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

    for pkg_root_id in codebase.packages.iter().map(|p| p.1.root_id).collect::<Vec<_>>() {
        let mut checker = Checker::new(&mut codebase);
        checker.check(pkg_root_id);
    }

    assert_eq!(
        codebase.messages.count_total(), 0,
        "messages was not empty after checking:\n{}", codebase.messages.to_test_string(&codebase)
    );
}
