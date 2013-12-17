Dose Response
=============


Requirements
------------

* Rust compiler, commit `3272b002b3d2943f17be70e719bf2ffa6058cf72`.
* Python 2.7
* Jinja2 (see `requirements.txt`)


Building from the source
------------------------

1. git clone http://example.com/dose-response.git
1. cd dose-response
1. virtualenv --distribute .venv
1. .venv/bin/pip install -r requirements.txt
1. make

Running the game
----------------

You can start Dose Response by running the `dose-response` shell script
generated by `make`. You can also use `make run`.

Each playthrough is automatically saved in the `replays` directory. To watch a
particular replay, pass the file as a command line argument:

    ./dose-response replays/replay-2013-10-21T01:26:29.409

To watch the latest replay, you can run `make replay`. This is useful for
development:

1. _write some code_
1. `make run`
1. _play the came, see it crash_
1. `make replay`
1. _verify the crash, try and fix the bug_
1. `make replay`
1. repeat
