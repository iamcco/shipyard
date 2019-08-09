mod view;

use crate::component_storage::AllStorages;
use std::convert::AsMut;
pub(crate) use view::EntityViewMut;

/* A Key is a handle to an entity and has two parts, the index and the version.
 * The length of the version can change but the index will always be size_of::<usize>() * 8 - version_len.
 * Valid versions can't exceed version::MAX() - 1, version::MAX() being used as flag for dead entities.
*/
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Key(usize);

impl Key {
    // Number of bits used by the version
    #[cfg(target_pointer_width = "64")]
    const VERSION_LEN: usize = 16;
    #[cfg(not(target_pointer_width = "64"))]
    const VERSION_LEN: usize = 12;

    const INDEX_MASK: usize = !0 >> Self::VERSION_LEN;
    const VERSION_MASK: usize = !Self::INDEX_MASK;

    /// Returns the index part of the Key.
    pub(crate) fn index(self) -> usize {
        self.0 & Self::INDEX_MASK
    }
    /// Returns the version part of the Key.
    fn version(self) -> usize {
        (self.0 & Self::VERSION_MASK) >> (0usize.count_zeros() as usize - Self::VERSION_LEN)
    }
    /// Make a new Key with the given index.
    fn new(index: usize) -> Self {
        assert!(index <= Self::INDEX_MASK);
        Key(index)
    }
    /// Modify the index.
    fn set_index(&mut self, index: usize) {
        assert!(index <= Self::INDEX_MASK);
        self.0 = (self.0 & Self::VERSION_MASK) | index
    }
    /// Increments the version, returns Err if version + 1 == version::MAX().
    fn bump_version(&mut self) -> Result<(), ()> {
        if self.0 < !(!0 >> (Self::VERSION_LEN - 1)) {
            self.0 = self.index()
                | ((self.version() + 1) << (std::mem::size_of::<usize>() * 8 - Self::VERSION_LEN));
            Ok(())
        } else {
            Err(())
        }
    }
}

/// Entities holds the Keys to all entities: living, removed and dead.
/// 
/// A living entity is an entity currently present, with or without component.
/// 
/// Removed and dead entities don't have any component.
/// 
/// The big difference is that removed ones can become alive again.
/// 
/// The life cycle of an entity looks like this:
/// 
/// Generation -> Deletion -> Dead\
///           ⬑----------↵
// An entity starts with a generation at 0, each removal will increase it by 1
// until version::MAX() where the entity is considered dead.
// Removed entities form a linked list inside the vector, using their index part to point to the next.
// Removed entities are added to one end and removed from the other.
// Dead entities are simply never added to the linked list.
pub struct Entities {
    data: Vec<Key>,
    list: Option<(usize, usize)>,
}

impl Default for Entities {
    fn default() -> Self {
        Entities {
            data: Vec::new(),
            list: None,
        }
    }
}

impl Entities {
    /// Returns a valid Key, reuse removed Key when possible
    pub(crate) fn generate(&mut self) -> Key {
        self.view_mut().generate()
    }
    pub(crate) fn view_mut(&mut self) -> EntityViewMut {
        EntityViewMut {
            data: &mut self.data,
            list: &mut self.list,
        }
    }
    /// Delete an entity and all its components.
    /// Returns true if the entity was alive.
    /// 
    /// [World::delete] is easier to use but will borrow and release [Entities] and [AllStorages] for each entity.
    /// # Example
    /// ```
    /// # use shipyard::*;
    /// let world = World::new::<(usize, u32)>();
    /// let entity1 = world.new_entity((0usize, 1u32));
    /// let entity2 = world.new_entity((2usize, 3u32));
    /// let mut entities = world.entities_mut();
    /// let mut all_storages = world.all_storages_mut();
    /// 
    /// entities.delete(&mut all_storages, entity1);
    /// 
    /// let (usizes, u32s) = all_storages.get_storage::<(&usize, &u32)>();
    /// assert_eq!((&usizes).get(entity1), None);
    /// assert_eq!((&u32s).get(entity1), None);
    /// assert_eq!(usizes.get(entity2), Some(&2));
    /// assert_eq!(u32s.get(entity2), Some(&3));
    /// ```
    /// 
    /// [World::delete]: struct.World.html#method.delete
    /// [Entities]: struct.Entities.html
    /// [AllStorages]: struct.AllStorages.html
    pub fn delete(
        &mut self,
        mut storages: impl AsMut<AllStorages>,
        entity: Key,
    ) -> bool {
        self.view_mut()
            .delete(&mut storages.as_mut().view_mut(), entity)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn key() {
        let mut key = Key::new(0);
        assert_eq!(key.index(), 0);
        assert_eq!(key.version(), 0);
        key.set_index(701);
        assert_eq!(key.index(), 701);
        assert_eq!(key.version(), 0);
        key.bump_version().unwrap();
        key.bump_version().unwrap();
        key.bump_version().unwrap();
        assert_eq!(key.index(), 701);
        assert_eq!(key.version(), 3);
        key.set_index(554);
        assert_eq!(key.index(), 554);
        assert_eq!(key.version(), 3);
    }
    #[test]
    fn entities() {
        let mut entities = Entities::default();

        let key00 = entities.generate();
        let key10 = entities.generate();

        assert_eq!(key00.index(), 0);
        assert_eq!(key00.version(), 0);
        assert_eq!(key10.index(), 1);
        assert_eq!(key10.version(), 0);

        assert!(entities.view_mut().delete_key(key00));
        assert!(!entities.view_mut().delete_key(key00));
        let key01 = entities.generate();

        assert_eq!(key01.index(), 0);
        assert_eq!(key01.version(), 1);

        assert!(entities.view_mut().delete_key(key10));
        assert!(entities.view_mut().delete_key(key01));
        let key11 = entities.generate();
        let key02 = entities.generate();

        assert_eq!(key11.index(), 1);
        assert_eq!(key11.version(), 1);
        assert_eq!(key02.index(), 0);
        assert_eq!(key02.version(), 2);

        let last_key = Key(!(!0 >> 15));
        entities.data[0] = last_key;
        assert!(entities.view_mut().delete_key(last_key));
        assert_eq!(entities.list, None);
        let dead = entities.generate();
        assert_eq!(dead.index(), 2);
        assert_eq!(dead.version(), 0);
    }
}
