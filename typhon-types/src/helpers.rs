pub mod serialize_jobs {
    use crate::responses::{JobInfo, JobSystemName};
    use serde::*;
    use std::collections::HashMap;

    /// In JSON, we store JSON as nested objects
    type Repr = HashMap<String, HashMap<String, JobInfo>>;
    type Type = HashMap<JobSystemName, JobInfo>;

    pub fn serialize<S: Serializer>(jobs: &Type, s: S) -> Result<S::Ok, S::Error> {
        let mut intermediate = Repr::new();
        for (system_name, info) in jobs.clone() {
            intermediate
                .entry(system_name.system)
                .or_insert(HashMap::new())
                .insert(system_name.name, info);
        }
        Serialize::serialize(&intermediate, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Type, D::Error> {
        Ok(Repr::deserialize(d)?
            .into_iter()
            .map(|(system, jobs)| {
                jobs.into_iter().map(move |(name, info)| {
                    let system = system.clone();
                    (JobSystemName { system, name }, info)
                })
            })
            .flatten()
            .collect())
    }
}
