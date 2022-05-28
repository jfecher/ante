use crate::{
    cache::ModuleCache,
    error::{location::Location, ErrorMessage},
};

use super::Type;

pub fn from_template<'c>(
    template: &'static str, location: Location<'c>, t1: &Type, t2: &Type, cache: &ModuleCache<'c>,
) -> ErrorMessage<'c> {
    let mut msg = String::new();
    let t1 = t1.display(cache);
    let t2 = t2.display(cache);

    let mut iter = template.chars();
    while let Some(c) = iter.next() {
        match c {
            '$' => match iter.next() {
                Some('1') => msg += &t1.to_string(),
                Some('2') => msg += &t2.to_string(),
                _ => unreachable!(),
            },
            other => msg.push(other),
        }
    }

    make_error!(location, "{}", msg)
}
