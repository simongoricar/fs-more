use std::collections::HashMap;

use quote::format_ident;
use syn::Ident;


pub struct NameCollisionAvoider {
    names_with_usages: HashMap<String, usize>,
}

impl NameCollisionAvoider {
    pub fn new_empty() -> Self {
        Self {
            names_with_usages: HashMap::new(),
        }
    }

    pub fn collision_free_name(&mut self, preferred_name: &str) -> String {
        if let Some(usages_for_name) = self.names_with_usages.get_mut(preferred_name) {
            *usages_for_name += 1;

            format!("{}{}", preferred_name, *usages_for_name)
        } else {
            self.names_with_usages.insert(preferred_name.to_string(), 1);

            preferred_name.to_string()
        }
    }

    pub fn collision_free_ident(&mut self, preferred_ident: &Ident) -> Ident {
        let ident_string = preferred_ident.to_string();

        let collision_free_name = self.collision_free_name(&ident_string);

        format_ident!("{}", collision_free_name)
    }
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn collision_avoider_works() {
        let mut avoider = NameCollisionAvoider::new_empty();

        assert_eq!(avoider.collision_free_name("HelloWorld"), "HelloWorld");
        assert_eq!(avoider.collision_free_name("HelloWorld"), "HelloWorld2");
        assert_eq!(avoider.collision_free_name("HelloWorld"), "HelloWorld3");
        assert_eq!(avoider.collision_free_name("FooBar"), "FooBar");
    }
}
