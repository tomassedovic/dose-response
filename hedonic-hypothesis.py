from collections import namedtuple
import os
from random import random

import libtcodpy as libtcod

from entity_component_manager import EntityComponentManager
from components import *

def initialise_consoles(console_count, w, h, transparent_color):
    """
    Initialise the given number of new off-screen consoles and return their list.
    """
    consoles = [libtcod.console_new(w, h) for _ in xrange(console_count)]
    for con in consoles:
        libtcod.console_set_key_color(con, transparent_color)
    return consoles

def tile_system(e, dt_ms, layers):
    pos = e.get(Position)
    if not pos:
        return
    tile = e.get(Tile)
    con = layers[tile.level]
    libtcod.console_set_char_background(con, pos.x, pos.y, libtcod.black)
    libtcod.console_put_char(con, pos.x, pos.y, tile.glyph, libtcod.BKGND_NONE)

def input_system(e, dt_ms, key):
    if not key:
        return
    pos = e.get(Position)
    if not pos:
        return
    dest = MoveDestination(pos.x, pos.y, pos.floor)
    if key.vk == libtcod.KEY_UP:
        dest.y -= 1
    elif key.vk == libtcod.KEY_DOWN:
        dest.y += 1
    elif key.vk == libtcod.KEY_LEFT:
        dest.x -= 1
    elif key.vk == libtcod.KEY_RIGHT:
        dest.x += 1
    e.set(dest)

def movement_system(e, dt_ms, w, h):
    def equal_pos(p1, p2):
        return p1.x == p2.x and p1.y == p2.y and p1.floor == p2.floor
    dest = e.get(MoveDestination)
    pos = e.get(Position)
    colliding = [entity for entity in e._ecm.entities(Position)
                 if equal_pos(entity.get(Position), dest) and e != entity]
    empty = len(colliding) == 0  # Assume that void (no tile) blocks player
    blocked = empty or any((entity.has(Solid) for entity in colliding))
    if not blocked:
        e.set(Position(dest.x, dest.y, dest.floor))
    e.remove(MoveDestination)

def gui_system(ecm, dt_ms, player, layers, w, h):
    attrs = player.get(Attributes)
    panel_height = 4
    panel = libtcod.console_new(w, panel_height)
    stats_template = "State of mind: %s  Confidence: %s  Will: %s  Nerve: %s"
    libtcod.console_print_ex(panel, 0, 1, libtcod.BKGND_NONE, libtcod.LEFT,
        stats_template % (attrs.state_of_mind, attrs.confidence, attrs.will, attrs.nerve))
    libtcod.console_blit(panel, 0, 0, 0, 0, layers[9], 0, h - panel_height)

def update(game, dt_ms, consoles, w, h, key):
    ecm = game['ecm']
    if key and key.vk == libtcod.KEY_ESCAPE:
        return None  # Quit the game
    for controllable in [e for e in ecm.entities(UserInput)]:
        input_system(controllable, dt_ms, key)
    for moving in [e for e in ecm.entities(MoveDestination)]:
        movement_system(e, dt_ms, w, h)
    for renderable in [e for e in ecm.entities(Tile)]:
        tile_system(renderable, dt_ms, consoles)
    player = [e for e in ecm.entities(Attributes)][0]
    gui_system(ecm, dt_ms, player, consoles, w, h)
    return game

def generate_map(w, h):
    floor = []
    for x in xrange(w):
        for y in xrange(h):
            tile_kind = 'empty'
            if random() > 0.7:
                tile_kind = 'wall'
            floor.append([x, y, tile_kind])
    return [floor]

def initial_state(w, h):
    ecm = EntityComponentManager()
    ecm.register_component_type(Position)
    ecm.register_component_type(MoveDestination)
    ecm.register_component_type(Tile)
    ecm.register_component_type(UserInput)
    ecm.register_component_type(Solid)
    ecm.register_component_type(Attributes)
    player = ecm.new_entity()
    player.set(Position(w / 2, h / 2, 1))
    player.set(Tile(9, None, '@'))
    player.set(UserInput())
    player.set(Attributes(state_of_mind=20, tolerance=0, confidence=5,
                          nerve=5, will=5))
    for floor, map in enumerate(generate_map(w, h)):
        for x, y, type in map:
            block = ecm.new_entity()
            block.set(Position(x, y, floor+1))
            if type == 'empty':
                block.set(Tile(0, None, ' '))
            else:
                block.set(Tile(0, None, '#'))
                block.set(Solid())
    return {'ecm': ecm}


if __name__ == '__main__':
    SCREEN_WIDTH = 80
    SCREEN_HEIGHT = 50
    LIMIT_FPS = 60
    TRANSPARENT_BG_COLOR = libtcod.red
    font_path = os.path.join('fonts', 'arial12x12.png')
    font_settings = libtcod.FONT_TYPE_GREYSCALE | libtcod.FONT_LAYOUT_TCOD
    game_title = 'Hedonic Hypothesis'
    libtcod.console_set_custom_font(font_path, font_settings)
    libtcod.console_init_root(SCREEN_WIDTH, SCREEN_HEIGHT, game_title, False)
    libtcod.sys_set_fps(LIMIT_FPS)
    consoles = initialise_consoles(10, SCREEN_WIDTH, SCREEN_HEIGHT, TRANSPARENT_BG_COLOR)
    game_state = initial_state(SCREEN_WIDTH, SCREEN_HEIGHT)
    while game_state and not libtcod.console_is_window_closed():
        libtcod.console_set_default_foreground(0, libtcod.white)
        key = libtcod.console_check_for_keypress(libtcod.KEY_PRESSED)
        if key.vk == libtcod.KEY_NONE:
            key = None
        dt_ms = 10
        libtcod.console_clear(None)
        for con in consoles:
            libtcod.console_set_default_background(con, TRANSPARENT_BG_COLOR)
            libtcod.console_set_default_foreground(con, libtcod.white)
            libtcod.console_clear(con)
        game_state = update(game_state, dt_ms, consoles, SCREEN_WIDTH, SCREEN_HEIGHT, key)
        for con in consoles:
            libtcod.console_blit(con, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, 0, 0, 0)
        libtcod.console_flush()
