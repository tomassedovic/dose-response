from collections import namedtuple
import sys

def Component(name, attrs=''):
    current_module = sys.modules[__name__]
    setattr(current_module, name, namedtuple(name, attrs))

Component('Position', 'x y floor')

Component('MoveDestination', 'x y floor')

Component('Tile', 'level color glyph')

Component('UserInput')

Component('Solid')

Component('Attributes', 'state_of_mind, tolerance, confidence, nerve, will')

Component('Statistics', 'turns, kills, doses')

Component('Dead', 'reason')

Component('Interactive')

Component('Info', 'name, description')

Component('Monster', 'kind, strength')

Component('Attacking', 'target')

Component('AI', 'kind')

Component('Addicted', 'rate_per_turn, turn_last_activated')

Component('Turn', 'action_points, max_aps, active, count')

Component('Explorable', 'explored')

Component('AttributeModifier',
          'state_of_mind, tolerance, confidence, nerve, will')
