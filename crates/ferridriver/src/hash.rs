pub type HashMap<K, V> = std::collections::HashMap<K, V, xxhash_rust::xxh3::Xxh3DefaultBuilder>;
pub type HashSet<T> = std::collections::HashSet<T, xxhash_rust::xxh3::Xxh3DefaultBuilder>;
