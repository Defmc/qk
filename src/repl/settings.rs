use smallvec::{SmallVec, ToSmallVec};

#[derive(Default, Clone)]
pub struct Setting {
    pub all: &'static [&'static str],
    pub on: SmallVec<[&'static str; 8]>,
}

impl Setting {
    pub fn parse_inspired<'a>(&self, value: &'a str) -> std::result::Result<Self, &'a str> {
        if value == "all" {
            return Ok(Setting {
                all: self.all,
                on: self.all.to_smallvec(),
            });
        }
        let mut s = Self::default();
        for v in value.split(',') {
            let trimmed = v.trim();
            if let Some(set) = self.all.iter().find(|&&a_v| a_v == trimmed) {
                s.on.push(set);
            } else {
                return Err(trimmed);
            }
        }
        Ok(s)
    }
}
