pub const SLOTS: usize = 48;
pub const SLOT_MINUTES: u32 = 24 * 60 / SLOTS as u32;

pub fn slot_at(hour: u32, minute: u32) -> usize {
    (hour as usize % 24) * 2 + usize::from(minute >= 30)
}

pub fn slot_label(slot: usize) -> String {
    let total = slot % SLOTS;
    let hour24 = total / 2;
    let minute = if total % 2 == 0 { "00" } else { "30" };
    let suffix = if hour24 < 12 { "am" } else { "pm" };
    let hour12 = if hour24 % 12 == 0 { 12 } else { hour24 % 12 };
    format!("{hour12}:{minute}{suffix}")
}

pub struct Block {
    pub id: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
}

pub fn block(slots: &[Option<String>], slot: usize) -> Block {
    let id = slots.get(slot).cloned().flatten();
    let mut start = slot;
    while start > 0 && slots[start - 1] == id {
        start -= 1;
    }
    let mut end = slot;
    while end + 1 < slots.len() && slots[end + 1] == id {
        end += 1;
    }
    Block { id, starts_at: slot_label(start), ends_at: slot_label(end + 1) }
}

pub struct Next {
    pub at: String,
    pub id: Option<String>,
}

pub fn next_change(slots: &[Option<String>], slot: usize) -> Option<Next> {
    let here = slots.get(slot).cloned().flatten();
    for step in 1..=SLOTS {
        let s = (slot + step) % SLOTS;
        let id = slots.get(s).cloned().flatten();
        if id == here {
            continue;
        }
        return Some(Next { at: slot_label(s), id });
    }
    None
}

pub fn seconds_until_next_slot(minute: u32, second: u32) -> u32 {
    let into = minute % 30;
    (30 - into) * 60 - second.min(59)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_boundaries() {
        assert_eq!(slot_at(0, 29), 0);
        assert_eq!(slot_at(0, 30), 1);
        assert_eq!(slot_at(23, 59), 47);
        assert_eq!(slot_label(0), "12:00am");
        assert_eq!(slot_label(3), "1:30am");
        assert_eq!(slot_label(24), "12:00pm");
        assert_eq!(slot_label(48), "12:00am");
    }

    #[test]
    fn touching_slots_with_one_mix_are_one_block() {
        let mut slots = vec![None; SLOTS];
        slots[2] = Some("a".to_string());
        slots[3] = Some("a".to_string());
        let b = block(&slots, 2);
        assert_eq!(b.id.as_deref(), Some("a"));
        assert_eq!(b.starts_at, "1:00am");
        assert_eq!(b.ends_at, "2:00am");
        let n = next_change(&slots, 2).unwrap();
        assert_eq!(n.at, "2:00am");
        assert_eq!(n.id, None);
    }

    #[test]
    fn last_slot_ends_at_midnight() {
        let mut slots = vec![None; SLOTS];
        slots[47] = Some("b".to_string());
        assert_eq!(block(&slots, 47).ends_at, "12:00am");
    }
}
