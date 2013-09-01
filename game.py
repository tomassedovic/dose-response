import collections
from datetime import datetime
import math
import os
from random import random, choice, seed, randint

import lib.libtcodpy as tcod
from lib.enum import Enum

from components import *
from ecm_artemis import EntityComponentManager
import entity_templates as templates
import level_generators as gen
import location_utils as loc
from partial_helpers import *
from systems.graphics import (tile_system, background_system, gui_system,
                              precise_distance, Color)
from systems import path
from systems.ai import ai_system


CHEATING = False

def has_free_aps(e, required=1):
    turn = e.get(Turn)
    return turn and turn.action_points >= required

def modify_entity_attributes(e, modif):
    """
    Updates entity's attributes based on the passed modifier.
    """
    assert e.has(Attributes) and modif
    e.update(Attributes,
             state_of_mind=bounded_add(0, modif.state_of_mind),
             tolerance=bounded_add(0, modif.tolerance),
             confidence=bounded_add(0, modif.confidence),
             nerve=bounded_add(0, modif.nerve),
             will=bounded_add(0, modif.will))


def initialise_consoles(console_count, w, h, transparent_color):
    """
    Initialise the given number of new off-screen consoles and return their list.
    """
    consoles = [tcod.console_new(w, h) for _ in xrange(console_count)]
    for con in consoles:
        tcod.console_set_key_color(con, transparent_color)
    return consoles

Commands = Enum('Commands', 'N E W S NE NW SE SW')

def command_from_key(key):
    ctrl_or_alt = key.lctrl or key.rctrl or key.lalt or key.ralt
    if key.vk == tcod.KEY_UP:
        return Commands.N
    elif key.vk == tcod.KEY_DOWN:
        return Commands.S
    elif key.vk == tcod.KEY_LEFT and key.shift:
        return Commands.NW
    elif key.vk == tcod.KEY_LEFT and ctrl_or_alt:
        return Commands.SW
    elif key.vk == tcod.KEY_LEFT:
        return Commands.W
    elif key.vk == tcod.KEY_RIGHT and key.shift:
        return Commands.NE
    elif key.vk == tcod.KEY_RIGHT and ctrl_or_alt:
        return Commands.SE
    elif key.vk == tcod.KEY_RIGHT:
        return Commands.E
    else:
        print "Unexpected key pressed: %s" % key.c

def input_system(e, ecm, commands, save_for_replay):
    if not commands:
        return
    pos = e.get(Position)
    if not pos:
        return
    cmd = commands.popleft()
    assert cmd is not None
    save_for_replay(cmd.name)
    dx, dy = 0, 0
    if cmd in (Commands.N, Commands.NE, Commands.NW):
        dy = -1
    elif cmd in (Commands.S, Commands.SE, Commands.SW):
        dy = 1

    if cmd in (Commands.E, Commands.NE, Commands.SE):
        dx = 1
    elif cmd in (Commands.W, Commands.NW, Commands.SW):
        dx = -1

    if dx != 0 or dy != 0:
        e.set(MoveDestination(pos.x + dx, pos.y + dy))

def entity_spend_ap(e, spent=1):
    turns = e.get(Turn)
    e.set(turns._replace(action_points = turns.action_points - spent))

def interaction_system(e, ecm):
    if not all((e.has(c) for c in (Position, Turn))):
        return
    pos = e.get(Position)
    if not e.has(Addicted):
        return  # Only addicted characters can interact with items for now
    for i in loc.entities_on_position(pos, ecm):
        if not i.has(Interactive):
            continue
        attribute_modifier = i.get(AttributeModifier)
        if attribute_modifier:
            modify_entity_attributes(e, attribute_modifier)
        area_kill_effect = i.get(KillSurroundingMonsters)
        if area_kill_effect:
            for monster in loc.entities_nearby(pos, area_kill_effect.radius,
                                               ecm, lambda e: e.has(Monster)):
                kill_entity(monster)
        ecm.remove_entity(i)

