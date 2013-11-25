struct AI{behaviour: ai::Behaviour, state: ai::State}
struct AcceptsUserInput
struct Addiction{tolerance: int, drop_per_turn: int, last_turn: int}
struct AnxietyKillCounter{count: int, threshold: int}
struct AttackTarget(ID)
enum   AttackType {Kill, Stun{duration: int}, Panic{duration: int}, ModifyAttributes}
struct AttributeModifier{state_of_mind: int, will: int}
struct Attributes{state_of_mind: int, will: int}
struct Exploration{radius: int}
struct Explored
struct FadeColor{from: Color, to: Color, duration_s: float, repetitions: Repetitions}
struct FadeOut{to: Color, duration_s: float}
struct FadingOut
struct ColorAnimation{color: Color, progress: float, forward: bool}
struct Background
struct Bump(ID)
struct DeathTile{glyph: char, color: Color}
struct Destination {x: int, y: int}
struct Dose{tolerance_modifier: int, resist_radius: int}
struct ExplosionEffect{radius: int}
struct Monster{kind: MonsterKind}
struct Panicking{turn: int, duration: int}
struct Position {x: int, y: int}  //callbacks
struct Solid
struct Stunned{turn: int, duration: int}
struct Tile{level: uint, glyph: char, color: Color}
struct Turn{side: Side, ap: int, max_ap: int, spent_this_tick: int}
---
use std::num;
use std::iter;
use std::vec::MoveIterator;
use std::hashmap::HashMap;
use std::container::Map;

use engine::{Color};

#[deriving(Eq)]
pub enum Side {
    Player,
    Computer,
}

#[deriving(Eq)]
pub enum MonsterKind {
    Anxiety,
    Depression,
    Hunger,
    Voices,
    Shadows,
}

#[deriving(Eq)]
pub enum Repetitions {
    Infinite,
    Count(int),
}

pub mod ai {
    #[deriving(Eq)]
    pub enum Behaviour {
        Individual,
        Pack,
    }

    #[deriving(Eq)]
    pub enum State {
        Idle,
        Aggressive,
    }

}

{%for def in definitions %}
#[deriving(Eq)]
pub {{ def }}
{% endfor %}

struct Entity {
    {% for component in components %}
    {{ component|ident }}: Option<{{ component }}>,
    {% endfor %}
}

impl Entity {
    fn new() -> Entity {
        Entity{
            {% for component in components %}
            {{ component|ident }}: None,
            {% endfor %}
        }
    }
}


pub enum ComponentType {
    {% for component in components %}
    t{{ component }},
    {% endfor %}
}

#[deriving(Clone, Eq, ToStr)]
pub struct ID(int);

pub struct ComponentManager {
    priv entities: ~[Entity],
    priv initial_id: ID,
    priv next_id: ID,
    priv position_cache: HashMap<(int, int), ~[ID]>,
    priv empty_vector: ~[ID],  // Used to return positions on cache miss
}

impl ComponentManager {
pub fn new() -> ComponentManager {
        ComponentManager{
            entities: ~[],
            initial_id: ID(0),
            next_id: ID(0),
            position_cache: HashMap::new(),
            empty_vector: ~[],
        }
    }

    pub fn new_entity(&mut self) -> ID {
        self.add_entity(Entity::new())
    }

    pub fn add_entity(&mut self, entity: Entity) -> ID {
        self.entities.push(entity);
        self.next_id = ID(*self.next_id + 1);
        let e = ID(*self.next_id - 1);
        if self.has_position(e) {  // update the cache
            let pos = self.get_position(e);
            // Just remove the component. There is no entry in the cache yet.
            self.remove_position_(e);
            // Add the Position back to the entity and cache it
            self.set_position(e, pos);
        }
        e
    }

    fn index(&self, id: ID) -> uint {
        (*id - *self.initial_id) as uint
    }

    pub fn has_entity(&self, id: ID) -> bool {
        let index = self.index(id);
        let out_of_bounds = index < 0 || index >= self.entities.len();
        return !out_of_bounds;
    }

    pub fn take_out(&mut self, id: ID) -> Entity {
        let index = self.index(id);
        if self.has_entity(id) {
            self.entities.remove(index)
        } else {
            fail!(format!("Invalid entity ID {}", index))
        }
    }

