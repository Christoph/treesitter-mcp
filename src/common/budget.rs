pub struct BudgetTracker {
    max_tokens: usize,
    current: usize,
}

impl BudgetTracker {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            current: 0,
        }
    }

    pub fn can_add(&self, estimated_tokens: usize) -> bool {
        self.current + estimated_tokens <= self.max_tokens
    }

    pub fn add(&mut self, estimated_tokens: usize) -> bool {
        if self.can_add(estimated_tokens) {
            self.current += estimated_tokens;
            true
        } else {
            false
        }
    }

    #[cfg(test)]
    pub fn remaining(&self) -> usize {
        self.max_tokens.saturating_sub(self.current)
    }
}

pub fn estimate_symbol_tokens(fields_total_chars: usize) -> usize {
    // Conservative: ~4 chars per token + small overhead.
    (fields_total_chars / 4) + 3
}