def update_kill_counter(killer, target):
    if not killer.has(KillCounter):
        return
    if not target.has(Monster):
        return
    if target.get(Monster).kind == 'anxiety':
        killer.update(KillCounter, anxieties=inc)


def combat_system(e, ecm):
    if not all((e.has(c) for c in (Attacking, Turn, Info))):
        return
    target = e.get(Attacking).target
    assert e != target, "%s tried to attack itself" % e
    e.remove(Attacking)
    if not has_free_aps(e) or not loc.neighbor_pos(e.get(Position),
                                                   target.get(Position)):
        return
    print "%s attacks %s" % (e, target)

    entity_spend_ap(e)
    death_reason = "Killed by %s" % e.get(Info).name
    if e.has(Monster):
        hit_effect = e.get(Monster).hit_effect
        if hit_effect == 'modify_attributes':
            assert target.has(Attributes) and e.has(AttributeModifier)
            modify_entity_attributes(target, e.get(AttributeModifier))
            if target.get(Attributes).state_of_mind <= 0:
                kill_entity(target, death_reason)
        elif hit_effect == 'stun':
            duration = 3
            if target.has(StunEffect):
                target.update(StunEffect, duration=add(duration))
            else:
                target.set(StunEffect(duration))
            kill_entity(e, "Disappeared after the attack.")
        elif hit_effect == 'panic':
            duration = 3
            if target.has(PanicEffect):
                target.update(PanicEffect, duration=add(duration))
            else:
                target.set(PanicEffect(duration))
            kill_entity(e, "Disappeared after the attack.")
        else:
            raise AssertionError('Unknown hit_effect')
    else:
        kill_entity(target, death_reason)
    if target.has(Dead) and e.has(Statistics):
        e.update(Statistics, kills=inc)
    if target.has(Dead):
        update_kill_counter(e, target)


def panic_system(e, ecm, w, h):
    if not all(e.has(c) for c in (PanicEffect, Position, MoveDestination)):
        return
    panic = e.get(PanicEffect)
    if panic.duration <= 0:
        e.remove(PanicEffect)
    else:
        print "%s panics" % e
        pos = e.get(Position)
        destinations = loc.available_destinations(pos, ecm, w, h)
        if destinations:
            dest = choice(destinations)
        else:
            dest = pos
        e.set(MoveDestination._make(dest))
        e.update(PanicEffect, duration=dec)

def stun_system(e, ecm):
    if not all(e.has(c) for c in (StunEffect, Position, MoveDestination)):
        return
    stun = e.get(StunEffect)
    if stun.duration <= 0:
        e.remove(StunEffect)
    else:
        print "%s is stunned" % e
        e.set(MoveDestination._make(e.get(Position)))
        e.update(StunEffect, duration=dec)

def dose_event_horizon(dose, addict):
    """Return the radius within which the addict loses control and must get the
    dose.
    """
    assert dose.has(Dose)
    assert addict.has(Addicted)
    return dose.get(Dose).irresistibility - addict.get(Addicted).resistance

def irresistible_dose_system(e, ecm, fov_map):
    if not all((e.has(c) for c in (Position, Addicted))):
        return
    pos = e.get(Position)
    if e.has(MovePath):
        print "entity already has a path"
        return  # The entity's already following a path, don't interfere
    addict = e  # make us a closure for the function below
    def irresistible_dose(e):
        if not all(e.has(c) for c in (Position, Dose)):
            return
        dose_pos = e.get(Position)
        if addict.get(Position) == dose_pos:
            return
        return loc.distance(dose_pos, addict.get(Position)) <= dose_event_horizon(e, addict)
    search_radius = 3  # max irresistibility for a dose is currently 3
    doses = list(loc.entities_nearby(pos, search_radius, ecm, pred=irresistible_dose))
    if not doses:
        return
    target_dose = min(doses, key=lambda e: loc.distance(pos, e.get(Position)))
    dest = MoveDestination._make(target_dose.get(Position))
    path_id = path.find(fov_map, pos, dest)
    # TODO: Check that all the path steps are within the radius instead. The
    # path may be longer if there is an obstacle but we want that to be
    # irresistible, too so long as the player wouldn't have to the sphere of
    # radius.
    if path_id is not None and path.length(path_id) <= dose_event_horizon(target_dose, addict):
        print "Setting path with destination: %s" % (dest,)
        e.set(MovePath(path_id))

