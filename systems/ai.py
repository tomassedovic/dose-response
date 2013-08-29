from random import choice

from components import *
import location_utils as loc
from partial_helpers import *
from systems import path


def find_player_callback(player_pos, ecm):
    def cb(x_from, y_from, x_to, y_to, user_data):
        if (x_to, y_to) == (player_pos.x, player_pos.y):
            # The player must be reachable for the monster, otherwise
            # the path will be never found.
            return 1.0
        elif loc.blocked_tile(MoveDestination(x_to, y_to), ecm):
            return 0.0
        else:
            return 1.0
    return cb

def follow_player(e, player, ecm, fov_map):
    pos = e.get(Position)
    player_pos = player.get(Position)
    if loc.neighbor_pos(player_pos, pos):
        dest = player_pos
        e.remove(MovePath)
    else:
        if e.has(MovePath):
            # We need to generate a new path because the player has most
            # likely moved away
            path.destroy(e.get(MovePath).id)
            e.remove(MovePath)
        path_id = path.find(fov_map, pos, player_pos,
                            path_cb=find_player_callback(player_pos, ecm))
        if path_id is not None:
            e.set(MovePath(path_id))
        else:
            print 'could not find path'
        dest = None
    e.set(Attacking(player))
    return dest

def individual_behaviour(e, ai, pos, ecm, player, fov_map, w, h):
    player_pos = player.get(Position)
    player_distance = loc.distance(pos, player_pos)
    if player_distance < 5:
        e.update(AI, state=replace('aggressive'))
    if player_distance > 8:
        e.update(AI, state=replace('idle'))
    destinations = loc.available_destinations(pos, ecm, w, h)
    if not destinations:
        dest = None
    elif e.get(AI).state == 'aggressive':
        dest = follow_player(e, player, ecm, fov_map)
    elif e.get(AI).state == 'idle':
        if e.has(MovePath):
            path.destroy(e.get(MovePath).id)
        e.remove(MovePath)
        dest = choice(destinations)
    else:
        raise AssertionError('Unknown AI state: "%s"' % e.get(AI).state)
    return dest


def hunting_pack_behaviour(e, ai, pos, ecm, player, fov_map, w, h):
    player_pos = player.get(Position)
    player_distance = loc.distance(pos, player_pos)
    if player_distance < 4:
        e.update(AI, state=replace('aggressive'))
    destinations = loc.available_destinations(pos, ecm, w, h)
    if not destinations:
        return
    if e.get(AI).state == 'idle':
        return choice(destinations)
    elif e.get(AI).state == 'aggressive':
        dest = follow_player(e, player, ecm, fov_map)
        monster_kind = e.get(Monster).kind
        def kindred_monster(e):
            return (e.has(AI) and e.has(Monster) and
                    e.get(Monster).kind == monster_kind)
        for nearby_hunter in loc.entities_nearby(pos, 8, ecm,
                                                 pred=kindred_monster):
            nearby_hunter.update(AI, state=replace('aggressive'))
    else:
        raise AssertionError('Unknown AI state: "%s"' % e.get(AI).state)
    return dest


def ai_system(e, ai, pos, ecm, player, fov_map, w, h):
    if not all((e.has(c) for c in (AI, Position))):
        return
    behaviour_map = {
        'individual': individual_behaviour,
        'pack': hunting_pack_behaviour,
    }
    behaviour = behaviour_map[e.get(AI).kind]
    dest = behaviour(e, ai, pos, ecm, player, fov_map, w, h)
    if dest:
        e.set(MoveDestination._make(dest))
    else:
        e.set(MoveDestination._make(pos))
