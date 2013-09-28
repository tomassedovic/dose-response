use std::iter::Enumerate;
use std::vec::{VecIterator, VecMutIterator};

struct EntityManager<E> {
    priv entities: ~[E],
    priv next_id: int,
}

impl<E> EntityManager<E> {
    pub fn new() -> EntityManager<E> {
        EntityManager{entities: ~[], next_id: 0}
    }

    pub fn add(&mut self, entity: E) -> int {
        self.entities.push(entity);
        self.next_id += 1;
        self.next_id - 1
    }

    pub fn get_ref<'r>(&'r self, id: int) -> &'r E {
        &self.entities[id]
    }

    pub fn get_ref_mut<'r>(&'r mut self, id: int) -> &'r mut E {
        &mut self.entities[id]
    }

    pub fn iter<'r>(&'r self) -> Enumerate<VecIterator<'r, E>> {
        return self.entities.iter().enumerate()
    }

    pub fn mut_iter<'r>(&'r mut self) -> Enumerate<VecMutIterator<'r, E>> {
        return self.entities.mut_iter().enumerate()
    }
}