def dose_glow_system(e, ecm, player):
    if not all(e.has(c) for c in (Dose, Glow)):
        return
    e.update(Glow, radius=replace(dose_event_horizon(e, player)))

def movement_system(e, ecm, w, h):
    if not all((e.has(c) for c in (Position, MoveDestination, Turn))):
        return
    pos = e.get(Position)
    walk_path = lambda: None
    if e.has(MovePath):
        path_id = e.get(MovePath).id
        if path.length(path_id) == 0:
            path.destroy(path_id)
            e.remove(MovePath)
            dest = e.get(MoveDestination)
        else:
            x, y = tcod.path_get(path_id, 0)  # get the next path cell
            walk_path = lambda: tcod.path_walk(path_id, True)
            if (x, y) != (None, None):
                dest = MoveDestination(x, y)
            else:
                assert False, "path was blocked"
    else:
        dest = e.get(MoveDestination)
    e.remove(MoveDestination)
    if not has_free_aps(e):
        print "%s tried to move but has no action points" % e
        return
    if loc.equal_pos(pos, dest):
        # The entity waits a turn
        entity_spend_ap(e)
    elif loc.blocked_tile(dest, ecm):
        bumped_entities = [entity for entity in loc.entities_on_position(dest, ecm)
                           if entity.has(Solid)]
        assert len(bumped_entities) < 2, "There should be at most 1 solid entity on a given position"
        if bumped_entities:
            e.set(Bump(bumped_entities[0]))
    elif not loc.within_rect(dest, 0, 0, w, h):
        if e.has(LeaveLevel):
            e.update(LeaveLevel, leaving=replace(True))
    else:
        e.set(Position._make(dest))
        entity_spend_ap(e)
        walk_path()  # Only walk the path when we're actually able to move

def bump_system(e, ecm):
    if not all((e.has(c) for c in (Bump,))):
        return
    target = e.get(Bump).target
    e.remove(Bump)
    assert e != target, "%s tried to bump itself" % e
    valid_target = ((not e.has(Monster) and target.has(Monster)) or
                    (e.has(Monster) and not target.has(Monster)))
    if valid_target:
        e.set(Attacking(target))
    else:
        pass  # bumped into a wall or something else that's not interactive

def kill_entity(e, death_reason=''):
    for ctype in (UserInput, AI, Solid, Tile, Turn):
        e.remove(ctype)
    e.set(Dead(death_reason))

def entity_start_a_new_turn(e):
    t = e.get(Turn)
    e.set(t._replace(active=True, action_points=t.max_aps))

def end_of_turn_system(e, ecm):
    if not all((e.has(c) for c in (Turn,))):
        return
    turn = e.get(Turn)
    e.set(turn._replace(count=turn.count+1))

def addiction_system(e, ecm):
    if not all((e.has(c) for c in (Addicted, Attributes, Turn))):
        return
    addiction = e.get(Addicted)
    attrs = e.get(Attributes)
    turn = e.get(Turn)
    dt = turn.count - addiction.turn_last_activated
    if attrs.state_of_mind == 98:
        e.update(Abilities, see_entities=replace(True))
    elif attrs.state_of_mind in (98, 99):
        e.set(Abilities(see_entities=True, see_world=True))
    if dt > 0:
        state_of_mind = attrs.state_of_mind - (addiction.rate_per_turn * dt)
        e.set(attrs._replace(state_of_mind=state_of_mind))
        e.set(addiction._replace(turn_last_activated=turn.count))
        if state_of_mind <= 0:
            kill_entity(e, "Withdrawal shock")
        elif state_of_mind > 100:
            kill_entity(e, "Overdosed")

