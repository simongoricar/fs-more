use std::collections::HashMap;


pub struct NameCollisionAvoider {
    names_with_usages: HashMap<String, usize>,
}

impl NameCollisionAvoider {
    pub fn new_empty() -> Self {
        Self {
            names_with_usages: HashMap::new(),
        }
    }

    pub fn get_collision_free_name(&mut self, preferred_struct_name: &str) -> String {
        if let Some(usages_for_name) = self.names_with_usages.get_mut(preferred_struct_name) {
            *usages_for_name += 1;

            format!("{}{}", preferred_struct_name, *usages_for_name - 2)
        } else {
            self.names_with_usages
                .insert(preferred_struct_name.to_string(), 1);

            preferred_struct_name.to_string()
        }
    }
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn collsion_avoider_works() {
        let mut avoider = NameCollisionAvoider::new_empty();

        assert_eq!(avoider.get_collision_free_name("HelloWorld"), "HelloWorld");
        assert_eq!(avoider.get_collision_free_name("HelloWorld"), "HelloWorld1");
        assert_eq!(avoider.get_collision_free_name("HelloWorld"), "HelloWorld2");
        assert_eq!(avoider.get_collision_free_name("FooBar"), "FooBar");
    }
}
