// LedgerEntry.rs

pub struct LedgerEntry { 
    pub date: String, 
    pub amount: f32, 
    pub deposit: bool,
    pub description: String,
}

pub struct Ledger {
    entries: Vec<LedgerEntry>,
    sum : f32
}

impl Ledger { 
    // public methods
    pub fn new() -> Self {
        Ledger {
            entries: Vec::new(),
            sum: 0.0
        }
    }
    pub fn add(&mut self, entry: LedgerEntry) {
        self.entries.push(entry);
        self.update_sum();
    }
    pub fn sum(&self) -> f32 {
        self.sum
    }
    // private methods
    fn update_sum(&mut self) {
        let mut sum : f32 = 0.0;
        for entry in &self.entries {
            if entry.deposit { 
                sum += entry.amount;
            } else {
                sum -= entry.amount;
            }
        }
        self.sum = sum;
    }
}