def will_system(e, ecm):
    if not all((e.has(c) for c in (Addicted, Attributes))):
        return
    kill_counter = e.get(KillCounter)
    if kill_counter:
        assert kill_counter.anxieties >= 0
        assert kill_counter.anxiety_threshold >= 0
        if kill_counter.anxieties >= kill_counter.anxiety_threshold:
            increment = kill_counter.anxieties // kill_counter.anxiety_threshold
            e.update(Attributes, will=add(increment))
            e.update(KillCounter,
                     anxieties=sub(increment * kill_counter.anxiety_threshold))
    attrs = e.get(Attributes)
    e.update(Addicted, resistance=replace(min(attrs.will, 2)))


def process_entities(player, ecm, w, h, fov_map, commands, save_for_replay):
    if player.has(Dead):
        return

    player_turn = player.get(Turn)
    if player_turn.active and not has_free_aps(player):
        player.set(player_turn._replace(active=False))
        for npc in ecm.entities(AI):
            entity_start_a_new_turn(npc)
    if not player_turn.active:
        npcs = list(ecm.entities(AI))
        if not any((has_free_aps(npc) for npc in npcs)):
            end_of_turn_system(player, ecm)
            for e in npcs:
                end_of_turn_system(e, ecm)
            entity_start_a_new_turn(player)
            for npc in npcs:
                npc.set(npc.get(Turn)._replace(active=False))
    assert any((e.get(Turn).active and e.get(Turn).action_points > 0
                for e in ecm.entities(Turn)))

    for e in ecm.entities(Addicted, Attributes, Turn):
        addiction_system(e, ecm)
        will_system(e, ecm)
    for e in ecm.entities(UserInput):
        if has_free_aps(e) and commands:
            input_system(e, ecm, commands, save_for_replay)
    for e, ai, pos in ecm.entities(AI, Position, include_components=True):
        if has_free_aps(e):
            ai_system(e, ai, pos, ecm, player, fov_map, w, h)
    for e in ecm.entities(Position, MoveDestination):
        panic_system(e, ecm, w, h)
        stun_system(e, ecm)
        movement_system(e, ecm, w, h)
        irresistible_dose_system(e, ecm, fov_map)
        bump_system(e, ecm)
        interaction_system(e, ecm)
    for e in ecm.entities(Attacking):
        combat_system(e, ecm)
    for e in ecm.entities(Dose):
        dose_glow_system(e, ecm, player)
    # TODO: Assert every entity with free turns spent at least one of them

