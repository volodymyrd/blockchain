use std::collections::HashMap;
use std::sync::Arc;

/// Node’s single storage source.
///
/// The Store holds one of the possible databases:
/// - The hot database - access to the hot database only
/// - The cold database - access to the cold database only
/// - The split database - access to both hot and cold databases
#[derive(Clone)]
pub struct Store {
    storage: Arc<HashMap<String, String>>,
}
