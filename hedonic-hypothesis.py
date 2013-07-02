import os
from random import random, choice

import libtcodpy as tcod

from entity_component_manager import EntityComponentManager
from components import *

def int_from_color(c):
    return c.r * 256 * 256 + c.g * 256 + c.b

def color_from_int(n):
    b = n % 256
    n = n / 256
    g = n % 256
    n = n / 256
    r = n
    return tcod.Color(r,g,b)


def equal_pos(p1, p2):
    if not (p1 and p2):
        return False
    return p1.x == p2.x and p1.y == p2.y and p1.floor == p2.floor

def neighbor_pos(p1, p2):
    if not (p1 and p2):
        return False
    return abs(p1.x - p2.x) <= 1 and abs(p1.y - p2.y) <= 1

def has_free_aps(e):
    turn = e.get(Turn)
    return turn and turn.action_points > 0

def initialise_consoles(console_count, w, h, transparent_color):
    """
    Initialise the given number of new off-screen consoles and return their list.
    """
    consoles = [tcod.console_new(w, h) for _ in xrange(console_count)]
    for con in consoles:
        tcod.console_set_key_color(con, transparent_color)
    return consoles

def tile_system(e, pos, tile, layers):
    if not all((e.has(c) for c in (Tile, Position))):
        return
    con = layers[tile.level]
    tcod.console_set_char_background(con, pos.x, pos.y, tcod.black)
    tcod.console_put_char(con, pos.x, pos.y, tile.glyph, tcod.BKGND_NONE)
    tcod.console_set_char_foreground(con, pos.x, pos.y, color_from_int(tile.color))

def input_system(e, ecm, key):
    if not key:
        return
    pos = e.get(Position)
    if not pos:
        return
    dest = MoveDestination(pos.x, pos.y, pos.floor)
    dx, dy = 0, 0
    if key.vk == tcod.KEY_UP:
        dy = -1
    elif key.vk == tcod.KEY_DOWN:
        dy = 1
    elif key.vk == tcod.KEY_LEFT:
        dx = -1
        if key.shift:
            dy = -1
        elif key.lctrl or key.rctrl or key.lalt or key.ralt:
            dy = 1
    elif key.vk == tcod.KEY_RIGHT:
        dx = 1
        if key.shift:
            dy = -1
        elif key.lctrl or key.rctrl or key.lalt or key.ralt:
            dy = 1
    if dx != 0 or dy != 0:
        e.set(dest._replace(x=pos.x+dx, y=pos.y+dy))

def ai_system(e, ai, pos, ecm, player):
    if not all((e.has(c) for c in (AI, Position))):
        return
    # TODO: use an action point system. It should make things simpler: if we
    # moved, we don't have any attack actions. If we didn't move, we can attack.
    # Will help us deal with the interactions, too.
    neighbor_vectors = ((-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1),
                        (0, 1), (1, 1))
    destinations = [Position(pos.x + dx, pos.y + dy, pos.floor) for dx, dy
                    in neighbor_vectors]
    player_pos = player.get(Position)
    if player_pos in destinations:
        dest = player_pos
    else:
        e.set(ai._replace(kind='idle'))
        destinations = [dest for dest in destinations
                        if not blocked_tile(dest, ecm)]
        if destinations:
            dest = choice(destinations)
        else:
            dest = None
    if dest:
        e.set(MoveDestination(dest.x, dest.y, dest.floor))
        if equal_pos(player_pos, dest) or neighbor_pos(player_pos, dest):
            if ai.kind == 'idle':
                e.set(ai._replace(kind='aggressive'))
            elif ai.kind == 'aggressive':
                e.set(Attacking(player))

def entities_on_position(pos, ecm):
    """
    Return all other entities with the same position.
    """
    return (entity for entity
            in ecm.entities_by_component_value(Position,
                                               x=pos.x, y=pos.y, floor=pos.floor))


def blocked_tile(pos, ecm):
    """
    True if the tile is non-empty or there's a bloking entity on it.
    """
    return any((entity.has(Solid) for entity
                in entities_on_position(pos, ecm)))

def entity_spend_ap(e, spent=1):
    turns = e.get(Turn)
    e.set(turns._replace(action_points = turns.action_points - spent))

def interaction_system(e, target, ecm):
    if not all((e.has(c) for c in (Position, MoveDestination, Turn))):
        return
    interactions = [entity for entity in entities_on_position(target, ecm)
                    if entity.has(Interactive)]
    monsters = [entity for entity in entities_on_position(target, ecm)
                if entity.has(Monster)]
    for m in monsters:
        if has_free_aps(e) and not e.has(Monster):
            e.set(Attacking(m))
    for i in interactions:
        if has_free_aps(e) > 0 and i.has(Interactive) and e.has(Addicted):
            attrs = e.get(Attributes)
            if attrs:  # base this off of the actual interaction type present
                som = attrs.state_of_mind + max(50 - attrs.tolerance, 5)
                e.set(attrs._replace(state_of_mind=som,
                                     tolerance=attrs.tolerance + 1))
            ecm.remove_entity(i)
    if monsters or interactions:
        return True