def update(game, dt_ms, consoles, w, h, panel_height, pressed_key):
    ecm = game['ecm']
    player = game['player']
    if pressed_key:
        if pressed_key.vk == tcod.KEY_ESCAPE:
            return None  # Quit the game
        elif pressed_key.vk == tcod.KEY_F5:
            initial_game_state = new_game()
            seed(initial_game_state['seed'])
            ecm = EntityComponentManager(autoregister_components=True)
            ecm.register_component_type(Position, (int, int, int), index=True)
            game_state = build_level(w, h - panel_height, ecm, player=None,
                                     level_generator=gen.forrest_level)
            game_state.update(initial_game_state)
            return game_state
        elif pressed_key.vk == tcod.KEY_F6:
            global CHEATING
            CHEATING = not CHEATING
        elif pressed_key.c == ord('d'):
            import pdb; pdb.set_trace()
        else:
            game['keys'].append(pressed_key)

    while game['keys']:
        key = game['keys'].popleft()
        cmd = command_from_key(key)
        if cmd:
            game['commands'].append(cmd)

    process_entities(player, ecm, w, h,
                     game['fov_map'],
                     game['commands'],
                     game['save_for_replay'])
    if player.get(LeaveLevel).leaving:
        player.update(LeaveLevel, leaving=replace(False))
        new_ecm = EntityComponentManager(autoregister_components=True)
        new_ecm.register_component_type(Position, (int, int, int), index=True)

        new_game_state = build_level(w, h - panel_height, new_ecm, player, gen.forrest_level)
        new_game_state['seed'] = game['seed']
        new_game_state['save_for_replay'] = game['save_for_replay']
        new_game_state['commands'] = game['commands']
        return new_game_state

    player_pos = player.get(Position)
    if player_pos:
        assert player.has(Attributes)
        som = player.get(Attributes).state_of_mind
        game['fov_radius'] = (4 * som + 293) / 99  # range(3, 8)
        game['recompute_fov'](game['fov_map'], player_pos.x, player_pos.y, game['fov_radius'])
    background_system(ecm, w, h, player_pos, game, consoles, player, CHEATING)
    for e, pos, tile in ecm.entities(Position, Tile, include_components=True):
        tile_system(e, pos, tile, consoles, game['fov_map'], player,
                    game['fov_radius'], CHEATING)
    game['fade'] = max(player.get(Attributes).state_of_mind / 100.0, 0.14)
    if player.has(Dead):
        game['fade'] = 2
    gui_system(ecm, player, consoles, w, h, panel_height, CHEATING, dt_ms)
    return game

def new_game():
    seed(datetime.now())
    random_seed = randint(1, 999999)
    command_queue = collections.deque()
    replay_file_name = "replay-%s" % datetime.now().isoformat()
    replay_file = open(replay_file_name, 'w')
    replay_file.write(str(random_seed) + '\n')
    def save_for_replay(step):
        replay_file.write(step + '\n')
        replay_file.flush()
    return {
        'seed': random_seed,
        'save_for_replay': save_for_replay,
        'commands': command_queue,
    }

def load_replay(replay_file_name):
    with open(replay_file_name, 'r') as replay_file:
        lines = [l.strip() for l in replay_file.readlines()]
    if not lines:
        print "Empty replay file"
        exit(1)
    seed_str, cmd_names = lines[0], lines[1:]
    try:
        random_seed = int(seed_str)
    except ValueError:
        print "The first line of the replay file must be a number (seed)"
        exit(1)
    assert 1 <= random_seed <= 999999, "The replay seed must be within the limit"
    commands = (Commands[name] for name in cmd_names)
    command_queue = collections.deque(commands)
    return {
        'seed': random_seed,
        'save_for_replay': lambda x: None,  # This is a replay, no need for save
        'commands': command_queue,
    }

