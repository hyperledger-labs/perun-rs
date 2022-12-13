use serde::ser::{Serialize, SerializeSeq, Serializer};

pub fn serialize<S, T>(v: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let mut s = serializer.serialize_seq(Some(v.len()))?;
    for e in v {
        s.serialize_element(e)?;
    }
    s.end()
}
