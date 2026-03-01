use std::collections::HashMap;

use crate::{check::ty::Item, pools::{codebase::{Codebase, ModId}, items::{ItemId, Items}, names::Names}};

pub struct Checker {
    package_roots: HashMap<String, ItemId>,
}

// impl Checker {
//     pub fn new(codebase: &mut Codebase) -> Self {
//         let pkgs = codebase.packages()
//                 // Because we can't have multiple borrows into Codebase...
//                 .map(|p| (p.0.to_owned(), p.1))
//                 .collect::<Vec<_>>().into_iter();
//         Self {
//             package_roots: pkgs
//                 .map(|p| (p.0.clone(), Self::make_submodule(codebase, &p.0, p.1)))
//                 .collect(),
//         }
//     }
//     fn make_submodule(codebase: &mut Codebase, name: &str, id: ModId) -> ItemId {
//         let mut result = HashMap::new();
//         for (name, sub) in codebase.get_submodules_for(id) {
//             result.insert(
//                 codebase.names.add(name),
//                 Self::make_submodule(codebase, name, sub)
//             );
//         }
//         codebase.items.add(Item::Module {
//             name: codebase.names.add(name),
//             definition: id,
//             items: result,
//         })
//     }

//     fn initial_discover(&mut self, item: ItemId) {
//         // Recursively check all subitems too
//         let items = self.items.get(item).get_subitems();
//         for id in items {
//             self.initial_discover(id);
//         }
//     }
//     pub fn run_initial_discovery(&mut self) {
//         for root in self.package_roots.values().copied().collect::<Vec<_>>() {
//             self.initial_discover(root);
//         }
//     }
// }

// #[test]
// fn type_checker() {
//     use crate::pools::codebase::Codebase;
//     use crate::pools::messages::Messages;
//     use crate::pools::exprs::Exprs;
//     use crate::pools::names::Names;
//     use crate::ast::expr::ParseArgs;

//     let (mut codebase, _) = Codebase::new_with_test_package("type_checker", r#"
//         const THING = 2;
//         2 + 2;
//     "#);

//     let names = Names::new();
//     let messages = Messages::new();
//     let exprs = Exprs::new();
//     codebase.parse_all(names, messages, exprs.clone(), ParseArgs {
//         allow_non_definitions_at_root: true,
//     });

//     assert_eq!(
//         messages.count_total(), 0,
//         "messages was not empty after parsing:\n{}", messages.to_test_string(&codebase)
//     );

//     let items = Items::new();
//     let mut checker = Checker::new(&codebase, names, items);
//     checker.run_initial_discovery();

//     assert_eq!(
//         messages.count_total(), 0,
//         "messages was not empty after checking:\n{}", messages.to_test_string(&codebase)
//     );
// }
