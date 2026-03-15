use std::collections::HashMap;

use crate::{
    check::ty::Item,
    codebase::Codebase, pools::{items::ItemId, modules::ModId}
};

pub struct Checker {
    package_roots: HashMap<String, ItemId>,
}

impl Checker {
    pub fn new(codebase: &mut Codebase) -> Self {
        let pkgs = codebase.packages.iter()
                // Because we can't have multiple borrows into Codebase...
                .map(|p| (p.0.to_owned(), p.1.root_id))
                .collect::<Vec<_>>().into_iter();
        Self {
            package_roots: pkgs
                .map(|p| (p.0.clone(), Self::make_submodule(codebase, &p.0, p.1)))
                .collect(),
        }
    }
    fn make_submodule(codebase: &mut Codebase, name: &str, id: ModId) -> ItemId {
        let mut result = HashMap::new();
        for (name, sub) in codebase.modules.get_submodules_for(id)
            // Because we can't have multiple borrows into Codebase...
            .map(|p| (p.0.to_owned(), p.1))
            .collect::<Vec<_>>().into_iter()
        {
            result.insert(
                codebase.names.add(&name),
                Self::make_submodule(codebase, &name, sub)
            );
        }
        codebase.items.add(Item::Module {
            name: codebase.names.add(name),
            definition: id,
            items: result,
        })
    }

    fn initial_discover(&mut self, item: ItemId, codebase: &mut Codebase) {
        // Recursively check all subitems too
        let items = codebase.items.get(item).get_subitems();
        for id in items {
            self.initial_discover(id, codebase);
        }
    }
    pub fn run_initial_discovery(&mut self, codebase: &mut Codebase) {
        for root in self.package_roots.values().copied().collect::<Vec<_>>() {
            self.initial_discover(root, codebase);
        }
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

    let mut checker = Checker::new(&mut codebase);
    checker.run_initial_discovery(&mut codebase);

    assert_eq!(
        codebase.messages.count_total(), 0,
        "messages was not empty after checking:\n{}", codebase.messages.to_test_string(&codebase)
    );
}