    pub fn iter(&self) -> iter::Map<int, ID, iter::Range<int>> {
        range(*self.initial_id, *self.next_id).map(|index| ID(index))
    }

    pub fn remove_all_entities(&mut self) {
        self.entities.truncate(0);
        self.position_cache.clear();
        self.initial_id = self.next_id;
    }

    pub fn has_component(&self, id: ID, ctype: ComponentType) -> bool {
        match ctype {
            {% for component in components %}
            t{{ component }} => self.has_{{ component|ident }}(id),
            {% endfor %}
        }
    }

    // Autogenerated `has_component` methods:
    {% for component in components %}
    pub fn has_{{ component|ident }}(&self, id: ID) -> bool {
        let index = self.index(id);
        if self.has_entity(id) {
            self.entities[index].{{ component|ident }}.is_some()
        } else {
            fail!(format!("has_component: Invalid entity ID {}.", id.to_str()));
        }
    }
    {% endfor %}

    // Autogenerated `get_component` methods:
    {% for component in components %}
    pub fn get_{{ component|ident }}(&self, id: ID) -> {{ component }} {
        let index = self.index(id);
        if self.has_entity(id) {
            self.entities[index].{{ component|ident }}.unwrap()
        } else {
            fail!(format!("get_component: Invalid entity ID {}.", id.to_str()));
        }
    }
    {% endfor %}


    // Autogenerated `set component` methods:
    {% for component in components %}
    fn set_{{ component|ident }}_(&mut self, id: ID, component: {{ component }}) {
        let index = self.index(id);
        if self.has_entity(id) {
            self.entities[index].{{ component|ident }} = Some(component);
        } else {
            fail!(format!("set_component: Invalid entity ID {}.", id.to_str()));
        }
    }
    {% endfor %}

    // Autogenerated `remove component` methods:
    {% for component in components %}
    fn remove_{{ component|ident }}_(&mut self, id: ID) {
        let index = self.index(id);
        if self.has_entity(id) {
            self.entities[index].{{ component|ident }} = None;
        } else {
            fail!(format!("remove_component: Invalid entity ID {}.", id.to_str()));
        }
    }
    {% endfor %}

    {% for component in default_components %}
    pub fn set_{{ component|ident }}(&mut self, id: ID, component: {{ component }}) {
        self.set_{{ component|ident }}_(id, component)
    }
    {% endfor %}

    {% for component in default_components %}
    pub fn remove_{{ component|ident }}(&mut self, id: ID) {
        self.remove_{{ component|ident }}_(id)
    }
    {% endfor %}

    pub fn set_position(&mut self, id: ID, pos: Position) {
        if self.has_position(id) {
            // clear it from the cache
            self.remove_position(id);
        }
        self.set_position_(id, pos);
        let cache = self.position_cache.find_or_insert((pos.x, pos.y), ~[]);
        cache.push(id);
    }

    pub fn remove_position(&mut self, id: ID) {
        if self.has_position(id) {
            let pos = self.get_position(id);
            let cache = self.position_cache.get_mut(&(pos.x, pos.y));
            let id_index = match cache.iter().position(|&i| i == id) {
                Some(index) => index,
                None => fail2!("Position cache is missing the entity {}", *id),
            };
            cache.remove(id_index);
        }
        self.remove_position_(id)
    }

    pub fn entities_on_pos(&self, pos: Position) -> MoveIterator<ID> {
        match self.position_cache.find(&(pos.x, pos.y)) {
            Some(entities) => entities.clone().move_iter(),
            None => self.empty_vector.clone().move_iter(),
        }
    }
}


impl Turn {
    pub fn spend_ap(&self, spend: int) -> Turn {
        assert!(spend <= self.ap);
        Turn{ap: self.ap - spend,
             spent_this_tick: self.spent_this_tick + spend,
             .. *self}
    }
}

impl Stunned {
    pub fn remaining(&self, current_turn: int) -> int {
        num::max((self.turn + self.duration) - current_turn, 0)
    }
}

impl Panicking {
    pub fn remaining(&self, current_turn: int) -> int {
        num::max((self.turn + self.duration) - current_turn, 0)
    }
}
