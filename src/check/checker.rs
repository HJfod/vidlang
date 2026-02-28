use std::collections::HashMap;

use crate::{check::ty::Item, pools::{PoolRef, codebase::{Codebase, ModId}, items::{ItemId, Items}, names::Names}};

pub struct Checker {
    package_roots: HashMap<String, ItemId>,
    names: PoolRef<Names>,
    items: PoolRef<Items>,
}

impl Checker {
    pub fn new(codebase: &Codebase, names: PoolRef<Names>, items: PoolRef<Items>) -> Self {
        Self {
            package_roots: codebase.packages()
                .map(|p| (p.0.to_owned(), Self::make_submodule(codebase, names.clone(), items.clone(), p.0, p.1)))
                .collect(),
            names,
            items,
        }
    }
    fn make_submodule(codebase: &Codebase, names: PoolRef<Names>, items: PoolRef<Items>, name: &str, id: ModId) -> ItemId {
        let mut result = HashMap::new();
        for (name, sub) in codebase.get_submodules_for(id) {
            result.insert(
                names.lock_mut().add(name),
                Self::make_submodule(codebase, names.clone(), items.clone(), name, sub)
            );
        }
        items.lock_mut().add(Item::Module {
            name: names.lock_mut().add(name),
            definition: id,
            items: result,
        })
    }

    fn initial_discover(&mut self, item: ItemId) {
        // Recursively check all subitems too
        let items = self.items.lock().get(item).get_subitems();
        for id in items {
            self.initial_discover(id);
        }
    }
    pub fn run_initial_discovery(&mut self) {
        for root in self.package_roots.values().copied().collect::<Vec<_>>() {
            self.initial_discover(root);
        }
    }
}

#[test]
fn type_checker() {
    use crate::pools::codebase::Codebase;
    use crate::pools::messages::Messages;
    use crate::pools::exprs::Exprs;
    use crate::pools::names::Names;
    use crate::ast::expr::ParseArgs;

    let (mut codebase, _) = Codebase::new_with_test_package("type_checker", r#"
        const THING = 2;
        2 + 2;
    "#);

    let names = Names::new();
    let messages = Messages::new();
    let exprs = Exprs::new();
    codebase.parse_all(names.clone(), messages.clone(), exprs.clone(), ParseArgs {
        allow_non_definitions_at_root: true,
    });

    assert_eq!(
        messages.lock().count_total(), 0,
        "messages was not empty after parsing:\n{}", messages.lock().to_test_string(&codebase)
    );

    let items = Items::new();
    let mut checker = Checker::new(&codebase, names, items);
    checker.run_initial_discovery();

    assert_eq!(
        messages.lock().count_total(), 0,
        "messages was not empty after checking:\n{}", messages.lock().to_test_string(&codebase)
    );
}