def build_level(w, h, ecm, player, level_generator):
    player_pos = Position(w / 2, h / 2)
    if player:  # We're moving an existing PC to a new world
        copied_player = ecm.new_entity()
        for component in player.components():
            copied_player.set(component)
        copied_player.set(player_pos)
        player = copied_player
    else:
        player = ecm.new_entity()
        templates.player(player, player_pos)
    initial_dose_pos = Position(
        player_pos.x + choice([n for n in range(-3, 3) if n != 0]),
        player_pos.y + choice([n for n in range(-3, 3) if n != 0]),
    )
    def near_player(x, y):
        return loc.distance(player_pos, (x, y)) < 6
    fov_map = tcod.map_new(w, h)
    for x, y, type in level_generator(w, h):
        transparent = True
        walkable = True
        pos = Position(x, y)
        background = ecm.new_entity()
        background.add(pos)
        background.add(Tile(0, Color.empty_tile, '.'))
        explored = precise_distance(pos, player_pos) < 6
        background.add(Explorable(explored=explored))
        if loc.equal_pos(player_pos, pos):
            pass
        elif (((type == 'dose' or type == 'stronger_dose')
               and not near_player(x, y))
               or loc.equal_pos(initial_dose_pos, pos)):
            if loc.equal_pos(initial_dose_pos, pos):
                type = 'dose'
            dose = ecm.new_entity()
            dose_type = 'weak' if type == 'dose' else 'strong'
            templates.dose(dose, pos)
            dose.update(Explorable, explored=replace(explored))
        elif type == 'wall':
            templates.wall(background, kind='wall')
            walkable = False
        elif type == 'monster' and not near_player(x, y):
            monster = ecm.new_entity()
            monster.add(pos)
            monster.add(Solid())
            factories = [
                templates.anxiety_monster,
                templates.depression_monster,
                templates.hunger_monster,
                templates.voices_monster,
                templates.shadows_monster,
            ]
            choice(factories)(monster)
        tcod.map_set_properties(fov_map, x, y, transparent, walkable)
    assert len(set(ecm.entities_by_component_value(Position, x=player_pos.x, y=player_pos.y))) > 1
    fov_radius = 3
    def recompute_fov(fov_map, x, y, radius):
        tcod.map_compute_fov(fov_map, x, y, radius, True)
    recompute_fov(fov_map, player_pos.x, player_pos.y, fov_radius)
    return {
        'ecm': ecm,
        'player': player,
        'keys': collections.deque(),
        'fov_map': fov_map,
        'fov_radius': fov_radius,
        'recompute_fov': recompute_fov,
    }


def run(replay_file_name=None):
    """Start the game.

    This is a blocking function that runs the main game loop.
    """
    if replay_file_name:
        initial_game_state = load_replay(replay_file_name)
    else:
        initial_game_state = new_game()
    print "Using random seed: %s" % initial_game_state['seed']
    seed(initial_game_state['seed'])

    SCREEN_WIDTH = 80
    SCREEN_HEIGHT = 50
    PANEL_HEIGHT = 2
    LIMIT_FPS = 60
    font_path = os.path.join('fonts', 'dejavu16x16_gs_tc.png')
    font_settings = tcod.FONT_TYPE_GREYSCALE | tcod.FONT_LAYOUT_TCOD
    game_title = 'Dose Response'
    tcod.console_set_custom_font(font_path, font_settings)
    tcod.console_init_root(SCREEN_WIDTH, SCREEN_HEIGHT, game_title, False)
    tcod.sys_set_fps(LIMIT_FPS)
    consoles = initialise_consoles(10, SCREEN_WIDTH, SCREEN_HEIGHT, Color.transparent.value)
    background_conlole = tcod.console_new(SCREEN_WIDTH, SCREEN_HEIGHT)

    ecm = EntityComponentManager(autoregister_components=True)
    ecm.register_component_type(Position, (int, int, int), index=True)
    game_state = build_level(SCREEN_WIDTH, SCREEN_HEIGHT - PANEL_HEIGHT,
                             ecm, player=None, level_generator=gen.forrest_level)
    game_state.update(initial_game_state)
    while not tcod.console_is_window_closed():
        tcod.console_set_default_foreground(0, Color.foreground.value)
        key = tcod.console_check_for_keypress(tcod.KEY_PRESSED)
        if key.vk == tcod.KEY_NONE:
            key = None
        dt_ms = math.trunc(tcod.sys_get_last_frame_length() * 1000)
        tcod.console_clear(None)
        for con in consoles:
            tcod.console_set_default_background(con, Color.transparent.value)
            tcod.console_set_default_foreground(con, Color.foreground.value)
            tcod.console_clear(con)
        game_state = update(game_state, dt_ms, consoles,
                            SCREEN_WIDTH, SCREEN_HEIGHT, PANEL_HEIGHT, key)
        if not game_state:
            break
        fade = game_state.get('fade', 1)
        for con in consoles[:-5]:
            tcod.console_blit(con, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, 0, 0, 0, fade)
        for con in consoles[-5:]:
            tcod.console_blit(con, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, 0, 0, 0, 1)
        tcod.console_flush()
