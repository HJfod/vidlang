use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct NameId(usize);

pub static MISSING_NAME: NameId = NameId(0);

#[derive(Debug, Clone)]
pub struct Names {
    names: Arc<Mutex<Vec<String>>>,
}

impl Names {
    pub fn new() -> Self {
        Self {
            // First name is always MISSING_NAME
            names: Arc::from(Mutex::new(vec!["<missing name>".into()]))
        }
    }
    pub fn add(&self, name: &str) -> NameId {
        let mut names = self.names.lock().unwrap();
        if let Some((id, _)) = names.iter().enumerate().find(|n| n.1 == name) {
            return NameId(id);
        }
        names.push(name.to_string());
        NameId(names.len() - 1)
    }
    pub fn fetch(&self, id: NameId) -> String {
        self.names.lock().unwrap()
            .get(id.0)
            .expect("NamePool has apparently handed out an invalid NameId")
            .clone()
    }
}