def combat_system(e, ecm):
    if not all((e.has(c) for c in (Attacking, Turn))):
        return
    target = e.get(Attacking).target
    e.remove(Attacking)
    if not has_free_aps(e) or not neighbor_pos(e.get(Position),
                                               target.get(Position)):
        return
    print "%s attacks %s" % (e, target)
    entity_spend_ap(e)
    if e.has(Info):
        death_reason = 'Killed by %s' % e.get(Info).name
    else:
        death_reason = ''
    kill_entity(target, death_reason)
    stats = e.get(Statistics)
    if stats:
        e.set(stats._replace(kills=stats.kills+1))

def movement_system(e, pos, dest, ecm, w, h):
    if not all((e.has(c) for c in (Position, MoveDestination, Turn))):
        return
    e.remove(MoveDestination)
    if has_free_aps(e) and not blocked_tile(dest, ecm):
        e.set(Position(dest.x, dest.y, dest.floor))
        entity_spend_ap(e)
        return True

def gui_system(ecm, player, layers, w, h, panel_height):
    attrs = player.get(Attributes)
    panel = tcod.console_new(w, panel_height)
    stats_template = "%s  Confidence: %s  Will: %s  Nerve: %s"
    tcod.console_print_ex(panel, 0, 0, tcod.BKGND_NONE, tcod.LEFT,
        stats_template % (player.get(Info).name, attrs.confidence, attrs.will,
                          attrs.nerve))
    if player.has(Dead):
        tcod.console_print_ex(panel, 0, 1, tcod.BKGND_NONE, tcod.LEFT,
                                 "DEAD: %s" % player.get(Dead).reason)
    else:
        max_bar_length = 20
        max_sate_of_mind = 100
        bar_length = attrs.state_of_mind * (max_bar_length - 1) / max_sate_of_mind
        full_bar = ' ' * (max_bar_length)
        bar = ' ' * (bar_length + 1)
        tcod.console_set_default_background(panel, tcod.dark_gray)
        tcod.console_print_ex(panel, 0, 1, tcod.BKGND_SET, tcod.LEFT, full_bar)
        if attrs.state_of_mind <  25:
            bar_color = tcod.dark_red
        elif attrs.state_of_mind < 60:
            bar_color = tcod.orange
        elif attrs.state_of_mind < 80:
            bar_color = tcod.chartreuse
        else:
            bar_color = tcod.turquoise
        tcod.console_set_default_background(panel, bar_color)
        tcod.console_print_ex(panel, 0, 1, tcod.BKGND_SET, tcod.LEFT, bar)
    doses = len([e for e in ecm.entities(Interactive)])
    monsters = len([e for e in ecm.entities(Monster)])
    tcod.console_print_ex(panel, w-1, 1, tcod.BKGND_NONE, tcod.RIGHT,
                          "Doses: %s,  Monsters: %s" % (doses, monsters))
    tcod.console_blit(panel, 0, 0, 0, 0, layers[9], 0, h - panel_height)

def kill_entity(e, death_reason=''):
    for ctype in (UserInput, AI, Position, Tile, Turn):
        e.remove(ctype)
    e.set(Dead(death_reason))

def entity_start_a_new_turn(e):
    t = e.get(Turn)
    e.set(t._replace(active=True, action_points=t.max_aps))

def end_of_turn_system(e, ecm):
    # TODO: trigger the end of turn effects here
    return False
    turn = e.get(Turn)
    e.set(turn._replace(action_points=turn.max_aps))
    attrs = e.get(Attributes)
    state_of_mind = attrs.state_of_mind - 1
    e.set(attrs._replace(state_of_mind=state_of_mind))
    if state_of_mind <= 0:
        kill_entity(e, "Exhausted")
    elif state_of_mind > 100:
        kill_entity(e, "Overdosed")

def process_entities(player, ecm, w, h, key):
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
            entity_start_a_new_turn(player)
            for npc in npcs:
                npc.set(npc.get(Turn)._replace(active=False))

    for e in ecm.entities(UserInput):
        if has_free_aps(e) and key:
            input_system(e, ecm, key)
    for e, ai, pos in ecm.entities(AI, Position, include_components=True):
        if has_free_aps(e):
            ai_system(e, ai, pos, ecm, player)
    for e, pos, dest in ecm.entities(Position, MoveDestination,
                                       include_components=True):
        interaction_system(e, dest, ecm)
        movement_system(e, pos, dest, ecm, w, h)
    for e in ecm.entities(Attacking):
        combat_system(e, ecm)
    for e in ecm.entities(Attributes):
        end_of_turn_system(e, ecm)

def update(game, dt_ms, consoles, w, h, panel_height, pressed_key):
    ecm = game['ecm']
    player = game['player']
    if pressed_key:
        if pressed_key.vk == tcod.KEY_ESCAPE:
            return None  # Quit the game
        elif pressed_key.vk == tcod.KEY_F5:
            return initial_state(w, h, game['empty_ratio'])

    process_entities(player, ecm, w, h, pressed_key)

    for e, pos, tile in ecm.entities(Position, Tile, include_components=True):
        tile_system(e, pos, tile, consoles)
    # empirically, the fading works best between 0.6 (darkest) and 1 (brightest)
    game['fade'] = (player.get(Attributes).state_of_mind * 0.4 / 100) + 0.6
    if player.has(Dead):
        game['fade'] = 1.6
    gui_system(ecm, player, consoles, w, h, panel_height)
    return game

