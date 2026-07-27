use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Category {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Category {
    pub fn new(id: u64, name: String, parent_id: Option<u64>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            parent_id,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_name(&mut self, name: String) {
        self.name = name;
        self.updated_at = Utc::now();
    }

    pub fn update_parent(&mut self, parent_id: Option<u64>) {
        self.parent_id = parent_id;
        self.updated_at = Utc::now();
    }
}
