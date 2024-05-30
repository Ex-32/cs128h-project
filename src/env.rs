use std::{collections::HashMap, env, ffi::{OsStr, OsString}, sync::OnceLock};

use parking_lot::RwLock;

fn envs() -> &'static RwLock<HashMap<OsString, OsString>> {
    static ENVS: OnceLock<RwLock<HashMap<OsString, OsString>>> = OnceLock::new();
    ENVS.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn get<K: AsRef<OsStr>>(name: K) -> OsString {
    match envs().read().get(name.as_ref()) {
        Some(x) => x.clone(),
        None => match env::var_os(name) {
            Some(x) => x,
            None => OsString::new(),
        },
    }
}

pub fn set(name: OsString, val: OsString) {
    envs().write().insert(name, val);
}

// TODO: avoid deep copy of envs
pub fn pairs() -> Vec<(OsString, OsString)> {
    envs()
        .read()
        .iter()
        .map(|(x, y)| (x.clone(), y.clone()))
        .collect::<Vec<_>>()
}