def generate_map(w, h, empty_ratio):
    floor = []
    for x in xrange(w):
        for y in xrange(h):
            rand = random()
            if rand < empty_ratio:
                tile_kind = 'empty'
            elif rand < 0.99:
                tile_kind = 'wall'
            else:
                tile_kind = 'dose'
            if tile_kind == 'empty' and random() < 0.05:
                tile_kind = 'monster'
            floor.append([x, y, tile_kind])
    return [floor]

def initial_state(w, h, empty_ratio=0.6):
    ecm = EntityComponentManager(autoregister_components=True)
    ecm.register_component_type(Position, (int, int, int), index=True)
    # TODO: register the component types here once things settled a bit
    player_x, player_y = w / 2, h / 2
    player = ecm.new_entity()
    player.add(Position(player_x, player_y, 0))
    player.add(Tile(9, int_from_color(tcod.white), '@'))
    player.add(UserInput())
    player.add(Info(name="The Nameless One", description=""))
    player.add(Attributes(state_of_mind=20, tolerance=0, confidence=5,
                          nerve=5, will=5))
    player.add(Turn(action_points=3, max_aps=3, active=True))
    player.add(Statistics(turns=0, kills=0, doses=0))
    player.add(Solid())
    player.add(Addicted())
    player_pos = player.get(Position)
    for floor, map in enumerate(generate_map(w, h, empty_ratio)):
        for x, y, type in map:
            if equal_pos(player_pos, Position(x, y, floor)):
                pass
            elif type == 'wall':
                block = ecm.new_entity()
                block.add(Position(x, y, floor))
                color = choice((tcod.dark_green, tcod.green, tcod.light_green))
                block.add(Tile(0, int_from_color(color), '#'))
                block.add(Solid())
            elif type == 'dose':
                dose = ecm.new_entity()
                dose.add(Position(x, y, floor))
                dose.add(Tile(5, int_from_color(tcod.light_azure), 'i'))
                dose.add(Interactive())
            elif type == 'monster':
                monster = ecm.new_entity()
                monster.add(Position(x, y, floor))
                monster.add(Tile(8, int_from_color(tcod.dark_red), 'a'))
                monster.add(Solid())
                monster.add(Monster('a', strength=10))
                monster.add(Info('Anxiety', "Won't give you a second of rest."))
                monster.add(AI('idle'))
                monster.add(Turn(action_points=0, max_aps=2, active=False))
    return {
        'ecm': ecm,
        'player': player,
        'empty_ratio': empty_ratio,
    }


if __name__ == '__main__':
    SCREEN_WIDTH = 80
    SCREEN_HEIGHT = 50
    PANEL_HEIGHT = 2
    LIMIT_FPS = 60
    TRANSPARENT_BG_COLOR = tcod.red
    font_path = os.path.join('fonts', 'dejavu16x16_gs_tc.png')
    font_settings = tcod.FONT_TYPE_GREYSCALE | tcod.FONT_LAYOUT_TCOD
    game_title = 'Dose Response'
    tcod.console_set_custom_font(font_path, font_settings)
    tcod.console_init_root(SCREEN_WIDTH, SCREEN_HEIGHT, game_title, False)
    tcod.sys_set_fps(LIMIT_FPS)
    consoles = initialise_consoles(10, SCREEN_WIDTH, SCREEN_HEIGHT, TRANSPARENT_BG_COLOR)
    background_conlole = tcod.console_new(SCREEN_WIDTH, SCREEN_HEIGHT)
    for x in xrange(SCREEN_WIDTH):
        for y in xrange(SCREEN_HEIGHT):
            tcod.console_put_char(background_conlole, x, y, '.', tcod.BKGND_NONE)
    game_state = initial_state(SCREEN_WIDTH, SCREEN_HEIGHT - PANEL_HEIGHT)
    while not tcod.console_is_window_closed():
        tcod.console_set_default_foreground(0, tcod.white)
        key = tcod.console_check_for_keypress(tcod.KEY_PRESSED)
        if key.vk == tcod.KEY_NONE:
            key = None
        dt_ms = 10
        tcod.console_clear(None)
        for con in consoles:
            tcod.console_set_default_background(con, TRANSPARENT_BG_COLOR)
            tcod.console_set_default_foreground(con, tcod.white)
            tcod.console_clear(con)
        game_state = update(game_state, dt_ms, consoles,
                            SCREEN_WIDTH, SCREEN_HEIGHT, PANEL_HEIGHT, key)
        if not game_state:
            break
        fade = game_state.get('fade', 1)
        tcod.console_blit(background_conlole, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, 0, 0, 0, fade)
        for con in consoles[:-5]:
            tcod.console_blit(con, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, 0, 0, 0, fade)
        for con in consoles[-5:]:
            tcod.console_blit(con, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, 0, 0, 0, 1)
        tcod.console_flush()
