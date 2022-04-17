## Code/Repo structure

- rework command error handling
    - maybe even do a rework s.t. cmds return a CommandOutput with stuff like { output, is_err, errs, ... }
        - which would make things cleaner, noo ?? ... idk

- maybe make an Erdos number implementation by watching who referenced by whom in channels?

- think about the way that commands get documented
    - ie. if a command takes into account other channels / can access data from other channels, ... 
    - (it is currently a goulash)

- investigate ways of making the bot comply with the concept of **continuous integration**


## Command ideas

- show chat statistics; could be like:
    - alltime messages
    - user with most messages since start of stream
    - ...
- last seen
