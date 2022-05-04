## Code/Repo structure

- rework command error handling
    - maybe even do a rework s.t. cmds return a CommandOutput with stuff like { output, is_err, errs, ... }
        - which would make things cleaner, noo ?? ... idk

- think about the way that commands get documented
    - ie. if a command takes into account other channels / can access data from other channels, ... 
    - (it is currently a goulash)

## Command ideas

- maybe make an Erdos number implementation by watching who referenced by whom in channels?
