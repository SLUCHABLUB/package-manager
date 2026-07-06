pub(crate) mod once_cell_as_option {
    use once_cell::unsync::OnceCell;
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;

    pub fn serialize<T, S>(cell: &OnceCell<T>, serialiser: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        cell.get().serialize(serialiser)
    }

    pub fn deserialize<'data, T, D>(deserialiser: D) -> Result<OnceCell<T>, D::Error>
    where
        T: Deserialize<'data>,
        D: Deserializer<'data>,
    {
        Ok(Option::<T>::deserialize(deserialiser)?
            .map(OnceCell::from)
            .unwrap_or_default())
    }
}
